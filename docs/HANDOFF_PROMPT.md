# Full CUBES handoff prompt

Copy everything below the divider into another AI session. This is the current state as of 2026-11-25.

---

You are continuing an existing Rust project called **CUBES** at `/home/damien/Documents/Default Project`. Do not restart it, discard working code, replace the architecture unnecessarily, or stop at a proposal. Inspect the workspace, preserve working systems, and implement improvements incrementally. Keep changes minimal but complete, compile and test continuously, and never claim a feature works unless it was actually run or tested. Update `docs/STATUS.md` honestly whenever feature status changes.

A "ponytail" senior-dev style was requested: smallest complete change, no unnecessary abstraction, no features nobody asked for. The user prefers clear, concise progress and dislikes fake or placeholder claims.

## Hard rules

- Primary target: Raspberry Pi 5 Model B Rev 1.0, Kali GNU/Linux Rolling 2026.2, aarch64, kernel `6.12.34+rpt-rpi-2712`, 8 GB RAM, X11 `DISPLAY=:0.0`.
- Vulkan is preferred and verified through Broadcom V3D; OpenGL is a fallback. Adapter selection must use the actual wgpu surface for windowed apps.
- No fabricated benchmark results. No claiming multi-SBC, shadows, audio, animation, full physics, export, or other unimplemented features work.
- MCP must never provide shell/code execution and must sandbox all project paths under the configured workspace root.
- Use `cargo fmt --all --check` and `cargo test --workspace` after changes.
- Ask before destructive Git actions, force pushes, deleting branches, or creating new repositories.

## Environment and toolchain

- Rust stable `1.100.0` under `~/.cargo`; activate with `. "$HOME/.cargo/env"`.
- Git `2.53.0` installed.
- GitHub CLI `gh 2.105.0` installed at `~/.local/bin/gh`; `~/.local/bin` is already in `~/.zshrc`.
- Authenticated to GitHub as **Damienchakma** via GitHub CLI browser device login using HTTPS. Git operations are configured through `gh auth setup-git`.
- Access token expires in about 8 hours but refreshes automatically; refresh token lasts about 6 months. Never print or store tokens in chat or files.
- Git identity: `Damien <122607828+Damienchakma@users.noreply.github.com>`.
- Workspace is now a Git repository on branch `main` with remote `origin` at `https://github.com/Damienchakma/CUBES.git`.
- Latest pushed commit at handoff: `5529dbf Initial CUBES implementation: editor, runtime, MCP, assets and docs`.
- Working tree was clean after the push.
- `browse` 0.1.5 was installed into `~/.local/bin/browse` and connected to Chromium earlier. Available browser tooling has changed across server restarts; do not assume a browser namespace is present.

## Product identity and compatibility

- User-facing product name is **CUBES**.
- Internal Rust crate names remain `vela-*` to avoid risky renames.
- Existing Vela files and integrations remain supported:
  - `project.vela.json`
  - `*.velascene`
  - `vela://` MCP resource URIs
  - `vela_*` MCP tool names
  - `Vela Editor (legacy)` desktop launcher
- Original 3D cube identity exists in `desktop/cubes-dark.svg`, `desktop/cubes-light.svg`, `desktop/cubes-icon-128.png`, and `desktop/cubes-icon-256.png`.
- Desktop installer creates both `CUBES Editor` and legacy `Vela Editor (legacy)` entries.

## Workspace overview

- `apps/vela-cli` — binary named `vela`; CLI help shows CUBES; commands `hardware`, `new-project`, `inspect-project`.
- `apps/vela-editor` — native winit/wgpu/egui editor.
- `apps/vela-runtime` — standalone windowed player.
- `benchmarks/vela-bench` — deterministic offscreen GPU benchmark.
- `crates/vela-core` — project format, atomic JSON, canonical path sandbox.
- `crates/vela-scene` — serializable scene graph and component schema.
- `crates/vela-ecs` — compact dense ECS with generation reuse; tested but not authoritative runtime world.
- `crates/vela-platform` — hardware, DRM, thermal, capability probe and quality policy.
- `crates/vela-renderer` — wgpu adapter, camera, instanced meshes, grid, shader, glTF import, texture upload.
- `crates/vela-assets` — asset classification, checksum, and project copy.
- `crates/vela-scripting` — embedded Lua 5.4 with `on_start`/`on_update`.
- `crates/vela-mcp` — rmcp 3.11 stdio server with sandboxed tools/resources/prompts.

