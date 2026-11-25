# CUBES product assessment

This is a concise assessment of the existing implementation against the CUBES product directive,
written before the next milestone work. It is intentionally honest.

## What already exists

- A real native Rust application stack: winit event loop, wgpu 30 renderer with verified Vulkan
  adapter on the Pi 5, egui editor, standalone runtime, CLI, Lua scripting, asset checksum/copy,
  compact ECS, and a secure rmcp stdio server.
- A working scene format, project sandbox, atomic writes, hierarchy operations, transform/material
  editing, play preview with scene restore, and a real 3D viewport.
- Verified hardware probe, thermal warnings, offscreen GPU benchmark, desktop launcher, and docs.
- `cargo test --workspace` passes with no compiler warnings.

## Gaps against the full vision

The product directive covers a general-purpose creation suite. The following are still absent:
visual scripting, animation timeline/keyframes, skeletal meshes, model editing, audio, particles,
navigation, UI authoring, shadows, post-processing, advanced lighting, advanced PBR maps, glTF
animations, Roblox/Blender interchange, standalone export, gizmo snapping and
multi-axis handles, true docking, templates beyond the starter scene, input mapping, C++/plugin SDK,
and cross-platform packaging. These are planned or unsupported, not implemented. glTF mesh geometry
and base-colour textures are now implemented.

## Architectural blockers / risks

1. **Resolution assumptions.** Project render settings exist but windowed apps currently size the
   window from the manifest; internal render resolution and dynamic scaling are not separated from
   output size.
2. **ECS is not authoritative.** `vela-ecs` is tested but the editor/runtime scene graph is still
   the live gameplay world. A larger gameplay system will need a clear ownership decision.
3. **Asset pipeline is partial.** glTF geometry and base-colour textures reach the GPU, but normal/ORM
   maps, skins, animations, and formal import reports are still missing.
4. **No stable public gameplay API.** Lua currently exposes only `on_start`/`on_update`; agents and
   users do not yet share a richer API surface.
5. **Undo is snapshot-based, not command-based.** Editor undo/redo now works, but agent-authored
   mutations still need to be routed through the same history before rollback is complete.
6. **Desktop packaging is user-local.** There is no `.deb`, `.AppImage`, Windows, or macOS package.

## Milestones completed so far

**Milestone 1: CUBES identity and first-contact editor experience.**

Completed: user-facing CUBES branding with Vela file/MCP/launcher compatibility, 3D cube logo,
first-run welcome screen, viewport picking, Move/Rotate/Scale gizmos, undo/redo, Help/About dialogs,
and matching docs.

**Milestone 2, first slice: real glTF mesh geometry.**

Completed: glTF 2.0 first-primitive loader, node-transform baking, GPU mesh upload, `MeshComponent.asset`
support in the inspector and scene files, automatic mesh assignment on model import, and real
rendering of an imported glTF pyramid in both editor and runtime.

**Milestone 2, second slice: texture and material import.**

Completed: UV-aware mesh format, texture sampling in the WGSL shader, GPU texture upload with
project-relative keys, `MaterialComponent.base_color_texture` with serde default, Inspector texture
field, external and embedded glTF base-colour image decoding, PNG persistence for embedded images,
and a textured glTF pyramid verified in both editor and runtime on the Pi 5.

Deferred from Milestone 2: normal/ORM maps, texture compression, skins, animation tracks, FBX, and a
formal import report UI.

## Smallest coherent next milestone

**Milestone 3, first slice: reusable gameplay systems.** Start with a real first-person controller
and input mapping, then expose transform and input APIs to Lua so beginner templates can be built on
the same public systems.

## Why this milestone next

The asset pipeline now delivers visible geometry and materials. The next user-visible gap is a
playable game loop with reusable controllers and scripting APIs, which does not require another
renderer rewrite.
