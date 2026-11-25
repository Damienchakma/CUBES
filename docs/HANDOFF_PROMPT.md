# Handoff prompt for continuing CUBES

Historical prompt from before the CUBES rename. For the current state, read
[CUBES_ASSESSMENT.md](CUBES_ASSESSMENT.md) and [STATUS.md](STATUS.md) first.

Copy everything below this line into another AI session.

---

You are continuing an already implemented Rust game-engine project called **CUBES**. Work in `/home/damien/Documents/Default Project` on a Raspberry Pi 5. Do not restart from a proposal; continue the existing workspace. Keep changes minimal but complete, compile and test continuously, and never claim a feature works unless it was actually run or tested. Update `docs/STATUS.md` honestly whenever feature status changes. The user prefers clear, concise progress and dislikes fake/placeholder claims. A "ponytail" senior-dev style was requested earlier: smallest complete change, no unnecessary abstraction, no features nobody asked for.

## Hard project rules

- Primary target: Raspberry Pi 5 Model B Rev 1.0, Kali GNU/Linux Rolling 2026.2, aarch64, kernel `6.12.34+rpt-rpi-2712`, 8 GiB RAM, X11 `DISPLAY=:0.0`.
- Vulkan is preferred and verified through Broadcom V3D; OpenGL is a fallback. Adapter selection must use the actual wgpu surface for windowed apps.
- No fabricated benchmark results. No claiming multi-SBC, shadows, audio, animation, glTF GPU import, export, or full physics works.
- MCP must never provide shell/code execution and must sandbox every project path under the configured workspace root.
- Use `cargo fmt --all` and `cargo test --workspace` after changes.

## Toolchain and environment

- Rust stable `1.100.0` installed under `~/.cargo`; activate with `. "$HOME/.cargo/env"`.
- Workspace is **not** a Git repository. No branch or PR exists.
- OpenCode `browse` 0.1.5 was installed into `~/.local/bin/browse` and connected to the user's Chromium once; it later disconnected. No active agent browser tabs remain.
- The desktop app has been installed into the user's XFCE application menu; see "Desktop app" below.

## Architecture and key decisions

- Workspace crates:
  - `crates/vela-core`: project format, atomic JSON writes, canonical sandbox checks.
  - `crates/vela-scene`: serializable scene graph and components.
  - `crates/vela-ecs`: compact dense type-specific ECS with generation reuse; tested, but not yet the authoritative runtime world.
  - `crates/vela-platform`: CPU/memory/thermal/DRM/Vulkan capability probe and quality policy.
  - `crates/vela-renderer`: wgpu adapter selection, camera, instanced meshes, grid, PBR-style WGSL shader, draw metrics.
  - `crates/vela-assets`: asset kind detection, SHA-256 checksum, and copy into a project's assets directory.
  - `crates/vela-scripting`: embedded Lua 5.4 runtime with `on_start` and `on_update`.
  - `crates/vela-mcp`: rmcp 3.11 stdio MCP server with sandboxed tools/resources/prompts.
  - `apps/vela-cli`: binary named `vela`; commands `hardware`, `new-project`, `inspect-project`.
  - `apps/vela-editor`: native winit/wgpu/egui editor.
  - `apps/vela-runtime`: standalone windowed player with Lua and simple dynamics.
  - `benchmarks/vela-bench`: deterministic offscreen GPU throughput benchmark.
- Pinned versions: `wgpu 30.0.2`, `winit 0.30.12` (resolves to 0.30.13), `egui/egui-wgpu/egui-winit 0.37.0`, `glam 0.34.3`, `mlua 0.12.3`, `rmcp 3.11.0`, `tokio 1.x`, `clap 4.6.9`, `sha2 0.10`.
- Project format: `project.vela.json`, `scenes/main.velascene`, directories `assets/`, `scripts/`, `builds/`, `.vela/`.
- The editor uses `ApplicationHandler`/`ActiveEventLoop`; do not revert to deprecated `EventLoop::run` or `EventLoop::create_window`.
- `egui` must keep the `default_fonts` feature; without it all text renders as tofu boxes on the Pi.
- `MeshComponent::default()` must keep `visible = true`; a derived default made all meshes disappear.
- Plane geometry must stay wound counter-clockwise from above or back-face culling hides the ground.
- `GpuContext::from_instance_for_surface(...)` is required for windowed apps so the adapter can present to the surface.

## Already completed and verified

### Editor

- Real X11/Vulkan editor window with resizable side/top/bottom panels, hierarchy, inspector, asset browser, console, profiler, and central 3D viewport.
- Selection highlights the chosen mesh and populates the inspector.
- Inspector edits name, visible/locked flags, transform, mesh primitive, material color/metallic/roughness, light type/intensity, camera FOV, and physics mass/friction/restitution/trigger.
- Hierarchy supports search, add empty/cube/plane/camera/light, visibility toggle, lock toggle, duplicate, delete, and focus.
- Viewport supports orbit, pan, scroll zoom, `F` focus, `Delete`, play/pause/stop, and a play-mode scene snapshot restored on stop.
- Vertical orbit direction was inverted after user feedback: `apps/vela-editor/src/ui.rs` now passes `delta.y * 0.008` instead of `-delta.y * 0.008`; a regression test in `apps/vela-editor/src/state.rs` verifies that dragging down moves the orbit camera above the target. The debug binary was rebuilt after this.
- Project new/open/save works via the modal and `Ctrl+S`; saving during play is blocked.
- Live screenshot proof: `docs/screenshots/editor.png`.

### Runtime

