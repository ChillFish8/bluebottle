use std::sync::LazyLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use iced::widget::shader::{self, Action, Viewport};
use iced::{Event, Rectangle, Size, Transformation, mouse, wgpu, window};

use crate::color;

/// The shared clock all skeletons read, so the sweep is coherent across them.
static ANCHOR: LazyLock<Instant> = LazyLock::new(Instant::now);

/// Seconds for the shimmer band to cross the window once. The shader sweeps over
/// this, and the clock is reduced modulo it so the phase keeps full `f32`
/// precision however long the process has been running.
const CYCLE: f32 = 2.5;

/// Slots the buffer starts with and never shrinks below.
const MIN_CAPACITY: usize = 16;

/// The program behind every skeleton; one type, so they share one pipeline.
pub struct SkeletonProgram {
    radius: f32,
}

impl SkeletonProgram {
    pub fn new(radius: f32) -> Self {
        Self { radius }
    }
}

impl<Message> shader::Program<Message> for SkeletonProgram {
    type State = ();
    type Primitive = SkeletonPrimitive;

    fn update(
        &self,
        _state: &mut Self::State,
        event: &Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        // A loading state shimmers for as long as it is shown, so every frame
        // asks for the next one.
        match event {
            Event::Window(window::Event::RedrawRequested(_)) => {
                Some(Action::request_redraw())
            },
            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: mouse::Cursor,
        _bounds: Rectangle,
    ) -> Self::Primitive {
        SkeletonPrimitive {
            radius: self.radius,
            // Reduce in f64 first, so the value handed to the shader stays small
            // and precise rather than growing without bound.
            time: (ANCHOR.elapsed().as_secs_f64() % CYCLE as f64) as f32,
        }
    }
}

/// One skeleton's per-frame data.
#[derive(Debug)]
pub struct SkeletonPrimitive {
    radius: f32,
    time: f32,
}

impl shader::Primitive for SkeletonPrimitive {
    type Pipeline = SkeletonPipeline;

    fn prepare(
        &self,
        pipeline: &mut Self::Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        viewport: &Viewport,
    ) {
        pipeline.prepare(device, queue, self, bounds, viewport);
    }

    fn draw(
        &self,
        pipeline: &Self::Pipeline,
        render_pass: &mut wgpu::RenderPass<'_>,
    ) -> bool {
        // Draw into the layer's shared pass, so hundreds of skeletons cost one
        // small draw call each rather than a render pass each.
        pipeline.draw(render_pass);
        true
    }
}

/// The packed `Shimmer` uniform; see `shimmer.wgsl`.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ShimmerUniform {
    viewport: [f32; 2],
    box_size: [f32; 2],
    base_color: [f32; 4],
    radius: f32,
    time: f32,
    cycle: f32,
    _pad0: f32,
}

impl ShimmerUniform {
    fn new(
        primitive: &SkeletonPrimitive,
        bounds: &Rectangle,
        viewport: &Viewport,
    ) -> Self {
        // The shader lifts this resting colour toward white for the highlight,
        // so it is passed as sRGB to brighten perceptually.
        let base = color::SECONDARY;

        Self {
            viewport: [
                viewport.physical_width() as f32,
                viewport.physical_height() as f32,
            ],
            box_size: [bounds.width, bounds.height],
            base_color: [base.r, base.g, base.b, base.a],
            radius: primitive.radius,
            time: primitive.time,
            cycle: CYCLE,
            _pad0: 0.0,
        }
    }
}

/// Byte size of one `Shimmer` uniform.
const UNIFORM_SIZE: u64 = size_of::<ShimmerUniform>() as u64;

/// The shared GPU resources for all skeletons.
///
/// Every instance writes its uniform into a slot of one dynamically-offset
/// buffer during `prepare`, then binds its slot during `draw`. iced prepares and
/// draws same-type primitives in the same order, so the prepare cursor and the
/// draw cursor address matching slots. `trim` marks the frame boundary that
/// resets both.
pub struct SkeletonPipeline {
    pipeline: wgpu::RenderPipeline,
    bind_layout: wgpu::BindGroupLayout,
    buffer: wgpu::Buffer,
    bind: wgpu::BindGroup,
    /// Per-instance slot stride, the uniform size aligned for dynamic offsets.
    stride: u64,
    /// Slots the buffer can hold.
    capacity: usize,
    /// This frame's uniforms, kept so a mid-frame grow can re-upload them.
    instances: Vec<ShimmerUniform>,
    prepare_cursor: usize,
    draw_cursor: AtomicUsize,
    /// How many slots the previous frame used, the hint for shrinking.
    used_last_frame: usize,
    new_frame: bool,
}

