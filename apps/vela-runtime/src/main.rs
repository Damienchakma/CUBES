use anyhow::{Context, Result, anyhow};
use glam::Vec3;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use vela_core::{EntityId, Project};
use vela_renderer::{
    BackendPolicy, Camera, CameraProjection, GpuContext, ImportedTexture, RenderInstance,
    RenderMetrics, SceneRenderer, load_gltf_mesh, write_rgba_png,
};
use vela_scene::{
    BodyType, ColliderShape, Component, ComponentKind, MaterialComponent, Scene, Transform,
};
use vela_scripting::ScriptRuntime;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

struct RuntimeApplication {
    session: Option<RuntimeSession>,
    project_path: Option<PathBuf>,
    error: Option<anyhow::Error>,
}

impl RuntimeApplication {
    fn new(project_path: Option<PathBuf>) -> Self {
        Self {
            session: None,
            project_path,
            error: None,
        }
    }
}

impl ApplicationHandler for RuntimeApplication {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.session.is_some() || self.error.is_some() {
            return;
        }
        match pollster::block_on(RuntimeSession::new(event_loop, self.project_path.take())) {
            Ok(session) => self.session = Some(session),
            Err(error) => {
                self.error = Some(error);
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let redraw = matches!(event, WindowEvent::RedrawRequested);
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    session.surface_config.width = size.width;
                    session.surface_config.height = size.height;
                    session
                        .surface
                        .configure(&session.gpu.device, &session.surface_config);
                    let (texture, view) = create_depth_texture(
                        &session.gpu.device,
                        size.width,
                        size.height,
                        session.depth_format,
                    );
                    session.depth_texture = texture;
                    session.depth_view = view;
                }
            }
            _ => {}
        }
        if redraw && let Err(error) = session.draw() {
            log::error!("runtime draw failed: {error:#}");
            self.error = Some(error);
            event_loop.exit();
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(session) = &self.session {
            session.window.request_redraw();
        }
    }
}

struct RuntimeSession {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    gpu: GpuContext,
    surface_config: wgpu::SurfaceConfiguration,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    depth_format: wgpu::TextureFormat,
    renderer: SceneRenderer,
    project: Project,
    scene: Scene,
    scripts: ScriptRuntime,
    velocities: HashMap<EntityId, Vec3>,
    last_frame: Instant,
    last_report: Instant,
    report_frames: u32,
    report_metrics: RenderMetrics,
    loaded_assets: HashSet<String>,
}

impl RuntimeSession {
    async fn new(event_loop: &ActiveEventLoop, project_path: Option<PathBuf>) -> Result<Self> {
        let root = project_path.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        let project = Project::open(&root).context("project could not be opened")?;
        let scene_path = project
            .paths()
            .resolve_existing(&project.manifest().main_scene)
            .context("main scene is missing")?;
        let scene = Scene::load(scene_path).context("main scene could not be loaded")?;

        let scripts = load_scripts(&project)?;

        let attributes = Window::default_attributes()
            .with_title(format!("{} - CUBES", project.manifest().name))
            .with_inner_size(winit::dpi::LogicalSize::new(
                project.manifest().render.width.max(320),
                project.manifest().render.height.max(240),
            ));
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .context("runtime window could not be created")?,
        );
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backends = wgpu::Backends::VULKAN | wgpu::Backends::GL;
        descriptor.display = Some(Box::new(event_loop.owned_display_handle()));
        let instance = wgpu::Instance::new(descriptor);
        let surface = instance
            .create_surface(Arc::clone(&window))
            .context("wgpu surface could not be created")?;
        let gpu = GpuContext::from_instance_for_surface(
            instance,
            BackendPolicy::AllHardware,
            Some(&surface),
        )
        .await
        .context("hardware-accelerated graphics adapter could not be opened")?;