- `apps/vela-runtime/src/main.rs` is a real windowed player, not a placeholder.
- Opens a project, loads the starter scene, uses the scene camera, loads sorted `scripts/*.lua`, calls `on_start`, calls `on_update(entity, delta)`, and runs simple semi-implicit Euler dynamics for dynamic bodies with floor collision, friction, and restitution.
- Logs real FPS, draw calls, triangles, and instances every 2 seconds.
- Verified on Pi 5: about 57–59 FPS on the two-instance starter scene over 12-second runs.

### MCP

- `vela-mcp` is a real stdio binary built on rmcp 3.11.
- Tools: `vela_hardware`, `vela_project_info`, `vela_list_entities`, `vela_get_entity`, `vela_list_assets`, `vela_read_asset`, `vela_create_entity`.
- Resources: `vela://hardware`, `vela://status`, `vela://schema/project`.
- Prompt: `build-scene` with optional `goal` argument.
- Sandbox: project path must canonicalize inside MCP workspace root; scene/asset paths use `ProjectPaths`, rejecting absolute paths, `..`, and symlink escapes; asset reads capped at 64 KiB.
- `crates/vela-mcp/tests/stdio.rs` starts the real binary over stdio and verifies listing, a sandboxed write, and traversal rejection.

### Assets, scripting, ECS

- `vela-assets` classifies and checksums files and copies them into a project's `assets/` directory. Unit test verifies checksum preservation after copy.
- `vela-scripting` was repaired (it had uncompiled dependency/signature bugs) and now has a unit test for `example_script`, `on_start`, and `on_update`.
- `vela-ecs` has passing tests for component round trip and generation reuse on despawn.

### Hardware and benchmark

- Verified adapter: `V3D 7.1.7.0`, `V3DV Mesa`, vendor `0x14e4`, device `0x55701c33`, Vulkan, render node `/dev/dri/renderD128`.
- `vela hardware --backend vulkan` works.
- Benchmark result saved in `docs/benchmarks/pi5-vulkan.json`: 1280×720, 512 instances, 300 measured frames after 30 warmup, average 39.51 ms, 25.31 FPS, p95 78.05 ms, 5 draw calls, 50,944 triangles. Thermal state at run was 82.1 °C with throttling detected. Do not present this as a stable sustained frame budget.
- Benchmark timing is offscreen CPU encoding plus GPU completion, not GPU timestamp queries or vsynced presentation FPS.

### Desktop app

- User asked for a desktop app. The editor is a native winit/wgpu app; a launcher was added:
  - Source: `desktop/install-desktop-app.sh`, `desktop/vela-editor.svg`, `desktop/README.md`.
  - Installed: `~/.local/share/applications/vela-editor.desktop`, `~/.local/share/icons/vela-editor.svg`, `~/.local/bin/vela-editor-desktop`.
  - Desktop entry validates with `desktop-file-validate`.
  - Launcher was tested and opened the real `CUBES Editor` window.
  - Launcher points at `target/debug/vela-editor`; rerun the installer if the workspace moves.

### Documentation written

- `README.md`
- `docs/ARCHITECTURE.md`
- `docs/DEPENDENCIES.md`
- `docs/HARDWARE.md`
- `docs/TUTORIAL.md`
- `docs/MCP.md`
- `docs/BENCHMARKS.md`
- `docs/STATUS.md`
- `docs/HANDOFF_PROMPT.md` (this prompt)

## Validation commands that pass

```bash
. "$HOME/.cargo/env"
cargo fmt --all --check
cargo test --workspace
cargo run -p vela-cli -- hardware --backend vulkan
cargo run -p vela-cli -- new-project /tmp/vela-demo "CUBES Demo"
cargo run -p vela-editor -- --project /tmp/vela-demo
cargo run -p vela-runtime -- /tmp/vela-demo
cargo run -p vela-mcp -- "$PWD"
cargo run --release -p vela-bench -- --instances 512 --frames 300 --backend vulkan
desktop/install-desktop-app.sh
```

`cargo test --workspace` passes cleanly with no compiler warnings. Current tests cover assets, core, ECS, editor state, MCP stdio, platform, renderer meshes, scene, and scripting.

## Known gaps / do not claim as working

- No second SBC was available, so multi-SBC compatibility is not verified.
- No shadow maps, audio playback, animation, terrain, skeletal meshes, or GPU texture/model upload from imported assets.
- glTF/OBJ files are classified, checksummed, and copied but not rendered.
- Physics is only a simple dynamic floor solver; no arbitrary colliders or body-to-body interactions.
- No true drag-docking; panels are resizable but fixed to sides.
- No transform gizmos; transforms are edited numerically in the inspector.
- No build/export/package pipeline beyond raw project directories.
- No GPU timestamp profiling; profiler uses rolling frame times and renderer draw metrics.
- No `.deb`, `.AppImage`, auto-update, or system-wide desktop install.

## Suggested next work, in priority order

1. Ask the user what to build next; do not assume. If they want more editor usability, add real transform gizmos and click-picking in the viewport.
2. If they want asset work, implement GPU texture upload and glTF/OBJ mesh loading instead of merely classifying files.
3. If they want gameplay, extend the runtime ECS/physics: velocity components, multiple colliders, static/kinematic bodies, and Lua-exposed transform APIs.
4. If they want tooling, extend MCP with read-only scene queries for components/materials and safe scene mutation tools, keeping the no-shell rule.
5. If they want performance, add GPU timestamp queries where supported and run a cooled/repeated benchmark matrix, recording every result honestly.
6. If they want portability, obtain a second SBC and verify Vulkan/GL fallback there before changing status.

## Current exact state at handoff

- The last user request was to fix inverted vertical mouse orbit and provide this handoff prompt.
- The orbit sign fix is applied, tested, formatted, and the debug editor binary was rebuilt.
- No source changes after that except `docs/STATUS.md` and this handoff prompt.
- The workspace is ready for the next instruction.
