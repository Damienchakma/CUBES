use crate::state::{
    EditorModal, EditorState, GizmoAxis, GizmoDrag, LogLevel, NewEntityKind, PlayMode,
    TransformSnapshot, TransformTool,
};
use egui::{
    Align, Align2, Color32, Context, Frame, PointerButton, Pos2, Rect, RichText, Stroke, Theme, Ui,
    vec2,
};
use glam::{Vec3, Vec4Swizzles};
use vela_renderer::{Camera, CameraProjection};
use vela_scene::{ComponentKind, LightType, PrimitiveKind};

const ACCENT: Color32 = Color32::from_rgb(70, 202, 214);
const ACCENT_DARK: Color32 = Color32::from_rgb(38, 112, 124);
const WARNING: Color32 = Color32::from_rgb(245, 177, 72);
const ERROR: Color32 = Color32::from_rgb(244, 91, 103);

pub fn show(root: &mut Ui, state: &mut EditorState) {
    let context = root.ctx().clone();
    apply_theme(&context, state.dark_theme);
    top_bar(root, state);
    hierarchy_panel(root, state);
    inspector_panel(root, state);
    bottom_panel(root, state);
    viewport_panel(root, state);
    project_modal(&context, state);
    welcome_window(&context, state);
    template_window(&context, state);
    help_window(&context, state);
    about_window(&context, state);
}

fn apply_theme(context: &Context, dark: bool) {
    let theme = if dark { Theme::Dark } else { Theme::Light };
    let mut style = (*context.style_of(theme)).clone();
    if dark {
        style.visuals.panel_fill = Color32::from_rgb(18, 23, 32);
        style.visuals.window_fill = Color32::from_rgb(25, 31, 42);
        style.visuals.extreme_bg_color = Color32::from_rgb(12, 16, 23);
        style.visuals.faint_bg_color = Color32::from_rgb(27, 34, 46);
        style.visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(28, 36, 48);
        style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(38, 48, 62);
        style.visuals.widgets.hovered.bg_fill = ACCENT_DARK;
        style.visuals.widgets.active.bg_fill = ACCENT_DARK;
        style.visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(70, 202, 214, 70);
        style.visuals.hyperlink_color = ACCENT;
    } else {
        style.visuals.panel_fill = Color32::from_rgb(237, 241, 246);
        style.visuals.window_fill = Color32::from_rgb(250, 252, 255);
        style.visuals.extreme_bg_color = Color32::from_rgb(220, 226, 234);
        style.visuals.faint_bg_color = Color32::from_rgb(245, 247, 250);
        style.visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(225, 231, 239);
        style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(211, 219, 229);
        style.visuals.widgets.hovered.bg_fill = ACCENT;
        style.visuals.widgets.active.bg_fill = ACCENT;
        style.visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(24, 125, 145, 55);
        style.visuals.hyperlink_color = Color32::from_rgb(20, 115, 135);
    }
    style.visuals.widgets.noninteractive.bg_stroke =
        Stroke::new(1.0, style.visuals.widgets.noninteractive.fg_stroke.color);
    context.set_style_of(theme, style);
    context.set_theme(theme);
}

