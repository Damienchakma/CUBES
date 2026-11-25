use anyhow::{Context, Result, bail};
use glam::{Mat4, Quat, Vec3};
use serde_json::json;
use std::time::Instant;
use vela_renderer::{
    BackendPolicy, Camera, CameraProjection, GpuContext, RenderInstance, SceneRenderer,
};
use vela_scene::PrimitiveKind;

#[derive(Debug)]
struct Options {
    width: u32,
    height: u32,
    instances: usize,
    frames: usize,
    warmup: usize,
    backend: BackendPolicy,
    output: Option<std::path::PathBuf>,
}

fn main() -> Result<()> {
    let options = parse_arguments(std::env::args().skip(1))?;
    let hardware = vela_platform::probe().context("hardware probe failed")?;
    let gpu = pollster::block_on(GpuContext::create(options.backend))
        .context("hardware-accelerated graphics adapter could not be opened")?;
    println!("Hardware: {}", hardware.board);
    println!("Adapter: {}", gpu.report);

    let color_format = wgpu::TextureFormat::Rgba8Unorm;
    let depth_format = wgpu::TextureFormat::Depth24Plus;
    let color = create_texture(
        &gpu.device,
        options.width,
        options.height,
        color_format,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
    );
    let depth = create_texture(
        &gpu.device,
        options.width,
        options.height,
        depth_format,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let color_view = color.create_view(&wgpu::TextureViewDescriptor::default());
    let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());
    let mut renderer = SceneRenderer::new(&gpu.device, &gpu.queue, color_format, depth_format);

    let instances = benchmark_instances(options.instances);
    let camera = Camera {
        position: Vec3::new(10.0, 9.0, 14.0),
        target: Vec3::new(0.0, 0.0, 0.0),
        projection: CameraProjection::Perspective {
            vertical_fov_degrees: 55.0,
        },
        near: 0.1,
        far: 500.0,
    };
    let aspect = options.width as f32 / options.height as f32;

    for _ in 0..options.warmup {
        render_frame(
            &gpu,
            &mut renderer,
            &color_view,
            &depth_view,
            &camera,
            aspect,
            &instances,
        )?;
    }

    let mut timings = Vec::with_capacity(options.frames);
    let mut metrics = vela_renderer::RenderMetrics::default();
    for _ in 0..options.frames {
        let start = Instant::now();
        metrics = render_frame(
            &gpu,
            &mut renderer,
            &color_view,
            &depth_view,
            &camera,
            aspect,
            &instances,
        )?;
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .context("GPU wait failed")?;
        timings.push(start.elapsed().as_secs_f64() * 1_000.0);
    }

    timings.sort_by(|a, b| a.total_cmp(b));
    let average = timings.iter().sum::<f64>() / timings.len() as f64;
    let p95 = timings[(timings.len() * 95 / 100).min(timings.len() - 1)];
    let result = json!({
        "hardware": hardware,
        "adapter": gpu.report,
        "configuration": {
            "width": options.width,
            "height": options.height,
            "instances": options.instances,
            "frames": options.frames,
            "warmupFrames": options.warmup,
            "backend": format!("{:?}", gpu.backend()),
            "presentation": "offscreen; measured CPU encoding plus GPU completion"
        },
        "results": {
            "averageFrameMilliseconds": average,
            "p95FrameMilliseconds": p95,
            "throughputFramesPerSecond": 1_000.0 / average,
            "drawCalls": metrics.draw_calls,
            "triangles": metrics.triangles,
            "instances": metrics.instances,
        }
    });
    println!("{}", serde_json::to_string_pretty(&result)?);
    if let Some(path) = options.output {
        let parent = path.parent().unwrap_or(std::path::Path::new("."));
        std::fs::create_dir_all(parent)
            .context("benchmark output directory could not be created")?;
        std::fs::write(&path, serde_json::to_vec_pretty(&result)?).with_context(|| {
            format!(
                "benchmark result could not be written to {}",
                path.display()
            )
        })?;
        println!("Saved results to {}", path.display());
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn render_frame(
    gpu: &GpuContext,
    renderer: &mut SceneRenderer,
    color_view: &wgpu::TextureView,
    depth_view: &wgpu::TextureView,
    camera: &Camera,
    aspect: f32,
    instances: &[RenderInstance],
) -> Result<vela_renderer::RenderMetrics> {
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("vela-bench-encoder"),
        });
    let metrics = {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("vela-bench-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        renderer.render(
            &mut pass,
            &gpu.device,
            &gpu.queue,
            camera,
            aspect,
            instances,
            Vec3::new(-0.4, -1.0, -0.3),
            [1.0, 0.93, 0.84],
            3.0,
        )
    };
    gpu.queue.submit([encoder.finish()]);
    Ok(metrics)
}

