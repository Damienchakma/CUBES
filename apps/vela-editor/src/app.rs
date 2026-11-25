use crate::state::{EditorState, PlayMode};
use crate::ui;
use anyhow::{Context, Result, anyhow};
use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use vela_renderer::{
    BackendPolicy, Camera, CameraProjection, GpuContext, ImportedTexture, RenderInstance,
    RenderMetrics, SceneRenderer, load_gltf_mesh, write_rgba_png,
};
use vela_scene::{Component, ComponentKind, MaterialComponent};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::Key;
use winit::window::{Window, WindowId};

pub struct EditorApplication {
    app: Option<EditorApp>,
    project_path: Option<PathBuf>,
    init_error: Option<anyhow::Error>,
}

impl EditorApplication {
    pub fn new(project_path: Option<PathBuf>) -> Self {
        Self {
            app: None,
            project_path,
            init_error: None,
        }
    }

    pub fn take_error(&mut self) -> Option<anyhow::Error> {
        self.init_error.take()
    }
}

impl ApplicationHandler for EditorApplication {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.app.is_some() || self.init_error.is_some() {
            return;
        }
        match pollster::block_on(EditorApp::new(event_loop, self.project_path.take())) {
            Ok(app) => self.app = Some(app),
            Err(error) => {
                self.init_error = Some(error);
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
        let Some(app) = self.app.as_mut() else {
            return;
        };
        let redraw = matches!(event, WindowEvent::RedrawRequested);
        app.handle_window_event(&event, event_loop);
        if redraw && let Err(error) = app.draw() {
            log::error!("editor draw failed: {error:#}");
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(app) = &self.app {
            app.window.request_redraw();
        }
    }
}

pub struct EditorApp {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    gpu: GpuContext,
    surface_config: wgpu::SurfaceConfiguration,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    scene_renderer: SceneRenderer,
    egui_context: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
    state: EditorState,
    frame_times: VecDeque<f32>,
    last_frame: Instant,
    metrics: RenderMetrics,
    loaded_assets: HashSet<String>,
}

impl EditorApp {
    pub async fn new(event_loop: &ActiveEventLoop, project_path: Option<PathBuf>) -> Result<Self> {
        let attributes = Window::default_attributes()
            .with_title("CUBES Editor")
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0))
            .with_min_inner_size(winit::dpi::LogicalSize::new(960.0, 600.0));
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .context("editor window could not be created")?,
        );

        let display_handle = event_loop.owned_display_handle();
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backends = wgpu::Backends::VULKAN | wgpu::Backends::GL;
        descriptor.display = Some(Box::new(display_handle));
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
            .ok_or_else(|| anyhow!("selected adapter is not compatible with the window surface"))?;
        surface_config.format = format;
        surface_config.usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
        surface_config.present_mode = wgpu::PresentMode::Fifo;
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
        let scene_renderer = SceneRenderer::new(&gpu.device, &gpu.queue, format, depth_format);

