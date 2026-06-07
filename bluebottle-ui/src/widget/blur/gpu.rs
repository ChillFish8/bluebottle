//! GPU constants and small helpers for the shared separable blur path.
//!
//! Consumer widgets build their own composite pipelines around these
//! primitives. The intent is one canonical Gaussian blur shader, sampler, and
//! uniform layout used by every shader widget that needs frosted output.

use iced::wgpu;

/// sRGB format for uploaded source images, so sampling decodes them to linear.
pub const SOURCE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// Linear 16-bit-float format for the blur intermediates. The extra precision
/// keeps the smooth blur from banding when it is later up-scaled.
pub const BLUR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// 8 `f32`s; see the `Blur` struct in `shader.wgsl`.
pub const BLUR_UNIFORM_SIZE: u64 = 8 * 4;

/// Runs one full-screen separable-blur pass into `target` with `bind`.
pub fn blur_pass(
    encoder: &mut wgpu::CommandEncoder,
    pipeline: &wgpu::RenderPipeline,
    target: &wgpu::TextureView,
    bind: &wgpu::BindGroup,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("blur pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            resolve_target: None,
            depth_slice: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, bind, &[]);
    pass.draw(0..3, 0..1);
}

/// Linear-filtered, clamped sampler shared across blur source and composite.
pub fn sampler(device: &wgpu::Device, label: &str) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some(label),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    })
}

/// Allocates a buffer sized for a single `Blur` uniform.
pub fn blur_uniform_buffer(device: &wgpu::Device, label: &str) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: BLUR_UNIFORM_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

/// Packs the 8-float `Blur` uniform; see `shader.wgsl`.
pub fn pack_blur_uniform(texel: [f32; 2], direction: [f32; 2], radius: f32) -> [f32; 8] {
    [
        texel[0],
        texel[1],
        direction[0],
        direction[1],
        radius,
        0.0,
        0.0,
        0.0,
    ]
}