        let size = window.inner_size();
        let capabilities = surface.get_capabilities(&gpu.adapter);
        let format = if capabilities
            .formats
            .contains(&wgpu::TextureFormat::Bgra8Unorm)
        {
            wgpu::TextureFormat::Bgra8Unorm
        } else if capabilities
            .formats
            .contains(&wgpu::TextureFormat::Rgba8Unorm)
        {
            wgpu::TextureFormat::Rgba8Unorm
        } else {
            *capabilities
                .formats
                .first()
                .ok_or_else(|| anyhow!("surface exposes no renderable texture format"))?
        };
        let mut surface_config = surface
            .get_default_config(&gpu.adapter, size.width.max(1), size.height.max(1))
            .ok_or_else(|| anyhow!("selected adapter is not compatible with the surface"))?;
        surface_config.format = format;
        surface_config.usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
        surface_config.present_mode = if project.manifest().render.vsync {
            wgpu::PresentMode::Fifo
        } else {
            wgpu::PresentMode::Immediate
        };
        surface_config.alpha_mode = wgpu::CompositeAlphaMode::Opaque;
        surface_config.view_formats = vec![format];
        surface.configure(&gpu.device, &surface_config);

        let depth_format = wgpu::TextureFormat::Depth24Plus;
        let (depth_texture, depth_view) = create_depth_texture(
            &gpu.device,
            size.width.max(1),
            size.height.max(1),
            depth_format,
        );
        let renderer = SceneRenderer::new(&gpu.device, &gpu.queue, format, depth_format);