fn top_bar(root: &mut Ui, state: &mut EditorState) {
    egui::Panel::top("top_bar")
        .exact_size(42.0)
        .frame(Frame::new().inner_margin(8.0))
        .show(root, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New project...").clicked() {
                        state.modal = EditorModal::NewProject;
                        ui.close();
                    }
                    if ui.button("Open project...").clicked() {
                        state.modal = EditorModal::OpenProject;
                        ui.close();
                    }
                    if ui.button("Save scene").clicked() {
                        if let Err(error) = state.save_scene() {
                            state.log(LogLevel::Error, error.to_string());
                        }
                    }
                    ui.separator();
                    if ui.button("Close editor").clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Edit", |ui| {
                    if ui
                        .add_enabled(state.can_undo(), egui::Button::new("Undo"))
                        .clicked()
                    {
                        state.undo();
                        ui.close();
                    }
                    if ui
                        .add_enabled(state.can_redo(), egui::Button::new("Redo"))
                        .clicked()
                    {
                        state.redo();
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Duplicate").clicked() {
                        state.duplicate_selected();
                        ui.close();
                    }
                    if ui.button("Delete").clicked() {
                        state.delete_selected();
                        ui.close();
                    }
                    if ui.button("Focus selected").clicked() {
                        state.focus_selected();
                        ui.close();
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("Welcome to CUBES").clicked() {
                        state.show_welcome = true;
                        ui.close();
                    }
                    if ui.button("Quick start").clicked() {
                        state.show_help = true;
                        ui.close();
                    }
                    if ui.button("About CUBES").clicked() {
                        state.show_about = true;
                        ui.close();
                    }
                });

                ui.separator();
                let project_name = state
                    .project
                    .as_ref()
                    .map(|project| project.manifest().name.as_str())
                    .unwrap_or("Untitled Scene");
                ui.label(RichText::new(project_name).strong());
                if state.dirty {
                    ui.label("*");
                }

                ui.separator();
                for tool in [
                    TransformTool::Translate,
                    TransformTool::Rotate,
                    TransformTool::Scale,
                ] {
                    if ui
                        .selectable_label(state.transform_tool == tool, tool.as_str())
                        .clicked()
                    {
                        state.transform_tool = tool;
                    }
                }
                ui.separator();
                if ui
                    .add_enabled(state.can_undo(), egui::Button::new("Undo"))
                    .clicked()
                {
                    state.undo();
                }
                if ui
                    .add_enabled(state.can_redo(), egui::Button::new("Redo"))
                    .clicked()
                {
                    state.redo();
                }

                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Stop").on_hover_text("Stop play mode").clicked() {
                        state.set_play_mode(PlayMode::Editing);
                    }
                    let play_label = match state.play_mode {
                        PlayMode::Playing => "Pause",
                        PlayMode::Paused | PlayMode::Editing => "Play",
                    };
                    if ui
                        .button(play_label)
                        .on_hover_text("Play / resume")
                        .clicked()
                    {
                        state.set_play_mode(match state.play_mode {
                            PlayMode::Playing => PlayMode::Paused,
                            _ => PlayMode::Playing,
                        });
                    }
                    if ui
                        .button(if state.dark_theme { "Light" } else { "Dark" })
                        .on_hover_text("Toggle editor theme")
                        .clicked()
                    {
                        state.dark_theme = !state.dark_theme;
                    }
                });
            });
        });
}

fn hierarchy_panel(root: &mut Ui, state: &mut EditorState) {
    egui::Panel::left("hierarchy")
        .resizable(true)
        .default_size(270.0)
        .frame(Frame::new())
        .show(root, |ui| {
            ui.strong("Scene");
            ui.horizontal(|ui| {
                ui.label("Search");
                ui.text_edit_singleline(&mut state.hierarchy_search);
            });
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Add");
                egui::ComboBox::from_id_salt("add-entity")
                    .selected_text("Object")
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(false, "Empty object").clicked() {
                            state.add_entity(NewEntityKind::Empty);
                        }
                        if ui.selectable_label(false, "Cube").clicked() {
                            state.add_entity(NewEntityKind::Cube);
                        }
                        if ui.selectable_label(false, "Plane").clicked() {
                            state.add_entity(NewEntityKind::Plane);
                        }
                        if ui.selectable_label(false, "Camera").clicked() {
                            state.add_entity(NewEntityKind::Camera);
                        }
                        if ui.selectable_label(false, "Light").clicked() {
                            state.add_entity(NewEntityKind::Light);
                        }
                    });
            });
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                let rows = hierarchy_rows(state);
                if rows.is_empty() {
                    ui.label("No matching objects");
                }
                for row in rows {
                    ui.horizontal(|ui| {
                        ui.add_space(row.depth as f32 * 14.0);
                        let selected = state.selected == Some(row.id);
                        if ui
                            .selectable_label(selected, row.name)
                            .on_hover_text(format!("Entity {}", row.id))
                            .clicked()
                        {
                            state.selected = Some(row.id);
                        }
                        if ui
                            .small_button(if row.visible { "V" } else { "-" })
                            .clicked()
                        {
                            if let Some(entity) = state.scene.entities.get_mut(&row.id) {
                                entity.visible = !entity.visible;
                                state.dirty = true;
                            }
                        }
                        if ui
                            .small_button(if row.locked { "L" } else { "-" })
                            .clicked()
                        {
                            if let Some(entity) = state.scene.entities.get_mut(&row.id) {
                                entity.locked = !entity.locked;
                                state.dirty = true;
                            }
                        }
                    });
                }
            });

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Duplicate").clicked() {
                    state.duplicate_selected();
                }
                if ui.button("Delete").clicked() {
                    state.delete_selected();
                }
                if ui.button("Focus").clicked() {
                    state.focus_selected();
                }
            });
        });
}

struct HierarchyRow {
    id: vela_core::EntityId,
    name: String,
    depth: usize,
    visible: bool,
    locked: bool,
}