fn create_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("vela-bench-target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}

fn benchmark_instances(count: usize) -> Vec<RenderInstance> {
    let kinds = [
        PrimitiveKind::Cube,
        PrimitiveKind::Sphere,
        PrimitiveKind::Plane,
        PrimitiveKind::Cylinder,
    ];
    let mut state = 0x9e37_79b9_7f4a_7c15_u64;
    let mut instances = Vec::with_capacity(count);
    let columns = (count as f64).sqrt().ceil() as usize;
    for index in 0..count {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let random = (state >> 11) as f64 / (1_u64 << 53) as f64;
        let x = (index % columns) as f32 - columns as f32 * 0.5;
        let z = (index / columns) as f32 - columns as f32 * 0.5;
        let scale = 0.18 + random as f32 * 0.22;
        instances.push(RenderInstance {
            primitive: kinds[index % kinds.len()],
            asset: None,
            texture: None,
            model: Mat4::from_scale_rotation_translation(
                Vec3::splat(scale),
                Quat::from_rotation_y(random as f32 * std::f32::consts::TAU),
                Vec3::new(x * 0.8, 0.0, z * 0.8),
            ),
            base_color: [0.18 + random as f32 * 0.3, 0.52, 0.85, 1.0],
            metallic: 0.05,
            roughness: 0.5,
            emissive: 0.0,
        });
    }
    instances
}

fn parse_arguments(arguments: impl Iterator<Item = String>) -> Result<Options> {
    let mut options = Options {
        width: 1280,
        height: 720,
        instances: 512,
        frames: 300,
        warmup: 30,
        backend: BackendPolicy::PreferVulkan,
        output: None,
    };
    let mut arguments = arguments;
    while let Some(argument) = arguments.next() {
        let (name, inline) = argument
            .split_once('=')
            .map_or((argument.as_str(), None), |(name, value)| {
                (name, Some(value.to_owned()))
            });
        let value = if let Some(value) = inline {
            value
        } else {
            arguments
                .next()
                .ok_or_else(|| anyhow::anyhow!("--{name} requires a value"))?
        };
        match name {
            "--width" => options.width = value.parse().context("invalid width")?,
            "--height" => options.height = value.parse().context("invalid height")?,
            "--instances" => options.instances = value.parse().context("invalid instance count")?,
            "--frames" => options.frames = value.parse().context("invalid frame count")?,
            "--warmup" => options.warmup = value.parse().context("invalid warmup count")?,
            "--backend" => {
                options.backend = match value.as_str() {
                    "vulkan" => BackendPolicy::PreferVulkan,
                    "gl" | "opengl" => BackendPolicy::PreferOpenGl,
                    "all" => BackendPolicy::AllHardware,
                    other => bail!("unknown backend: {other}"),
                }
            }
            "--output" => options.output = Some(value.into()),
            other => bail!("unknown benchmark option: {other}"),
        }
    }
    if options.width == 0 || options.height == 0 {
        bail!("width and height must be positive");
    }
    if options.instances == 0 || options.frames == 0 {
        bail!("instances and frames must be positive");
    }
    Ok(options)
}