Major decisions and versions: `wgpu 30.0.2`, `winit 0.30.12` resolving to 0.30.13, `egui/egui-wgpu/egui-winit 0.37.0`, `glam 0.34.3`, `mlua 0.12.3`, `rmcp 3.11.0`, `gltf 1.4.1`, `image 0.25.10`.

## Implemented and tested

### Editor

- Real X11/Vulkan editor window titled `CUBES Editor`.
- Resizable hierarchy, inspector, asset browser, console, and profiler panels.
- First-run welcome window with Create New Game, Open Existing Project, Start From Template, Learn CUBES, and Browse Example Projects (disabled/planned). First-run marker at `~/.config/cubes/welcome-seen`.
- New project wizard and project open/save using atomic JSON writes.
- Hierarchy search, add empty/cube/plane/camera/light, visibility/lock toggles, duplicate, delete, focus.
- Inspector editing for name, flags, transform, mesh primitive and asset, material base colour/metallic/roughness/texture, light, camera, and physics basics.
- Viewport orbit/pan/zoom, object picking by local-space ray/AABB/ground plane tests, and real Move/Rotate/Scale gizmos.
- Undo/redo using bounded scene snapshots; toolbar buttons, Edit menu, and Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z.
- Play/pause/stop with scene snapshot restored on stop; saving disabled during play.
- Profiler shows real FPS, frame time, draw calls, triangles, instances, and adapter.

### Runtime

- Windowed player opens a project and loads `scenes/main.velascene`.
- Uses the scene camera and clear colour.
- Loads sorted `scripts/*.lua`, calls `on_start`, then `on_update(entity, delta)`.
- Simple semi-implicit Euler physics for dynamic bodies with floor collision, friction, and restitution.
- Lazily loads glTF meshes and textures through `ProjectPaths`.
- Logs real FPS, draw calls, triangles, and instances every two seconds.

### Renderer and assets

- Vulkan adapter selected with compatible surface on Pi 5: `V3D 7.1.7.0`, `V3DV Mesa`, vendor `0x14e4`, device `0x55701c33`.
- Instanced renderer groups draws by mesh and texture key, keeping draw calls bounded.
- PBR-style WGSL shader with GGX direct light, ambient, gamma, texture sampling, and selection emissive.
- UV-aware mesh format.
- `load_gltf_mesh` imports first usable glTF 2.0 primitive, bakes first node transform, reads positions/normals/UVs/indices, base colour factor, and base colour texture.
- External and embedded glTF images are decoded; embedded images are persisted as PNG under project `assets/`.
- `write_rgba_png` persists decoded RGBA8 textures.
- `MaterialComponent.base_color_texture` has `#[serde(default)]` for old project compatibility. MaterialComponent is no longer `Copy`.

### MCP

- `vela-mcp` stdio server built on rmcp 3.11.
- Tools: `vela_hardware`, `vela_project_info`, `vela_list_entities`, `vela_get_entity`, `vela_list_assets`, `vela_read_asset`, `vela_create_entity`.
- Resources: `vela://hardware`, `vela://status`, `vela://schema/project`.
- Prompt: `build-scene` with optional `goal`.
- Sandbox: project root must canonicalize inside workspace root; scene/asset paths validated; traversal rejected; no shell or arbitrary code execution.
- Integration test starts the real binary over stdio.

### Desktop and docs