fn hierarchy_rows(state: &EditorState) -> Vec<HierarchyRow> {
    let mut rows = Vec::new();
    let query = state.hierarchy_search.trim().to_ascii_lowercase();
    collect_hierarchy(&state.scene, None, 0, &query, &mut rows);
    rows
}

fn collect_hierarchy(
    scene: &vela_scene::Scene,
    parent: Option<vela_core::EntityId>,
    depth: usize,
    query: &str,
    output: &mut Vec<HierarchyRow>,
) {
    for entity in scene.entities.values() {
        if entity.parent != parent {
            continue;
        }
        let matches = query.is_empty() || entity.name.to_ascii_lowercase().contains(query);
        if matches {
            output.push(HierarchyRow {
                id: entity.id,
                name: entity.name.clone(),
                depth,
                visible: entity.visible,
                locked: entity.locked,
            });
        }
        collect_hierarchy(scene, Some(entity.id), depth + 1, query, output);
    }
}

fn inspector_panel(root: &mut Ui, state: &mut EditorState) {
    egui::Panel::right("inspector")
        .resizable(true)
        .default_size(320.0)
        .frame(Frame::new())
        .show(root, |ui| {
            ui.strong("Inspector");
            ui.separator();
            let Some(mut snapshot) = state.inspector_snapshot() else {
                ui.centered_and_justified(|ui| {
                    ui.label("Select an object to inspect it");
                });
                return;
            };

            ui.label("Entity");
            let mut flags_changed = false;
            ui.horizontal(|ui| {
                ui.label(RichText::new(snapshot.id.to_string()).weak());
                flags_changed |= ui.checkbox(&mut snapshot.visible, "Visible").changed();
                flags_changed |= ui.checkbox(&mut snapshot.locked, "Locked").changed();
            });
            if flags_changed {
                state.set_entity_flags(snapshot.visible, snapshot.locked);
            }
            if ui.text_edit_singleline(&mut snapshot.name).changed() {
                state.set_entity_name(snapshot.name.clone());
            }

            ui.separator();
            ui.label("Transform");
            let mut transform_changed = false;
            let mut transform_drag_started = false;
            let (changed, drag_started) =
                vector3_editor(ui, "Position", &mut snapshot.transform.translation);
            transform_changed |= changed;
            transform_drag_started |= drag_started;
            let (changed, drag_started) =
                vector3_editor(ui, "Rotation °", &mut snapshot.transform.rotation_degrees);
            transform_changed |= changed;
            transform_drag_started |= drag_started;
            let (changed, drag_started) =
                vector3_editor(ui, "Scale", &mut snapshot.transform.scale);
            transform_changed |= changed;
            transform_drag_started |= drag_started;
            if transform_drag_started {
                state.checkpoint();
            }
            if transform_changed {
                state.apply_transform(snapshot.transform);
            }

            if let Some(mut mesh) = snapshot.mesh.clone() {
                ui.separator();
                ui.label("Mesh");
                let mut mesh_changed = false;
                egui::ComboBox::from_id_salt("mesh-primitive")
                    .selected_text(format!("{:?}", mesh.primitive))
                    .show_ui(ui, |ui| {
                        for primitive in [
                            PrimitiveKind::Cube,
                            PrimitiveKind::Sphere,
                            PrimitiveKind::Plane,
                            PrimitiveKind::Cylinder,
                        ] {
                            if ui
                                .selectable_label(
                                    mesh.primitive == primitive,
                                    format!("{primitive:?}"),
                                )
                                .clicked()
                            {
                                mesh.primitive = primitive;
                                mesh_changed = true;
                            }
                        }
                    });
                ui.horizontal(|ui| {
                    ui.label("Asset");
                    let asset = mesh.asset.get_or_insert_with(String::new);
                    mesh_changed |= ui.text_edit_singleline(asset).changed();
                });
                if mesh.asset.as_deref() == Some("") {
                    mesh.asset = None;
                }
                if mesh_changed {
                    state.set_mesh(mesh);
                }
            }

            if let Some(mut material) = snapshot.material {
                ui.separator();
                ui.label("Material");
                ui.horizontal(|ui| {
                    ui.label("Base");
                    let mut color = Color32::from_rgba_unmultiplied(
                        (material.base_color[0] * 255.0) as u8,
                        (material.base_color[1] * 255.0) as u8,
                        (material.base_color[2] * 255.0) as u8,
                        (material.base_color[3] * 255.0) as u8,
                    );
                    if ui.color_edit_button_srgba(&mut color).changed() {
                        let color = color.to_srgba_unmultiplied();
                        material.base_color = [
                            color[0] as f32 / 255.0,
                            color[1] as f32 / 255.0,
                            color[2] as f32 / 255.0,
                            color[3] as f32 / 255.0,
                        ];
                        state.set_material(material.clone());
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Metallic");
                    if ui
                        .add(egui::Slider::new(&mut material.metallic, 0.0..=1.0).show_value(true))
                        .changed()
                    {
                        state.set_material(material.clone());
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Roughness");
                    if ui
                        .add(
                            egui::Slider::new(&mut material.roughness, 0.04..=1.0).show_value(true),
                        )
                        .changed()
                    {
                        state.set_material(material.clone());
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Texture");
                    let mut texture = material.base_color_texture.clone().unwrap_or_default();
                    if ui.text_edit_singleline(&mut texture).changed() {
                        material.base_color_texture = if texture.is_empty() {
                            None
                        } else {
                            Some(texture)
                        };
                        state.set_material(material.clone());
                    }
                });
            }

            if let Some(mut light) = snapshot.light {
                ui.separator();
                ui.label("Light");
                egui::ComboBox::from_id_salt("light-type")
                    .selected_text(format!("{:?}", light.light_type))
                    .show_ui(ui, |ui| {
                        for light_type in
                            [LightType::Directional, LightType::Point, LightType::Spot]
                        {
                            if ui
                                .selectable_label(
                                    light.light_type == light_type,
                                    format!("{light_type:?}"),
                                )
                                .clicked()
                            {
                                light.light_type = light_type;
                                state.set_light(light);
                            }
                        }
                    });
                if ui
                    .add(egui::Slider::new(&mut light.intensity, 0.0..=20.0).show_value(true))
                    .changed()
                {
                    state.set_light(light);
                }
            }

            if let Some(mut camera) = snapshot.camera {
                ui.separator();
                ui.label("Camera");
                if ui
                    .add(
                        egui::Slider::new(&mut camera.vertical_fov_degrees, 20.0..=120.0)
                            .show_value(true),
                    )
                    .changed()
                {
                    state.set_camera(camera);
                }
            }

            if let Some(mut physics) = snapshot.physics {
                ui.separator();
                ui.label("Physics");
                ui.label(format!("Body: {:?}", physics.body_type));
                if ui
                    .add(egui::Slider::new(&mut physics.mass, 0.01..=100.0).show_value(true))
                    .changed()
                {
                    state.set_physics(physics);
                }
                if ui
                    .add(egui::Slider::new(&mut physics.friction, 0.0..=2.0).show_value(true))
                    .changed()
                {
                    state.set_physics(physics);
                }
                if ui
                    .add(egui::Slider::new(&mut physics.restitution, 0.0..=1.0).show_value(true))
                    .changed()
                {
                    state.set_physics(physics);
                }
                if ui.checkbox(&mut physics.is_trigger, "Trigger").changed() {
                    state.set_physics(physics);
                }
            }

            ui.separator();
            ui.label("Components");
            for kind in snapshot.component_kinds.clone() {
                ui.horizontal(|ui| {
                    ui.label(kind.as_str());
                    if kind != ComponentKind::Transform && ui.small_button("X").clicked() {
                        state.remove_component(kind);
                    }
                });
            }
            ui.horizontal(|ui| {
                ui.label("Add");
                egui::ComboBox::from_id_salt("add-component")
                    .selected_text("Component")
                    .show_ui(ui, |ui| {
                        for kind in [
                            ComponentKind::Mesh,
                            ComponentKind::Material,
                            ComponentKind::Camera,
                            ComponentKind::Light,
                            ComponentKind::Physics,
                            ComponentKind::Script,
                            ComponentKind::AudioSource,
                        ] {
                            if ui.selectable_label(false, kind.as_str()).clicked() {
                                state.add_component(kind);
                            }
                        }
                    });
            });
        });
}

fn vector3_editor(ui: &mut Ui, label: &str, value: &mut [f32; 3]) -> (bool, bool) {
    ui.label(label);
    let mut changed = false;
    let mut drag_started = false;
    ui.horizontal(|ui| {
        for (index, axis) in ["X", "Y", "Z"].iter().enumerate() {
            ui.label(*axis);
            let response = ui.add(egui::DragValue::new(&mut value[index]).speed(0.05));
            changed |= response.changed();
            drag_started |= response.drag_started();
        }
    });
    (changed, drag_started)
}

fn bottom_panel(root: &mut Ui, state: &mut EditorState) {
    egui::Panel::bottom("assets_console")
        .resizable(true)
        .default_size(190.0)
        .min_size(100.0)
        .frame(Frame::new())
        .show(root, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(state.show_asset_browser, "Assets")
                    .clicked()
                {
                    state.show_asset_browser = true;
                    state.show_console = false;
                    state.show_profiler = false;
                }
                if ui.selectable_label(state.show_console, "Console").clicked() {
                    state.show_asset_browser = false;
                    state.show_console = true;
                    state.show_profiler = false;
                }
                if ui
                    .selectable_label(state.show_profiler, "Profiler")
                    .clicked()
                {
                    state.show_asset_browser = false;
                    state.show_console = false;
                    state.show_profiler = true;
                }
            });
            ui.separator();
            if state.show_asset_browser {
                asset_browser(ui, state);
            } else if state.show_console {
                console(ui, state);
            } else {
                profiler(ui, state);
            }
        });
}

