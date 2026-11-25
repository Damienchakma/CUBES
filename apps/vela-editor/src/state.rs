use anyhow::{Context, Result, bail};
use glam::{EulerRot, Quat, Vec3};
use std::path::{Path, PathBuf};
use vela_core::{EntityId, Project};
use vela_scene::{
    CameraComponent, Component, ComponentKind, LightComponent, MaterialComponent, MeshComponent,
    PhysicsComponent, PrimitiveKind, Scene, Transform,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlayMode {
    #[default]
    Editing,
    Playing,
    Paused,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TransformTool {
    #[default]
    Translate,
    Rotate,
    Scale,
}

impl TransformTool {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Translate => "Move",
            Self::Rotate => "Rotate",
            Self::Scale => "Scale",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GizmoAxis {
    X,
    Y,
    Z,
}

impl GizmoAxis {
    pub fn vector(self) -> Vec3 {
        match self {
            Self::X => Vec3::X,
            Self::Y => Vec3::Y,
            Self::Z => Vec3::Z,
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GizmoDrag {
    pub tool: TransformTool,
    pub axis: GizmoAxis,
    pub start_transform: TransformSnapshot,
    pub start_pointer: egui::Pos2,
}

#[derive(Clone, Copy, Debug)]
pub enum EditorCamera {
    Orbit { yaw: f32, pitch: f32, distance: f32 },
}

impl Default for EditorCamera {
    fn default() -> Self {
        Self::Orbit {
            yaw: 0.72,
            pitch: 0.48,
            distance: 10.0,
        }
    }
}

impl EditorCamera {
    pub fn orbit(&mut self, delta_yaw: f32, delta_pitch: f32, delta_distance: f32) {
        let Self::Orbit {
            yaw,
            pitch,
            distance,
        } = self;
        *yaw += delta_yaw;
        *pitch = (*pitch + delta_pitch).clamp(-1.45, 1.45);
        *distance = (*distance + delta_distance).clamp(0.5, 100.0);
    }

    pub fn position(&self, target: Vec3) -> Vec3 {
        match self {
            Self::Orbit {
                yaw,
                pitch,
                distance,
            } => {
                let yaw = *yaw;
                let pitch = *pitch;
                let distance = *distance;
                target
                    + Vec3::new(
                        yaw.cos() * pitch.cos() * distance,
                        pitch.sin() * distance,
                        yaw.sin() * pitch.cos() * distance,
                    )
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PerformanceStats {
    pub frames_per_second: f32,
    pub frame_time_milliseconds: f32,
    pub draw_calls: u32,
    pub triangles: u32,
    pub instances: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct TransformSnapshot {
    pub translation: [f32; 3],
    pub rotation_degrees: [f32; 3],
    pub scale: [f32; 3],
}

#[derive(Clone, Debug)]
pub struct InspectorSnapshot {
    pub id: EntityId,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub transform: TransformSnapshot,
    pub mesh: Option<MeshComponent>,
    pub material: Option<MaterialComponent>,
    pub camera: Option<CameraComponent>,
    pub light: Option<LightComponent>,
    pub physics: Option<PhysicsComponent>,
    pub component_kinds: Vec<ComponentKind>,
}

pub enum EditorModal {
    None,
    NewProject,
    OpenProject,
}

pub struct EditorState {
    pub scene: Scene,
    pub project: Option<Project>,
    pub selected: Option<EntityId>,
    pub camera: EditorCamera,
    pub camera_target: Vec3,
    pub play_mode: PlayMode,
    pub hierarchy_search: String,
    pub asset_search: String,
    pub asset_import_path: String,
    pub logs: Vec<LogEntry>,
    pub dirty: bool,
    pub dark_theme: bool,
    pub modal: EditorModal,
    pub modal_path: String,
    pub modal_name: String,
    pub show_asset_browser: bool,
    pub show_console: bool,
    pub show_profiler: bool,
    pub viewport_rect: egui::Rect,
    pub performance: PerformanceStats,
    pub graphics_backend: String,
    pub play_backup: Option<Scene>,
    pub show_welcome: bool,
    pub show_help: bool,
    pub show_templates: bool,
    pub show_about: bool,
    pub gizmo_axis: Option<GizmoAxis>,
    pub gizmo_drag: Option<GizmoDrag>,
    pub transform_tool: TransformTool,
    pub undo_stack: Vec<Scene>,
    pub redo_stack: Vec<Scene>,
}

impl Default for EditorState {
    fn default() -> Self {
        // shortcut: debug-only preselection via CUBES_SELECT; upgrade to a command flag if exposed to users
        let selected = std::env::var("CUBES_SELECT")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .map(EntityId);
        // shortcut: debug-only tool selection via CUBES_TOOL; upgrade to a command flag if exposed to users
        let transform_tool = match std::env::var("CUBES_TOOL").as_deref() {
            Ok("rotate") => TransformTool::Rotate,
            Ok("scale") => TransformTool::Scale,
            _ => TransformTool::Translate,
        };
        // shortcut: debug-only mesh assignment via CUBES_MESH_ASSET; upgrade to a command flag if exposed to users
        let mut scene = Scene::starter_scene();
        if let Ok(asset) = std::env::var("CUBES_MESH_ASSET") {
            for entity in scene.entities.values_mut() {
                if let Some(Component::Mesh(mesh)) = entity.components.get_mut(&ComponentKind::Mesh)
                    && mesh.primitive == PrimitiveKind::Cube
                {
                    mesh.asset = Some(asset.clone());
                }
            }
        }
        Self {
            scene,
            project: None,
            selected,
            camera: EditorCamera::Orbit {
                yaw: 0.72,
                pitch: 0.48,
                distance: 10.0,
            },
            camera_target: Vec3::ZERO,
            play_mode: PlayMode::Editing,
            hierarchy_search: String::new(),
            asset_search: String::new(),
            asset_import_path: String::new(),
            logs: Vec::new(),
            dirty: false,
            dark_theme: true,
            modal: EditorModal::None,
            modal_path: String::new(),
            modal_name: "Untitled Game".to_owned(),
            show_asset_browser: true,
            show_console: false,
            show_profiler: false,
            viewport_rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 720.0)),
            performance: PerformanceStats::default(),
            graphics_backend: String::new(),
            play_backup: None,
            show_welcome: !welcome_marker_exists(),
            show_help: false,
            show_templates: false,
            show_about: false,
            gizmo_axis: None,
            gizmo_drag: None,
            transform_tool,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }
}

impl EditorState {
    pub fn log(&mut self, level: LogLevel, message: impl Into<String>) {
        let message = message.into();
        if self.logs.len() >= 500 {
            self.logs.remove(0);
        }
        self.logs.push(LogEntry { level, message });
    }

    pub fn checkpoint(&mut self) {
        self.undo_stack.push(self.scene.clone());
        if self.undo_stack.len() > 64 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo(&mut self) -> bool {
        let Some(scene) = self.undo_stack.pop() else {
            return false;
        };
        self.redo_stack.push(self.scene.clone());
        self.restore_scene(scene);
        self.log(LogLevel::Info, "Undo");
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(scene) = self.redo_stack.pop() else {
            return false;
        };
        self.undo_stack.push(self.scene.clone());
        self.restore_scene(scene);
        self.log(LogLevel::Info, "Redo");
        true
    }

    fn restore_scene(&mut self, scene: Scene) {
        self.scene = scene;
        if self
            .selected
            .is_some_and(|id| !self.scene.entities.contains_key(&id))
        {
            self.selected = None;
        }
        self.dirty = true;
    }

    pub fn set_play_mode(&mut self, mode: PlayMode) {
        match mode {
            PlayMode::Editing => {
                if let Some(scene) = self.play_backup.take() {
                    self.scene = scene;
                }
                self.play_mode = PlayMode::Editing;
            }
            PlayMode::Playing => {
                if self.play_mode == PlayMode::Editing {
                    self.play_backup = Some(self.scene.clone());
                }
                self.play_mode = PlayMode::Playing;
            }
            PlayMode::Paused => self.play_mode = PlayMode::Paused,
        }
    }

    pub fn dismiss_welcome(&mut self) {
        self.show_welcome = false;
        mark_welcome_seen();
    }

    pub fn start_starter_template(&mut self) {
        self.dismiss_welcome();
        self.show_templates = false;
        self.modal = EditorModal::NewProject;
        self.modal_name = "CUBES Game".to_owned();
    }

    pub fn pick_entity(&mut self, pointer: egui::Pos2, viewport: egui::Rect) -> Option<EntityId> {
        if viewport.width() <= 0.0 || viewport.height() <= 0.0 {
            return None;
        }
        let ndc_x = (pointer.x - viewport.min.x) / viewport.width() * 2.0 - 1.0;
        let ndc_y = 1.0 - (pointer.y - viewport.min.y) / viewport.height() * 2.0;
        let origin = self.camera.position(self.camera_target);
        let forward = (self.camera_target - origin).normalize_or_zero();
        let right = forward.cross(Vec3::Y).normalize_or_zero();
        let up = right.cross(forward).normalize_or_zero();
        let aspect = viewport.width() / viewport.height();
        let tan_half_fov = (60.0_f32.to_radians() * 0.5).tan();
        let direction =
            (forward + right * ndc_x * aspect * tan_half_fov + up * ndc_y * tan_half_fov)
                .normalize_or_zero();

        let mut nearest: Option<(f32, EntityId)> = None;
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
            let Ok(world) = self.scene.world_transform(entity.id) else {
                continue;
            };
            let inverse = world.inverse();
            let local_origin = inverse.transform_point3(origin);
            let local_end = inverse.transform_point3(origin + direction);
            let local_direction = (local_end - local_origin).normalize_or_zero();
            let local_distance = if mesh.primitive == PrimitiveKind::Plane {
                ray_plane(local_origin, local_direction)
            } else {
                ray_box(local_origin, local_direction)
            };
            if let Some(distance) = local_distance {
                let hit = world.transform_point3(local_origin + local_direction * distance);
                let world_distance = (hit - origin).length();
                if nearest.is_none_or(|(current, _)| world_distance < current) {
                    nearest = Some((world_distance, entity.id));
                }
            }
        }
        self.selected = nearest.map(|(_, id)| id);
        self.selected
    }

    pub fn gizmo_origin(&self) -> Option<Vec3> {
        let id = self.selected?;
        self.scene
            .world_transform(id)
            .ok()
            .map(|transform| transform.w_axis.truncate())
    }

    pub fn selected_transform_snapshot(&self) -> Option<TransformSnapshot> {
        let id = self.selected?;
        let entity = self.scene.entities.get(&id)?;
        let transform = match entity.components.get(&ComponentKind::Transform) {
            Some(Component::Transform(transform)) => *transform,
            _ => Transform::default(),
        };
        let (rx, ry, rz) = transform.rotation.to_euler(EulerRot::XYZ);
        Some(TransformSnapshot {
            translation: translation_array(transform.translation),
            rotation_degrees: [rx.to_degrees(), ry.to_degrees(), rz.to_degrees()],
            scale: scale_array(transform.scale),
        })
    }

    pub fn apply_gizmo_drag(&mut self, drag: GizmoDrag, axis_delta: f32, rotation_delta: f32) {
        let mut snapshot = drag.start_transform;
        match drag.tool {
            TransformTool::Translate => {
                snapshot.translation[drag.axis.index()] += axis_delta;
            }
            TransformTool::Scale => {
                let index = drag.axis.index();
                snapshot.scale[index] = (snapshot.scale[index] + axis_delta).max(0.01);
            }
            TransformTool::Rotate => {
                snapshot.rotation_degrees[drag.axis.index()] += rotation_delta.to_degrees();
            }
        }
        self.apply_transform(snapshot);
    }

    pub fn open_project(&mut self, path: &Path) -> Result<()> {
        let project = Project::open(path).context("project could not be opened")?;
        let scene_path = project
            .paths()
            .resolve_existing(&project.manifest().main_scene)
            .context("main scene is missing")?;
        let scene = Scene::load(scene_path).context("main scene could not be loaded")?;
        self.project = Some(project);
        self.scene = scene;
        self.selected = None;
        self.dirty = false;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.log(LogLevel::Info, "Project opened");
        Ok(())
    }

    pub fn create_project(&mut self, path: &Path, name: &str) -> Result<()> {
        let project = Project::create(path, name).context("project creation failed")?;
        let scene_path = project
            .paths()
            .resolve_for_write(&project.manifest().main_scene)
            .context("main scene path is outside the project")?;
        Scene::starter_scene().save(&scene_path)?;
        self.project = Some(project);
        self.scene = Scene::starter_scene();
        self.selected = None;
        self.dirty = false;
        self.modal = EditorModal::None;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.log(LogLevel::Info, format!("Created project '{name}'"));
        Ok(())
    }

    pub fn save_scene(&mut self) -> Result<()> {
        if self.play_backup.is_some() {
            self.log(
                LogLevel::Warning,
                "Save skipped: stop play mode before saving",
            );
            return Ok(());
        }
        let Some(project) = &self.project else {
            self.log(LogLevel::Warning, "Save skipped: no project is open");
            return Ok(());
        };
        let path = project
            .paths()
            .resolve_for_write(&project.manifest().main_scene)
            .context("main scene path is outside the project")?;
        self.scene.save(&path)?;
        self.dirty = false;
        self.log(LogLevel::Info, "Scene saved");
        Ok(())
    }

    pub fn add_entity(&mut self, kind: NewEntityKind) -> EntityId {
        self.checkpoint();
        let id = match kind {
            NewEntityKind::Empty => self.scene.create_entity("Empty"),
            NewEntityKind::Cube => self.scene.create_entity("Cube"),
            NewEntityKind::Plane => self.scene.create_entity("Plane"),
            NewEntityKind::Camera => self.scene.create_entity("Camera"),
            NewEntityKind::Light => self.scene.create_entity("Light"),
        };
        match kind {
            NewEntityKind::Cube => {
                let _ = self.scene.add_component(
                    id,
                    Component::Mesh(MeshComponent {
                        primitive: PrimitiveKind::Cube,
                        ..MeshComponent::default()
                    }),
                );
                let _ = self
                    .scene
                    .add_component(id, Component::Material(MaterialComponent::default()));
            }
            NewEntityKind::Plane => {
                let _ = self.scene.add_component(
                    id,
                    Component::Mesh(MeshComponent {
                        primitive: PrimitiveKind::Plane,
                        ..MeshComponent::default()
                    }),
                );
                let _ = self
                    .scene
                    .add_component(id, Component::Material(MaterialComponent::default()));
            }
            NewEntityKind::Camera => {
                let _ = self
                    .scene
                    .add_component(id, Component::Camera(CameraComponent::default()));
            }
            NewEntityKind::Light => {
                let _ = self
                    .scene
                    .add_component(id, Component::Light(LightComponent::default()));
            }
            NewEntityKind::Empty => {}
        }
        self.selected = Some(id);
        self.dirty = true;
        self.log(LogLevel::Info, format!("Created entity {id}"));
        id
    }

    pub fn delete_selected(&mut self) {
        let Some(id) = self.selected else {
            return;
        };
        self.checkpoint();
        if self.scene.delete_entity(id).is_ok() {
            self.selected = None;
            self.dirty = true;
            self.log(LogLevel::Info, format!("Deleted entity {id}"));
        }
    }

    pub fn duplicate_selected(&mut self) {
        let Some(source) = self.selected.and_then(|id| {
            self.scene.entities.get(&id).map(|entity| {
                (
                    entity.name.clone(),
                    entity.parent,
                    entity.components.clone(),
                )
            })
        }) else {
            return;
        };
        self.checkpoint();
        let name = format!("{} Copy", source.0);
        let components = source.2;
        let id = self.scene.create_entity(name);
        let _ = self.scene.set_parent(id, source.1);
        if let Ok(entity) = self.scene.entity_mut(id) {
            entity.components = components;
        }
        self.selected = Some(id);
        self.dirty = true;
        self.log(LogLevel::Info, format!("Duplicated entity as {id}"));
    }

    pub fn focus_selected(&mut self) {
        if let Some(id) = self.selected
            && let Ok(transform) = self.scene.world_transform(id)
        {
            self.camera_target = transform.w_axis.truncate();
            self.log(LogLevel::Info, "Focused selected entity");
        }
    }

    pub fn inspector_snapshot(&self) -> Option<InspectorSnapshot> {
        let id = self.selected?;
        let entity = self.scene.entities.get(&id)?;
        let transform = match entity.components.get(&ComponentKind::Transform) {
            Some(Component::Transform(transform)) => *transform,
            _ => Transform::default(),
        };
        let (rx, ry, rz) = transform.rotation.to_euler(EulerRot::XYZ);
        let component = |kind| entity.components.get(&kind).cloned();
        let mesh = match component(ComponentKind::Mesh) {
            Some(Component::Mesh(mesh)) => Some(mesh),
            _ => None,
        };
        let material = match component(ComponentKind::Material) {
            Some(Component::Material(material)) => Some(material),
            _ => None,
        };
        let camera = match component(ComponentKind::Camera) {
            Some(Component::Camera(camera)) => Some(camera),
            _ => None,
        };
        let light = match component(ComponentKind::Light) {
            Some(Component::Light(light)) => Some(light),
            _ => None,
        };
        let physics = match component(ComponentKind::Physics) {
            Some(Component::Physics(physics)) => Some(physics),
            _ => None,
        };
        Some(InspectorSnapshot {
            id,
            name: entity.name.clone(),
            visible: entity.visible,
            locked: entity.locked,
            transform: TransformSnapshot {
                translation: translation_array(transform.translation),
                rotation_degrees: [rx.to_degrees(), ry.to_degrees(), rz.to_degrees()],
                scale: scale_array(transform.scale),
            },
            mesh,
            material,
            camera,
            light,
            physics,
            component_kinds: entity.components.keys().copied().collect(),
        })
    }

    pub fn apply_transform(&mut self, snapshot: TransformSnapshot) {
        let Some(id) = self.selected else {
            return;
        };
        let rotation = Quat::from_euler(
            EulerRot::XYZ,
            snapshot.rotation_degrees[0].to_radians(),
            snapshot.rotation_degrees[1].to_radians(),
            snapshot.rotation_degrees[2].to_radians(),
        );
        let transform = Transform {
            translation: Vec3::from_array(snapshot.translation),
            rotation,
            scale: Vec3::from_array(snapshot.scale),
        };
        if let Ok(Component::Transform(current)) =
            self.scene.component_mut(id, ComponentKind::Transform)
        {
            *current = transform;
            self.dirty = true;
        }
    }

    pub fn set_entity_name(&mut self, name: String) {
        let Some(id) = self.selected else {
            return;
        };
        if self
            .scene
            .entity(id)
            .is_ok_and(|entity| entity.name == name)
        {
            return;
        }
        self.checkpoint();
        if let Ok(entity) = self.scene.entity_mut(id) {
            entity.name = name;
            self.dirty = true;
        }
    }

    pub fn set_entity_flags(&mut self, visible: bool, locked: bool) {
        self.checkpoint();
        if let Some(id) = self.selected
            && let Ok(entity) = self.scene.entity_mut(id)
            && (entity.visible != visible || entity.locked != locked)
        {
            entity.visible = visible;
            entity.locked = locked;
            self.dirty = true;
        }
    }

    pub fn set_mesh(&mut self, mesh: MeshComponent) {
        self.checkpoint();
        if let Some(id) = self.selected
            && let Ok(Component::Mesh(current)) = self.scene.component_mut(id, ComponentKind::Mesh)
            && *current != mesh
        {
            *current = mesh;
            self.dirty = true;
        }
    }

    pub fn set_imported_material(&mut self, id: EntityId, base_color: [f32; 4], texture: String) {
        let mut changed = self
            .scene
            .entity(id)
            .is_ok_and(|entity| !entity.components.contains_key(&ComponentKind::Material));
        if !changed
            && let Ok(Component::Material(material)) =
                self.scene.component(id, ComponentKind::Material)
        {
            changed = material.base_color != base_color
                || material.base_color_texture.as_deref() != Some(texture.as_str());
        }
        if !changed {
            return;
        }
        self.checkpoint();
        if self
            .scene
            .entity(id)
            .is_ok_and(|entity| !entity.components.contains_key(&ComponentKind::Material))
        {
            let _ = self.scene.add_component(
                id,
                Component::Material(MaterialComponent {
                    base_color,
                    base_color_texture: Some(texture),
                    ..MaterialComponent::default()
                }),
            );
        } else if let Ok(Component::Material(material)) =
            self.scene.component_mut(id, ComponentKind::Material)
        {
            material.base_color = base_color;
            material.base_color_texture = Some(texture);
        }
        self.dirty = true;
    }

    pub fn set_material(&mut self, material: MaterialComponent) {
        self.checkpoint();
        if let Some(id) = self.selected
            && let Ok(Component::Material(current)) =
                self.scene.component_mut(id, ComponentKind::Material)
        {
            *current = material;
            self.dirty = true;
        }
    }

    pub fn set_light(&mut self, light: LightComponent) {
        self.checkpoint();
        if let Some(id) = self.selected
            && let Ok(Component::Light(current)) =
                self.scene.component_mut(id, ComponentKind::Light)
        {
            *current = light;
            self.dirty = true;
        }
    }

    pub fn set_camera(&mut self, camera: CameraComponent) {
        self.checkpoint();
        if let Some(id) = self.selected
            && let Ok(Component::Camera(current)) =
                self.scene.component_mut(id, ComponentKind::Camera)
        {
            *current = camera;
            self.dirty = true;
        }
    }

    pub fn set_physics(&mut self, physics: PhysicsComponent) {
        self.checkpoint();
        if let Some(id) = self.selected
            && let Ok(Component::Physics(current)) =
                self.scene.component_mut(id, ComponentKind::Physics)
        {
            *current = physics;
            self.dirty = true;
        }
    }

    pub fn add_component(&mut self, kind: ComponentKind) {
        self.checkpoint();
        let Some(id) = self.selected else {
            return;
        };
        let component = match kind {
            ComponentKind::Material => Component::Material(MaterialComponent::default()),
            ComponentKind::Camera => Component::Camera(CameraComponent::default()),
            ComponentKind::Light => Component::Light(LightComponent::default()),
            ComponentKind::Physics => Component::Physics(PhysicsComponent::default()),
            ComponentKind::Mesh => Component::Mesh(MeshComponent::default()),
            ComponentKind::Script => Component::Script(vela_scene::ScriptComponent {
                path: "scripts/main.lua".to_owned(),
                enabled: true,
            }),
            ComponentKind::AudioSource => {
                Component::AudioSource(vela_scene::AudioSourceComponent::default())
            }
            ComponentKind::Transform => return,
        };
        if self.scene.add_component(id, component).is_ok() {
            self.dirty = true;
            self.log(LogLevel::Info, format!("Added {} component", kind.as_str()));
        }
    }

    pub fn remove_component(&mut self, kind: ComponentKind) {
        self.checkpoint();
        let Some(id) = self.selected else {
            return;
        };
        if self.scene.remove_component(id, kind).is_ok() {
            self.dirty = true;
        }
    }

    pub fn set_selected_mesh_asset(&mut self, asset: String) {
        let Some(id) = self.selected else {
            return;
        };
        let changed = self
            .scene
            .component(id, ComponentKind::Mesh)
            .ok()
            .is_some_and(|component| match component {
                Component::Mesh(mesh) => mesh.asset.as_deref() != Some(asset.as_str()),
                _ => false,
            });
        if !changed {
            return;
        }
        self.checkpoint();
        if let Ok(Component::Mesh(mesh)) = self.scene.component_mut(id, ComponentKind::Mesh) {
            mesh.asset = Some(asset);
            self.dirty = true;
        }
    }

    pub fn import_asset(&mut self, source: &Path) -> Result<vela_assets::ImportedAsset> {
        let Some(project) = &self.project else {
            bail!("open a project before importing assets");
        };
        let asset =
            vela_assets::import_asset_into(source, &project.paths().assets_dir().to_path_buf())?;
        let name = source
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "asset".to_owned());
        if asset.kind == vela_assets::AssetKind::Model {
            self.set_selected_mesh_asset(format!("assets/{name}"));
            if self
                .selected
                .and_then(|id| self.scene.component(id, ComponentKind::Mesh).ok())
                .is_some_and(|component| {
                    matches!(component, Component::Mesh(mesh) if mesh.asset.as_deref() == Some(format!("assets/{name}").as_str()))
                })
            {
                self.log(LogLevel::Info, "Assigned imported mesh to selected object");
            }
        }
        self.log(
            LogLevel::Info,
            format!("Imported {:?} asset ({})", asset.kind, asset.checksum),
        );
        Ok(asset)
    }

    pub fn asset_files(&self) -> Vec<PathBuf> {
        let Some(project) = &self.project else {
            return Vec::new();
        };
        let mut files = Vec::new();
        collect_files(&project.paths().assets_dir(), &mut files, 0);
        files.sort();
        files
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NewEntityKind {
    Empty,
    Cube,
    Plane,
    Camera,
    Light,
}

fn ray_box(origin: Vec3, direction: Vec3) -> Option<f32> {
    let origin = origin.to_array();
    let direction = direction.to_array();
    let mut near = f32::NEG_INFINITY;
    let mut far = f32::INFINITY;
    for axis in 0..3 {
        if direction[axis].abs() < f32::EPSILON {
            if origin[axis].abs() > 1.0 {
                return None;
            }
            continue;
        }
        let first = (-1.0 - origin[axis]) / direction[axis];
        let second = (1.0 - origin[axis]) / direction[axis];
        near = near.max(first.min(second));
        far = far.min(first.max(second));
        if near > far {
            return None;
        }
    }
    Some(if near >= 0.0 { near } else { far.max(0.0) })
}

fn ray_plane(origin: Vec3, direction: Vec3) -> Option<f32> {
    if direction.y.abs() < f32::EPSILON {
        return None;
    }
    let distance = -origin.y / direction.y;
    if distance < 0.0 {
        return None;
    }
    let point = origin + direction * distance;
    (point.x.abs() <= 1.0 && point.z.abs() <= 1.0).then_some(distance)
}

fn welcome_marker_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("cubes").join("welcome-seen"))
}

fn welcome_marker_exists() -> bool {
    welcome_marker_path().is_some_and(|path| path.exists())
}

fn mark_welcome_seen() {
    if let Some(path) = welcome_marker_path()
        && let Some(parent) = path.parent()
    {
        let _ = std::fs::create_dir_all(parent);
        let _ = std::fs::write(path, b"seen");
    }
}

fn collect_files(path: &Path, output: &mut Vec<PathBuf>, depth: usize) {
    if depth > 8 || output.len() >= 2_000 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, output, depth + 1);
        } else {
            output.push(path);
        }
    }
}

fn translation_array(value: Vec3) -> [f32; 3] {
    [value.x, value.y, value.z]
}

fn scale_array(value: Vec3) -> [f32; 3] {
    [value.x, value.y, value.z]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stopping_play_restores_the_edited_scene() {
        let mut state = EditorState::default();
        let id = EntityId(1);
        state.scene.entity_mut(id).unwrap().name = "Before play".to_owned();

        state.set_play_mode(PlayMode::Playing);
        state.scene.entity_mut(id).unwrap().name = "Preview".to_owned();
        state.set_play_mode(PlayMode::Editing);

        assert_eq!(state.scene.entity(id).unwrap().name, "Before play");
        assert!(state.play_backup.is_none());
    }

    #[test]
    fn dragging_down_moves_orbit_camera_above_the_target() {
        let mut camera = EditorCamera::default();
        let before = camera.position(Vec3::ZERO).y;
        camera.orbit(0.0, 0.1, 0.0);
        assert!(camera.position(Vec3::ZERO).y > before);
    }

    #[test]
    fn clicking_viewport_center_selects_an_object() {
        let mut state = EditorState::default();
        let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        assert!(state.pick_entity(viewport.center(), viewport).is_some());
    }

    fn selected_cube(state: &EditorState) -> EntityId {
        state
            .scene
            .entities
            .values()
            .find(|entity| {
                entity
                    .components
                    .get(&ComponentKind::Mesh)
                    .is_some_and(|component| {
                        matches!(component, Component::Mesh(mesh) if mesh.primitive == PrimitiveKind::Cube)
                    })
            })
            .map(|entity| entity.id)
            .unwrap()
    }

    #[test]
    fn gizmo_translation_moves_the_selected_object() {
        let mut state = EditorState::default();
        state.selected = Some(selected_cube(&state));
        let start = state.selected_transform_snapshot().unwrap();
        let drag = GizmoDrag {
            tool: TransformTool::Translate,
            axis: GizmoAxis::X,
            start_transform: start,
            start_pointer: egui::Pos2::ZERO,
        };
        state.apply_gizmo_drag(drag, 0.5, 0.0);
        assert_eq!(
            state.selected_transform_snapshot().unwrap().translation[0],
            start.translation[0] + 0.5
        );
    }

    #[test]
    fn gizmo_scale_and_rotate_change_the_selected_object() {
        let mut state = EditorState::default();
        state.selected = Some(selected_cube(&state));
        let start = state.selected_transform_snapshot().unwrap();
        state.apply_gizmo_drag(
            GizmoDrag {
                tool: TransformTool::Scale,
                axis: GizmoAxis::X,
                start_transform: start,
                start_pointer: egui::Pos2::ZERO,
            },
            0.5,
            0.0,
        );
        assert_eq!(
            state.selected_transform_snapshot().unwrap().scale[0],
            start.scale[0] + 0.5
        );

        state.apply_gizmo_drag(
            GizmoDrag {
                tool: TransformTool::Rotate,
                axis: GizmoAxis::Y,
                start_transform: start,
                start_pointer: egui::Pos2::ZERO,
            },
            0.0,
            std::f32::consts::FRAC_PI_2,
        );
        assert!(
            (state
                .selected_transform_snapshot()
                .unwrap()
                .rotation_degrees[1]
                - 90.0)
                .abs()
                < 0.01
        );
    }

    #[test]
    fn undo_and_redo_restore_scene_operations() {
        let mut state = EditorState::default();
        let before = state.scene.entities.len();
        state.checkpoint();
        state.add_entity(NewEntityKind::Cube);
        assert_eq!(state.scene.entities.len(), before + 1);
        assert!(state.undo());
        assert_eq!(state.scene.entities.len(), before);
        assert!(state.redo());
        assert_eq!(state.scene.entities.len(), before + 1);
    }
}