        let egui_context = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_context.clone(),
            egui::ViewportId::ROOT,
            window.as_ref(),
            None,
            None,
            Some(gpu.capabilities.max_texture_dimension_2d as usize),
        );
        let egui_renderer = egui_wgpu::Renderer::new(
            &gpu.device,
            format,
            egui_wgpu::RendererOptions {
                msaa_samples: 1,
                depth_stencil_format: None,
                ..Default::default()
            },
        );

        let mut state = EditorState::default();
        if let Some(path) = project_path {
            if let Err(error) = state.open_project(&path) {
                state.log(crate::state::LogLevel::Error, error.to_string());
            }
        }
        state.graphics_backend = gpu.report.to_string();
        state.log(
            crate::state::LogLevel::Info,
            format!("Graphics adapter: {}", gpu.report),
        );

        Ok(Self {
            window,
            surface,
            gpu,
            surface_config,
            depth_texture,
            depth_view,
            scene_renderer,
            egui_context,
            egui_state,
            egui_renderer,
            state,
            frame_times: VecDeque::with_capacity(240),
            last_frame: Instant::now(),
            metrics: RenderMetrics::default(),
            loaded_assets: HashSet::new(),
        })
    }

    pub fn handle_window_event(&mut self, event: &WindowEvent, event_loop: &ActiveEventLoop) {
        let _ = self.egui_state.on_window_event(&self.window, event);
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    self.surface_config.width = size.width;
                    self.surface_config.height = size.height;
                    self.surface
                        .configure(&self.gpu.device, &self.surface_config);
                    let (texture, view) = create_depth_texture(
                        &self.gpu.device,
                        size.width,
                        size.height,
                        wgpu::TextureFormat::Depth24Plus,
                    );
                    self.depth_texture = texture;
                    self.depth_view = view;
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.state.is_pressed() && !event.repeat =>
            {
                if event.logical_key == Key::Named(winit::keyboard::NamedKey::Save) {
                    if let Err(error) = self.state.save_scene() {
                        self.state
                            .log(crate::state::LogLevel::Error, error.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    pub fn draw(&mut self) -> Result<()> {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame).as_secs_f32() * 1_000.0;
        self.last_frame = now;
        if elapsed > 0.0 && elapsed < 1_000.0 {
            if self.frame_times.len() == self.frame_times.capacity() {
                self.frame_times.pop_front();
            }
            self.frame_times.push_back(elapsed);
        }

        self.update_play_preview();
        let raw_input = self.egui_state.take_egui_input(&self.window);
        let output = {
            let EditorApp {
                egui_context,
                state,
                ..
            } = self;
            egui_context.run_ui(raw_input, |root| ui::show(root, state))
        };

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

        let surface_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("vela-editor-encoder"),
            });

        let viewport = self.state.viewport_rect;
        let pixels_per_point = self.window.scale_factor() as f32;
        let viewport_pixels = viewport * pixels_per_point;
        let surface_width = self.surface_config.width as f32;
        let surface_height = self.surface_config.height as f32;
        let x = viewport_pixels.min.x.max(0.0);
        let top = viewport_pixels.min.y.max(0.0);
        let right = viewport_pixels.max.x.min(surface_width);
        let bottom = viewport_pixels.max.y.min(surface_height);
        let width = (right - x).max(0.0);
        let height = (bottom - top).max(0.0);
        let y = surface_height - bottom;

        {
            let mut scene_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("vela-editor-viewport"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.018,
                            g: 0.026,
                            b: 0.039,
                            a: 1.0,
                        }),
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
            if width > 0.0 && height > 0.0 {
                scene_pass.set_viewport(x, y, width, height, 0.0, 1.0);
                scene_pass.set_scissor_rect(x as u32, y as u32, width as u32, height as u32);
                let camera = self.editor_camera();
                let aspect = width / height;
                let (light_direction, light_color, light_intensity) = self.primary_light();
                self.load_asset_meshes();
                let instances = self.render_instances();
                self.metrics = self.scene_renderer.render(
                    &mut scene_pass,
                    &self.gpu.device,
                    &self.gpu.queue,
                    &camera,
                    aspect,
                    &instances,
                    light_direction,
                    light_color,
                    light_intensity,
                );
            }
        }

        let clipped_primitives = self
            .egui_context
            .tessellate(output.shapes, output.pixels_per_point);
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.surface_config.width, self.surface_config.height],
            pixels_per_point,
        };
        let mut textures_delta = output.textures_delta;
        for (id, image_delta) in textures_delta.set.drain() {
            for image_delta in image_delta {
                self.egui_renderer.update_texture(
                    &self.gpu.device,
                    &self.gpu.queue,
                    id,
                    &image_delta,
                );
            }
        }
        self.egui_renderer.update_buffers(
            &self.gpu.device,
            &self.gpu.queue,
            &mut encoder,
            &clipped_primitives,
            &screen_descriptor,
        );
        {
            let ui_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("vela-editor-ui"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            self.egui_renderer.render(
                &mut ui_pass.forget_lifetime(),
                &clipped_primitives,
                &screen_descriptor,
            );
        }

        let frame_time = if self.frame_times.is_empty() {
            0.0
        } else {
            self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32
        };
        self.state.performance = crate::state::PerformanceStats {
            frames_per_second: if frame_time > 0.0 {
                1_000.0 / frame_time
            } else {
                0.0
            },
            frame_time_milliseconds: frame_time,
            draw_calls: self.metrics.draw_calls,
            triangles: self.metrics.triangles,
            instances: self.metrics.instances,
        };

        self.gpu.queue.submit([encoder.finish()]);
        self.gpu.queue.present(frame);
        Ok(())
    }

    fn editor_camera(&self) -> Camera {
        Camera {
            position: self.state.camera.position(self.state.camera_target),
            target: self.state.camera_target,
            projection: CameraProjection::Perspective {
                vertical_fov_degrees: 60.0,
            },
            near: 0.05,
            far: 500.0,
        }
    }

    fn primary_light(&self) -> (glam::Vec3, [f32; 3], f32) {
        for entity in self.state.scene.entities.values() {
            if let Some(Component::Light(light)) = entity.components.get(&ComponentKind::Light) {
                let translation = match entity.components.get(&ComponentKind::Transform) {
                    Some(Component::Transform(transform)) => transform.translation,
                    _ => glam::Vec3::new(4.0, 8.0, 3.0),
                };
                return (
                    -translation.normalize_or_zero(),
                    light.color,
                    light.intensity,
                );
            }
        }
        (glam::Vec3::new(-0.4, -1.0, -0.3), [1.0, 0.92, 0.82], 3.0)
    }

    fn load_asset_meshes(&mut self) {
        let Some(project) = self.state.project.as_ref() else {
            return;
        };
        let root = project.paths().root().to_path_buf();
        let requested = self
            .state
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
                let path = project.paths().resolve_existing(asset);
                Some((asset.clone(), entity.id, path))
            })
            .collect::<Vec<_>>();
        for (asset, id, path) in requested {
            self.loaded_assets.insert(asset.clone());
            let result = path.map_err(anyhow::Error::from).and_then(|path| {
                let imported = load_gltf_mesh(&path)?;
                self.scene_renderer
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
                    self.scene_renderer.load_texture(
                        &self.gpu.device,
                        &self.gpu.queue,
                        &key,
                        &texture.rgba,
                        texture.width,
                        texture.height,
                    );
                    self.state
                        .set_imported_material(id, imported.base_color, key);
                }
                Ok(())
            });
            if let Err(error) = result {
                self.state.log(
                    crate::state::LogLevel::Error,
                    format!("Could not load mesh '{asset}' for entity {id}: {error}"),
                );
            }
        }
    }

    fn render_instances(&self) -> Vec<RenderInstance> {
        let mut instances = Vec::new();
        for entity in self.state.scene.entities.values() {
            if !entity.visible {
                continue;
            }
            let Some(Component::Mesh(mesh)) = entity.components.get(&ComponentKind::Mesh) else {
                continue;
            };
            if !mesh.visible {
                continue;
            }
            let Ok(model) = self.state.scene.world_transform(entity.id) else {
                continue;
            };
            let material = match entity.components.get(&ComponentKind::Material) {
                Some(Component::Material(material)) => material.clone(),
                _ => MaterialComponent::default(),
            };
            let selected = self.state.selected == Some(entity.id);
            instances.push(RenderInstance {
                primitive: mesh.primitive,
                asset: mesh.asset.clone(),
                texture: material.base_color_texture.clone(),
                model,
                base_color: if selected {
                    [
                        material.base_color[0] * 0.7 + 0.18,
                        material.base_color[1] * 0.7 + 0.55,
                        material.base_color[2] * 0.7 + 0.58,
                        material.base_color[3],
                    ]
                } else {
                    material.base_color
                },
                metallic: material.metallic,
                roughness: material.roughness,
                emissive: if selected { 0.18 } else { 0.0 },
            });
        }
        instances
    }

    fn update_play_preview(&mut self) {
        if self.state.play_mode != PlayMode::Playing {
            return;
        }
        let delta = self.egui_context.input(|input| input.predicted_dt);
        let Some(entity_id) = self.state.scene.entities.values().find_map(|entity| {
            entity
                .components
                .contains_key(&ComponentKind::Physics)
                .then_some(entity.id)
        }) else {
            return;
        };
        if let Ok(Component::Transform(transform)) = self
            .state
            .scene
            .component_mut(entity_id, ComponentKind::Transform)
        {
            transform.translation.x += delta * 2.0;
            transform.translation.x = transform.translation.x.rem_euclid(8.0) - 4.0;
        }
    }
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
        label: Some("vela-editor-depth"),
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