fn asset_browser(ui: &mut Ui, state: &mut EditorState) {
    ui.horizontal(|ui| {
        ui.label("Search");
        ui.text_edit_singleline(&mut state.asset_search);
        ui.label("Import path");
        ui.text_edit_singleline(&mut state.asset_import_path);
        if ui.button("Import").clicked() {
            let source = std::path::PathBuf::from(&state.asset_import_path);
            match state.import_asset(&source) {
                Ok(_) => {}
                Err(error) => state.log(LogLevel::Error, error.to_string()),
            }
        }
    });
    ui.separator();
    egui::ScrollArea::vertical().show(ui, |ui| {
        let files = state.asset_files();
        if files.is_empty() {
            ui.label("No assets. Import a model, texture, audio file, or script.");
        }
        let query = state.asset_search.to_ascii_lowercase();
        for file in files {
            let name = file.file_name().unwrap_or_default().to_string_lossy();
            if !query.is_empty() && !name.to_ascii_lowercase().contains(&query) {
                continue;
            }
            ui.horizontal(|ui| {
                ui.label(asset_icon(&name));
                ui.label(name);
                ui.label(RichText::new(file.display().to_string()).weak().small());
            });
        }
    });
}

fn asset_icon(name: &str) -> &'static str {
    let extension = std::path::Path::new(name)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "gltf" | "glb" | "obj" => "M",
        "png" | "jpg" | "jpeg" | "webp" | "ktx2" => "T",
        "wav" | "ogg" | "mp3" => "A",
        "lua" => "S",
        _ => "F",
    }
}