impl shader::Pipeline for SkeletonPipeline {
    fn new(
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("skeleton shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shimmer.wgsl").into()),
        });

        let bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("skeleton bind layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: wgpu::BufferSize::new(UNIFORM_SIZE),
                    },
                    count: None,
                }],
            });

        let pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("skeleton pipeline layout"),
                bind_group_layouts: &[&bind_layout],
                push_constant_ranges: &[],
            });

        // Straight-alpha blend so the rounded edge composites over the page.
        let blend = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("skeleton pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_fullscreen"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_shimmer"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(blend),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let alignment = device.limits().min_uniform_buffer_offset_alignment as u64;
        let stride = UNIFORM_SIZE.div_ceil(alignment) * alignment;
        let capacity = MIN_CAPACITY;
        let buffer = make_buffer(device, stride, capacity);
        let bind = make_bind(device, &bind_layout, &buffer);

        Self {
            pipeline,
            bind_layout,
            buffer,
            bind,
            stride,
            capacity,
            instances: Vec::new(),
            prepare_cursor: 0,
            draw_cursor: AtomicUsize::new(0),
            used_last_frame: 0,
            new_frame: true,
        }
    }

    fn trim(&mut self) {
        // Reset at the guaranteed end-of-frame hook rather than on the first
        // `prepare`, so the cursors never carry into a frame where `prepare` ran
        // for fewer primitives than `draw`.
        self.used_last_frame = self.prepare_cursor;
        self.instances.clear();
        self.prepare_cursor = 0;
        *self.draw_cursor.get_mut() = 0;
        self.new_frame = true;
    }
}

impl SkeletonPipeline {
    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        primitive: &SkeletonPrimitive,
        bounds: &Rectangle,
        viewport: &Viewport,
    ) {
        if self.new_frame {
            self.new_frame = false;
            self.shrink_to_fit(device);
        }

        // The renderer skips `draw` for primitives that fall outside the window,
        // so skip them here too. Otherwise the prepare and draw cursors would
        // drift apart when a skeleton scrolls off screen and the wrong slot
        // would feed a visible one. This mirrors the renderer's own test.
        if !visible(bounds, viewport) {
            return;
        }

        let uniform = ShimmerUniform::new(primitive, bounds, viewport);
        let slot = self.prepare_cursor;
        self.instances.push(uniform);
        self.prepare_cursor += 1;

        if self.prepare_cursor > self.capacity {
            self.grow(device, queue);
        } else {
            queue.write_buffer(
                &self.buffer,
                slot as u64 * self.stride,
                bytemuck::bytes_of(&uniform),
            );
        }
    }

    /// Shrinks the buffer toward the previous frame's need, so a one-off dense
    /// frame does not pin the allocation for the rest of the run.
    fn shrink_to_fit(&mut self, device: &wgpu::Device) {
        let target = self.used_last_frame.next_power_of_two().max(MIN_CAPACITY);
        if target >= self.capacity {
            return;
        }

        self.capacity = target;
        self.buffer = make_buffer(device, self.stride, self.capacity);
        self.bind = make_bind(device, &self.bind_layout, &self.buffer);
    }

    /// Doubles the buffer to fit the frame and re-uploads what it holds so far.
    fn grow(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        while self.prepare_cursor > self.capacity {
            self.capacity *= 2;
        }
        self.buffer = make_buffer(device, self.stride, self.capacity);
        self.bind = make_bind(device, &self.bind_layout, &self.buffer);

        for (slot, uniform) in self.instances.iter().enumerate() {
            queue.write_buffer(
                &self.buffer,
                slot as u64 * self.stride,
                bytemuck::bytes_of(uniform),
            );
        }
    }

    fn draw(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        let slot = self.draw_cursor.fetch_add(1, Ordering::Relaxed);

        // The renderer's layer culling differs subtly from prepare's, so it can
        // draw a primitive prepare skipped. Bind only slots that were written
        // this frame, turning any overrun into a dropped box rather than a stale
        // slot or an out-of-range dynamic offset.
        if slot >= self.instances.len() {
            return;
        }

        let offset = (slot as u64 * self.stride) as u32;

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind, &[offset]);
        render_pass.draw(0..3, 0..1);
    }
}

/// Whether `bounds` is on screen, matching the renderer's own per-primitive
/// visibility test so this side and the draw side agree on which slots exist.
fn visible(bounds: &Rectangle, viewport: &Viewport) -> bool {
    let physical = Rectangle::with_size(Size::new(
        viewport.physical_width() as f32,
        viewport.physical_height() as f32,
    ));

    (*bounds * Transformation::scale(viewport.scale_factor()))
        .intersection(&physical)
        .and_then(Rectangle::snap)
        .is_some()
}

/// Allocates a dynamically-offset uniform buffer of `capacity` slots.
fn make_buffer(device: &wgpu::Device, stride: u64, capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("skeleton instances"),
        size: stride * capacity as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

/// Binds the buffer at one instance's worth of size, addressed by dynamic offset.
fn make_bind(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("skeleton bind"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer,
                offset: 0,
                size: wgpu::BufferSize::new(UNIFORM_SIZE),
            }),
        }],
    })
}