- `desktop/install-desktop-app.sh` installs CUBES and legacy Vela launchers plus icons.
- Docs: `README.md`, `docs/ARCHITECTURE.md`, `docs/DEPENDENCIES.md`, `docs/HARDWARE.md`, `docs/TUTORIAL.md`, `docs/MCP.md`, `docs/BENCHMARKS.md`, `docs/STATUS.md`, `docs/CUBES_ASSESSMENT.md`.
- Screenshots: welcome, gizmos, imported mesh editor/runtime, textured mesh editor/runtime.

## Validation evidence

- `cargo fmt --all --check` passes.
- `cargo test --workspace` passes. At handoff: 20 unit/integration tests, 0 failures. The `grep` count of `test result: ok` lines included doc-test result lines, so trust the actual test counts, not that count alone.
- Editor, runtime, CLI, MCP stdio test, and benchmark were run on the Pi 5.
- Runtime starter scene: about 57–59 FPS.
- Textured imported mesh runtime: about 58–59 FPS, 3 draw calls, 6 triangles.
- Benchmark result: `docs/benchmarks/pi5-vulkan.json`: 1280×720, 512 instances, 300 measured frames, average 39.51 ms, 25.31 FPS, p95 78.05 ms, 5 draw calls, 50,944 triangles. Run occurred at 82.1 °C with throttling detected; do not present as stable sustained performance.
- Desktop entries validated with `desktop-file-validate`.

## Important technical gotchas

- `egui` must keep the `default_fonts` feature or all text renders as boxes.
- `MeshComponent::default()` must keep `visible = true`; a derived default once made every mesh invisible.
- Plane geometry must stay wound counter-clockwise from above or back-face culling hides the ground.
- Editor/runtime must use winit `ApplicationHandler`/`ActiveEventLoop`; do not revert to deprecated `EventLoop::run` or `EventLoop::create_window`.
- Windowed GPU contexts must use `GpuContext::from_instance_for_surface(...)`.
- Wgpu 30 uses `CurrentSurfaceTexture` enum and `queue.present(frame)`; `queue.submit([encoder.finish()])`.
- `MaterialComponent` is clone-but-not-copy because it owns a texture path string.
- `base_color_texture` uses serde default for old project compatibility.
- Texture rows use `Rgba8UnormSrgb`, a texture bind group, and a white default texture.
- glTF test fixtures must use `f32` literals and correct bufferView byte offsets; earlier failures came from `f64` position arrays and misordered UV/index data.
- `CUBES_SELECT=<id>`, `CUBES_TOOL=rotate|scale`, and `CUBES_MESH_ASSET=assets/model.gltf` are hidden debug environment shortcuts in the editor; upgrade to proper command flags if exposed to users.

## Known gaps / do not claim as working

- No second SBC tested.
- No shadows, ambient occlusion, post-processing, advanced ray tracing, audio, particles, terrain, skeletal meshes, or animation editor.
- No normal/ORM maps, texture compression, glTF skins, animation tracks, FBX, or formal import report UI.
- No visual scripting, node editor, input mapping, reusable gameplay controllers, prefabs, navigation, UI authoring, or localization.
- No standalone export/packaging, no `.deb`/`.AppImage`, no Windows/macOS builds.
- No true drag-docking, beginner/advanced UI modes, gizmo snapping, multi-selection, or examples gallery.
- Physics is only a simple dynamic floor solver.
- Undo is snapshot-based, not command-based; MCP mutations do not yet enter the editor undo history.

## Next coherent milestone

**Milestone 3, first slice: reusable gameplay systems.**

Start with a real first-person controller and input mapping, then expose transform and input APIs to Lua so beginner templates can be built on the same public systems. Do not implement all of Milestone 3 at once; make a small tested increment and update `docs/STATUS.md`.

Suggested order after that: animation workspace, native export, Roblox/Blender interchange, advanced rendering tiers, cross-platform packaging and fair Godot comparisons.

## Immediate instruction

Inspect the current workspace first, confirm Git status, then continue from Milestone 3 with a small working increment. Do not overwrite or discard existing functionality. Report exactly what changed, what passed, what failed, and what remains incomplete.