fn console(ui: &mut Ui, state: &mut EditorState) {
    egui::ScrollArea::vertical()
        .stick_to_bottom(true)
        .show(ui, |ui| {
            if state.logs.is_empty() {
                ui.label("Console is ready");
            }
            for entry in &state.logs {
                let color = match entry.level {
                    LogLevel::Info => ui.visuals().text_color(),
                    LogLevel::Warning => WARNING,
                    LogLevel::Error => ERROR,
                };
                ui.label(RichText::new(entry.message.clone()).color(color));
            }
        });
}

fn profiler(ui: &mut Ui, state: &EditorState) {
    let stats = state.performance;
    egui::Grid::new("profiler-grid")
        .num_columns(2)
        .spacing([18.0, 6.0])
        .show(ui, |ui| {
            ui.strong("Frame rate");
            ui.label(format!("{:.1} FPS", stats.frames_per_second));
            ui.end_row();
            ui.strong("Frame time");
            ui.label(format!("{:.2} ms", stats.frame_time_milliseconds));
            ui.end_row();
            ui.strong("Draw calls");
            ui.label(stats.draw_calls.to_string());
            ui.end_row();
            ui.strong("Triangles");
            ui.label(stats.triangles.to_string());
            ui.end_row();
            ui.strong("Instances");
            ui.label(stats.instances.to_string());
            ui.end_row();
            ui.strong("Graphics adapter");
            ui.label(state.graphics_backend.clone());
            ui.end_row();
        });
}

