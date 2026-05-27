use iced::wgpu;

/// sRGB format for uploaded source images, so sampling decodes them to linear.
pub const SOURCE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// Linear 16-bit-float format for the blur intermediates. The extra precision
/// keeps the smooth blur from banding when it is later up-scaled.
pub const BLUR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

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