        log::info!("runtime graphics adapter: {}", gpu.report);
        Ok(Self {
            window,
            surface,
            gpu,
            surface_config,
            depth_texture,
            depth_view,
            depth_format,
            renderer,
            project,
            scene,
            scripts,
            velocities: HashMap::new(),
            last_frame: Instant::now(),
            last_report: Instant::now(),
            report_frames: 0,
            report_metrics: RenderMetrics::default(),
            loaded_assets: HashSet::new(),
        })
    }

    fn draw(&mut self) -> Result<()> {
        let now = Instant::now();
        let delta = (now.duration_since(self.last_frame).as_secs_f32()).min(0.05);
        self.last_frame = now;

        self.step(delta);
        let clear_color = self.clear_color();
        let camera = self.camera();

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                self.surface
                    .configure(&self.gpu.device, &self.surface_config);
                frame
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface
                    .configure(&self.gpu.device, &self.surface_config);
                return Ok(());
            }
            _ => return Ok(()),
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("vela-runtime-encoder"),
            });
        let metrics = {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("vela-runtime-frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
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
            self.load_asset_meshes();
            let instances = self.render_instances();
            let (direction, color, intensity) = self.primary_light();
            self.renderer.render(
                &mut pass,
                &self.gpu.device,
                &self.gpu.queue,
                &camera,
                self.surface_config.width as f32 / self.surface_config.height as f32,
                &instances,
                direction,
                color,
                intensity,
            )
        };
        self.gpu.queue.submit([encoder.finish()]);
        self.gpu.queue.present(frame);

        self.report_frames += 1;
        self.report_metrics = metrics;
        let report_elapsed = now.duration_since(self.last_report).as_secs_f32();
        if report_elapsed >= 2.0 {
            let fps = self.report_frames as f32 / report_elapsed;
            log::info!(
                "runtime: {fps:.1} FPS, {} draw calls, {} triangles, {} instances",
                self.report_metrics.draw_calls,
                self.report_metrics.triangles,
                self.report_metrics.instances
            );
            self.report_frames = 0;
            self.last_report = now;
        }
        Ok(())
    }

    fn step(&mut self, delta: f32) {
        if self.scripts.has_function("on_update") {
            for id in self.scene.entities.keys().copied().collect::<Vec<_>>() {
                if let Err(error) = self.scripts.call_update(id, delta) {
                    log::warn!("script update failed for entity {id}: {error}");
                    break;
                }
            }
        }

        let floor = -1.0;
        let bodies = self
            .scene
            .entities
            .values()
            .filter_map(|entity| {
                let Component::Physics(physics) = entity.components.get(&ComponentKind::Physics)?
                else {
                    return None;
                };
                if physics.body_type != BodyType::Dynamic {
                    return None;
                }
                let half_height = match physics.collider {
                    ColliderShape::Box { half_extents } => half_extents[1],
                    ColliderShape::Sphere { radius } | ColliderShape::Capsule { radius, .. } => {
                        radius
                    }
                };
                Some((
                    entity.id,
                    half_height,
                    physics.restitution,
                    physics.friction.min(1.0),
                ))
            })
            .collect::<Vec<_>>();
        for (id, half_height, restitution, friction) in bodies {
            let velocity = self.velocities.entry(id).or_default();
            velocity.y -= 9.81 * delta;
            let velocity = *velocity;
            if let Ok(Component::Transform(transform)) =
                self.scene.component_mut(id, ComponentKind::Transform)
            {
                transform.translation += velocity * delta;
                let minimum = floor + half_height;
                if transform.translation.y < minimum {
                    transform.translation.y = minimum;
                    let mut bounced = velocity;
                    bounced.y = if velocity.y.abs() > 0.2 {
                        -velocity.y * restitution
                    } else {
                        0.0
                    };
                    bounced.x *= 1.0 - friction;
                    bounced.z *= 1.0 - friction;
                    self.velocities.insert(id, bounced);
                }
            }
        }
    }

    fn camera(&self) -> Camera {
        for entity in self.scene.entities.values() {
            let Some(Component::Camera(camera)) = entity.components.get(&ComponentKind::Camera)
            else {
                continue;
            };
            let transform = match entity.components.get(&ComponentKind::Transform) {
                Some(Component::Transform(transform)) => *transform,
                _ => Transform::default(),
            };
            let forward = transform.rotation * Vec3::NEG_Z;
            return Camera {
                position: transform.translation,
                target: transform.translation + forward,
                projection: CameraProjection::Perspective {
                    vertical_fov_degrees: camera.vertical_fov_degrees,
                },
                near: camera.near,
                far: camera.far,
            };
        }
        Camera::default()
    }

    fn clear_color(&self) -> wgpu::Color {
        for entity in self.scene.entities.values() {
            if let Some(Component::Camera(camera)) = entity.components.get(&ComponentKind::Camera) {
                let color = camera.clear_color;
                return wgpu::Color {
                    r: color[0] as f64,
                    g: color[1] as f64,
                    b: color[2] as f64,
                    a: color[3] as f64,
                };
            }
        }
        wgpu::Color::BLACK
    }

    fn primary_light(&self) -> (Vec3, [f32; 3], f32) {
        for entity in self.scene.entities.values() {
            if let Some(Component::Light(light)) = entity.components.get(&ComponentKind::Light) {
                let translation = match entity.components.get(&ComponentKind::Transform) {
                    Some(Component::Transform(transform)) => transform.translation,
                    _ => Vec3::new(4.0, 8.0, 3.0),
                };
                return (
                    -translation.normalize_or_zero(),
                    light.color,
                    light.intensity,
                );
            }
        }
        (Vec3::new(-0.4, -1.0, -0.3), [1.0, 0.92, 0.82], 3.0)
    }

    fn load_asset_meshes(&mut self) {
        let root = self.project.paths().root().to_path_buf();
        let requested = self
            .scene
            .entities
            .values()
            .filter_map(|entity| {
                let Component::Mesh(mesh) = entity.components.get(&ComponentKind::Mesh)? else {
                    return None;
                };
                let asset = mesh.asset.as_ref()?;
                if self.loaded_assets.contains(asset) {
                    return None;
                }
                let path = self.project.paths().resolve_existing(asset);
                Some((asset.clone(), entity.id, path))
            })
            .collect::<Vec<_>>();
        for (asset, id, path) in requested {
            self.loaded_assets.insert(asset.clone());
            let result = path.map_err(anyhow::Error::from).and_then(|path| {
                let imported = load_gltf_mesh(&path)?;
                self.renderer
                    .load_asset_mesh(&self.gpu.device, &asset, &imported.data);
                if let Some(texture) = imported.texture {
                    let key = texture_key(&root, &texture);
                    if let Some(parent) = Path::new(&key).parent() {
                        let _ = std::fs::create_dir_all(root.join(parent));
                    }
                    let _ = write_rgba_png(
                        &root.join(&key),
                        &texture.rgba,
                        texture.width,
                        texture.height,
                    );
                    self.renderer.load_texture(
                        &self.gpu.device,
                        &self.gpu.queue,
                        &key,
                        &texture.rgba,
                        texture.width,
                        texture.height,
                    );
                    let material = self
                        .scene
                        .component(id, ComponentKind::Material)
                        .ok()
                        .and_then(|component| match component {
                            Component::Material(material) => Some(material.clone()),
                            _ => None,
                        })
                        .unwrap_or_default();
                    let _ = self.scene.add_component(
                        id,
                        Component::Material(MaterialComponent {
                            base_color: imported.base_color,
                            base_color_texture: Some(key),
                            ..material
                        }),
                    );
                }
                Ok(())
            });
            if let Err(error) = result {
                log::warn!("Could not load mesh '{asset}' for entity {id}: {error}");
            }
        }
    }

    fn render_instances(&self) -> Vec<RenderInstance> {
        let mut instances = Vec::new();
        for entity in self.scene.entities.values() {
            if !entity.visible {
                continue;
            }
            let Some(Component::Mesh(mesh)) = entity.components.get(&ComponentKind::Mesh) else {
                continue;
            };
            if !mesh.visible {
                continue;
            }
            let Ok(model) = self.scene.world_transform(entity.id) else {
                continue;
            };
            let material = match entity.components.get(&ComponentKind::Material) {
                Some(Component::Material(material)) => material.clone(),
                _ => Default::default(),
            };
            instances.push(RenderInstance {
                primitive: mesh.primitive,
                asset: mesh.asset.clone(),
                texture: material.base_color_texture.clone(),
                model,
                base_color: material.base_color,
                metallic: material.metallic,
                roughness: material.roughness,
                emissive: material.emissive[0],
            });
        }
        instances
    }
}