fn viewport_panel(root: &mut Ui, state: &mut EditorState) {
    egui::CentralPanel::no_frame().show(root, |ui| {
        let rect = ui.available_rect_before_wrap();
        state.viewport_rect = rect;

        let context = ui.ctx().clone();
        let camera = editor_camera(state);
        let aspect = rect.width() / rect.height().max(1.0);
        let pointer = context.input(|input| input.pointer.hover_pos());
        let pointer_in_viewport = pointer.is_some_and(|position| rect.contains(position));
        if !context.egui_wants_pointer_input()
            && (pointer_in_viewport || state.gizmo_drag.is_some())
        {
            let (delta, zoom, pan, primary_pressed, primary_released, primary_down, panning) =
                context.input(|input| {
                    (
                        input.pointer.delta(),
                        input.zoom_delta(),
                        input.pointer.delta(),
                        input.pointer.button_pressed(PointerButton::Primary),
                        input.pointer.button_released(PointerButton::Primary),
                        input.pointer.button_down(PointerButton::Primary),
                        input.pointer.secondary_down(),
                    )
                });
            let pointer = pointer.unwrap_or_default();

            if primary_pressed && state.gizmo_drag.is_none() {
                if let Some(axis) = gizmo_handle_near(state, &camera, rect, aspect, pointer) {
                    state.checkpoint();
                    state.gizmo_axis = Some(axis);
                    state.gizmo_drag = Some(GizmoDrag {
                        tool: state.transform_tool,
                        axis,
                        start_transform: state.selected_transform_snapshot().unwrap_or(
                            TransformSnapshot {
                                translation: [0.0; 3],
                                rotation_degrees: [0.0; 3],
                                scale: [1.0; 3],
                            },
                        ),
                        start_pointer: pointer,
                    });
                } else {
                    state.pick_entity(pointer, rect);
                }
            }
            if primary_released {
                state.gizmo_axis = None;
                state.gizmo_drag = None;
            }

            if let Some(drag) = state.gizmo_drag {
                if let Some(origin) = state.gizmo_origin() {
                    let center = world_to_screen(origin, &camera, rect, aspect);
                    if drag.tool == TransformTool::Rotate {
                        if let Some(center) = center {
                            let start_angle = pointer_angle(drag.start_pointer, center);
                            let angle = pointer_angle(pointer, center);
                            state.apply_gizmo_drag(drag, 0.0, angle_delta(start_angle, angle));
                        }
                    } else if let (Some(start), Some(end)) = (
                        center,
                        world_to_screen(origin + drag.axis.vector(), &camera, rect, aspect),
                    ) {
                        let screen_axis = end - start;
                        let pixels_per_unit = screen_axis.length();
                        if pixels_per_unit > 0.0 {
                            let direction = screen_axis / pixels_per_unit;
                            let world_delta =
                                (pointer - drag.start_pointer).dot(direction) / pixels_per_unit;
                            state.apply_gizmo_drag(drag, world_delta, 0.0);
                        }
                    }
                }
            } else {
                if primary_down && !panning {
                    state
                        .camera
                        .orbit(delta.x * 0.008, delta.y * 0.008, -delta.y * 0.01);
                }
                if zoom.abs() > 0.001 {
                    state.camera.orbit(0.0, 0.0, (1.0 - zoom) * 2.0);
                }
                if panning {
                    state.camera_target.x -= pan.x * 0.02;
                    state.camera_target.y += pan.y * 0.02;
                }
            }
        }

        if !context.egui_wants_keyboard_input()
            && context.input(|input| input.key_pressed(egui::Key::Delete))
        {
            state.delete_selected();
        }
        if !context.egui_wants_keyboard_input()
            && context.input(|input| input.key_pressed(egui::Key::F))
        {
            state.focus_selected();
        }
        if !context.egui_wants_keyboard_input()
            && context.input(|input| {
                input.modifiers.ctrl && input.key_pressed(egui::Key::Z) && !input.modifiers.shift
            })
        {
            state.undo();
        }
        if !context.egui_wants_keyboard_input()
            && context.input(|input| {
                (input.modifiers.ctrl && input.key_pressed(egui::Key::Y))
                    || (input.modifiers.ctrl
                        && input.modifiers.shift
                        && input.key_pressed(egui::Key::Z))
            })
        {
            state.redo();
        }

        if let Some(origin) = state.gizmo_origin() {
            let size = 0.6 + (camera.position - camera.target).length() * 0.04;
            let center = world_to_screen(origin, &camera, rect, aspect);
            let colors = [
                (GizmoAxis::X, Color32::from_rgb(232, 86, 96)),
                (GizmoAxis::Y, Color32::from_rgb(104, 214, 112)),
                (GizmoAxis::Z, Color32::from_rgb(86, 150, 240)),
            ];
            if state.transform_tool == TransformTool::Rotate {
                if let Some(center) = center {
                    let screen_unit =
                        world_to_screen(origin + Vec3::Y * size, &camera, rect, aspect)
                            .map(|point| point.distance(center))
                            .unwrap_or(60.0);
                    for (index, (axis, color)) in colors.into_iter().enumerate() {
                        let active = state.gizmo_axis == Some(axis);
                        ui.painter().circle_stroke(
                            center,
                            ring_radius(screen_unit, index),
                            Stroke::new(if active { 4.0 } else { 2.5 }, color),
                        );
                    }
                    ui.painter().circle_filled(center, 3.0, Color32::WHITE);
                }
            } else {
                for (axis, color) in colors {
                    let target = origin + axis.vector() * size;
                    if let (Some(start), Some(end)) =
                        (center, world_to_screen(target, &camera, rect, aspect))
                    {
                        let active = state.gizmo_axis == Some(axis);
                        ui.painter().line_segment(
                            [start, end],
                            Stroke::new(if active { 4.0 } else { 2.5 }, color),
                        );
                        if state.transform_tool == TransformTool::Scale {
                            ui.painter().rect_filled(
                                Rect::from_center_size(end, vec2(9.0, 9.0)),
                                0.0,
                                color,
                            );
                        } else {
                            ui.painter()
                                .circle_filled(end, if active { 6.0 } else { 4.0 }, color);
                        }
                    }
                }
            }
        }

        ui.horizontal(|ui| {
            ui.label("Perspective");
            ui.separator();
            ui.label(match state.play_mode {
                PlayMode::Editing => "Editing",
                PlayMode::Playing => "Playing",
                PlayMode::Paused => "Paused",
            });
            ui.separator();
            if let Some(id) = state.selected {
                let name = state
                    .scene
                    .entities
                    .get(&id)
                    .map(|entity| entity.name.as_str())
                    .unwrap_or("Missing entity");
                ui.label(RichText::new(name).color(ACCENT));
            }
        });
    });
}

fn editor_camera(state: &EditorState) -> Camera {
    Camera {
        position: state.camera.position(state.camera_target),
        target: state.camera_target,
        projection: CameraProjection::Perspective {
            vertical_fov_degrees: 60.0,
        },
        near: 0.05,
        far: 500.0,
    }
}

fn world_to_screen(point: Vec3, camera: &Camera, rect: Rect, aspect: f32) -> Option<Pos2> {
    let clip = camera.view_projection(aspect) * point.extend(1.0);
    if clip.w <= 0.0 {
        return None;
    }
    let ndc = clip.xyz() / clip.w;
    Some(Pos2::new(
        rect.min.x + (ndc.x + 1.0) * 0.5 * rect.width(),
        rect.min.y + (1.0 - ndc.y) * 0.5 * rect.height(),
    ))
}

fn gizmo_handle_near(
    state: &EditorState,
    camera: &Camera,
    rect: Rect,
    aspect: f32,
    pointer: Pos2,
) -> Option<GizmoAxis> {
    let origin = state.gizmo_origin()?;
    let size = 0.6 + (camera.position - camera.target).length() * 0.04;
    let center = world_to_screen(origin, camera, rect, aspect)?;
    let mut nearest = None;
    for (index, axis) in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z]
        .into_iter()
        .enumerate()
    {
        let distance = if state.transform_tool == TransformTool::Rotate {
            let screen_unit = world_to_screen(origin + Vec3::Y * size, camera, rect, aspect)
                .map(|point| point.distance(center))
                .unwrap_or(60.0);
            (pointer.distance(center) - ring_radius(screen_unit, index)).abs()
        } else {
            let end = world_to_screen(origin + axis.vector() * size, camera, rect, aspect)?;
            distance_to_segment(pointer, center, end)
        };
        if distance <= 8.0 && nearest.is_none_or(|(current, _)| distance < current) {
            nearest = Some((distance, axis));
        }
    }
    nearest.map(|(_, axis)| axis)
}

fn ring_radius(screen_unit: f32, index: usize) -> f32 {
    screen_unit * 0.45 + index as f32 * 12.0
}

fn pointer_angle(pointer: Pos2, center: Pos2) -> f32 {
    (pointer.y - center.y).atan2(pointer.x - center.x)
}

fn angle_delta(start: f32, current: f32) -> f32 {
    let full = std::f32::consts::TAU;
    let mut delta = (current - start) % full;
    if delta > std::f32::consts::PI {
        delta -= full;
    }
    if delta < -std::f32::consts::PI {
        delta += full;
    }
    delta
}

fn distance_to_segment(point: Pos2, start: Pos2, end: Pos2) -> f32 {
    let segment = end - start;
    let length_squared = segment.length_sq();
    if length_squared <= 0.0 {
        return point.distance(start);
    }
    let t = ((point - start).dot(segment) / length_squared).clamp(0.0, 1.0);
    point.distance(start + segment * t)
}