fn load_scripts(project: &Project) -> Result<ScriptRuntime> {
    let runtime = ScriptRuntime::new();
    let scripts_dir = project.paths().scripts_dir();
    let mut paths = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&scripts_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|extension| extension == "lua") {
                paths.push(path);
            }
        }
    }
    paths.sort();
    for path in paths {
        let canonical = path
            .canonicalize()
            .with_context(|| format!("script could not be read: {}", path.display()))?;
        if !canonical.starts_with(project.paths().root()) {
            continue;
        }
        let source = std::fs::read_to_string(canonical).context("script could not be decoded")?;
        runtime
            .execute(&source)
            .map_err(|error| anyhow!("script failed ({}): {error}", path.display()))?;
    }
    runtime
        .call_start()
        .map_err(|error| anyhow!("on_start failed: {error}"))?;
    Ok(runtime)
}

fn texture_key(root: &Path, texture: &ImportedTexture) -> String {
    if let Some(source) = &texture.source_path
        && let Ok(relative) = source.strip_prefix(root)
    {
        return relative.to_string_lossy().replace('\\', "/");
    }
    PathBuf::from("assets")
        .join(&texture.key)
        .to_string_lossy()
        .replace('\\', "/")
}

fn create_depth_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("vela-runtime-depth"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let event_loop =
        winit::event_loop::EventLoop::new().context("event loop could not be created")?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let project_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok());
    let mut application = RuntimeApplication::new(project_path);
    event_loop.run_app(&mut application)?;
    if let Some(error) = application.error {
        return Err(error);
    }
    Ok(())
}