fn welcome_window(context: &Context, state: &mut EditorState) {
    if !state.show_welcome {
        return;
    }
    egui::Window::new("Welcome to CUBES")
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .show(context, |ui| {
            ui.set_width(440.0);
            ui.vertical_centered(|ui| {
                ui.add_space(6.0);
                ui.heading("Make something in 3D");
                ui.label("Choose a starting point. Existing Vela projects open unchanged.");
            });
            ui.add_space(12.0);
            if ui.button("Create New Game").clicked() {
                state.dismiss_welcome();
                state.modal = EditorModal::NewProject;
            }
            if ui.button("Open Existing Project").clicked() {
                state.dismiss_welcome();
                state.modal = EditorModal::OpenProject;
            }
            if ui.button("Start From Template").clicked() {
                state.dismiss_welcome();
                state.show_templates = true;
            }
            if ui.button("Learn CUBES").clicked() {
                state.dismiss_welcome();
                state.show_help = true;
            }
            ui.add_enabled(
                false,
                egui::Button::new("Browse Example Projects (planned)"),
            );
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Close").clicked() {
                    state.dismiss_welcome();
                }
                if ui.button("Don't show again").clicked() {
                    state.dismiss_welcome();
                }
            });
        });
}

fn template_window(context: &Context, state: &mut EditorState) {
    if !state.show_templates {
        return;
    }
    egui::Window::new("Start From Template")
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .show(context, |ui| {
            ui.set_width(440.0);
            ui.label("Basic 3D Starter - camera, light, cube and ground");
            if ui.button("Use Basic 3D Starter").clicked() {
                state.start_starter_template();
            }
            ui.separator();
            ui.label("Planned templates:");
            for name in [
                "First-person",
                "Third-person",
                "Platformer",
                "Racing",
                "Puzzle",
                "Survival",
                "Exploration",
                "Top-down",
            ] {
                ui.add_enabled(false, egui::Button::new(format!("{name} (planned)")));
            }
            if ui.button("Close").clicked() {
                state.show_templates = false;
            }
        });
}

fn help_window(context: &Context, state: &mut EditorState) {
    if !state.show_help {
        return;
    }
    egui::Window::new("Quick start")
        .collapsible(false)
        .resizable(true)
        .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .show(context, |ui| {
            ui.set_width(460.0);
            ui.label("1. Create a project or open an existing one.");
            ui.label("2. Add a cube from the hierarchy Add menu.");
            ui.label("3. Click an object to select it, then edit it in the Inspector.");
            ui.label("4. Drag in the viewport to orbit; right-drag to pan; scroll to zoom.");
            ui.label("5. Drag the coloured X, Y or Z gizmo arrow to move the selection.");
            ui.label("6. Press Play to preview, then Stop to restore the edited scene.");
            ui.label("7. Save with Ctrl+S or File > Save scene.");
            if ui.button("Got it").clicked() {
                state.show_help = false;
            }
        });
}

fn about_window(context: &Context, state: &mut EditorState) {
    if !state.show_about {
        return;
    }
    egui::Window::new("About CUBES")
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .show(context, |ui| {
            ui.set_width(360.0);
            ui.heading("CUBES");
            ui.label("A lightweight, AI-native 3D creation platform.");
            ui.label("Version 0.1.0-dev");
            ui.label("Existing Vela projects and MCP integrations remain supported.");
            if ui.button("Close").clicked() {
                state.show_about = false;
            }
        });
}

fn project_modal(context: &Context, state: &mut EditorState) {
    let title = match state.modal {
        EditorModal::NewProject => "New CUBES Project",
        EditorModal::OpenProject => "Open CUBES Project",
        EditorModal::None => return,
    };
    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .show(context, |ui| {
            ui.set_width(420.0);
            ui.label("Project directory");
            ui.text_edit_singleline(&mut state.modal_path);
            if matches!(state.modal, EditorModal::NewProject) {
                ui.label("Project name");
                ui.text_edit_singleline(&mut state.modal_name);
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    state.modal = EditorModal::None;
                }
                let action = match state.modal {
                    EditorModal::NewProject => "Create",
                    EditorModal::OpenProject => "Open",
                    EditorModal::None => "",
                };
                if ui.button(action).clicked() {
                    let path = std::path::PathBuf::from(&state.modal_path);
                    let name = state.modal_name.clone();
                    let result = match state.modal {
                        EditorModal::NewProject => state.create_project(&path, &name),
                        EditorModal::OpenProject => state.open_project(&path),
                        EditorModal::None => Ok(()),
                    };
                    if let Err(error) = result {
                        state.log(LogLevel::Error, error.to_string());
                    } else {
                        state.modal = EditorModal::None;
                    }
                }
            });
        });
}
