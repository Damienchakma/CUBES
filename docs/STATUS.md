# CUBES feature status

Status values: **Implemented and tested**, **Implemented but not fully tested**, **In progress**,
**Planned**, **Unsupported**. "Implemented and tested" means there is an automated test, a real run,
or both on the Raspberry Pi 5 described in [HARDWARE.md](HARDWARE.md). Planned and unsupported
features are never described as working.

Last validation: after texture/material import, `cargo fmt --all --check` and
`cargo test --workspace` pass. Editor and runtime binaries were rebuilt. The welcome screen,
move/rotate/scale gizmos, an imported glTF pyramid, and a textured glTF pyramid were visually checked
on the Pi 5 in both the editor and runtime.

## Implemented and tested

| Feature | Evidence |
|---|---|
| Hardware/CPU/memory/thermal/DRM probe | `cargo run -p vela-cli -- hardware --backend vulkan` |
| Vulkan adapter selection through a real surface | V3D 7.1.7.0 selected with wgpu on the Pi 5 |
| Project create/open/inspect and sandbox rejection | CLI, editor, and MCP tests |
| Vela project file compatibility | No format change; `project.vela.json` and `*.velascene` still open |
| Scene serialization, hierarchy, and starter scene | CLI inspect, editor, MCP write test, scene unit tests |
| CUBES user-facing identity | Window title, menus, CLI help, runtime title, MCP descriptions, docs |
| 3D cube logo and desktop icons | `desktop/cubes-dark.svg`, `cubes-light.svg`, generated PNG icons |
| CUBES desktop launcher with legacy Vela alias | Installer creates both entries; CUBES entry validated and launched |
| First-run welcome screen | Real X11 screenshot: `docs/screenshots/cubes-welcome.png` |
| Viewport object picking | Unit test ray-selects the starter cube through the viewport center |
| Translation gizmo | Unit test moves selection; real screenshot: `docs/screenshots/cubes-gizmo.png` |
| Rotation and scale gizmos | Unit tests and real screenshots: `cubes-rotate-gizmo.png`, `cubes-scale-gizmo.png` |
| Undo/redo history | Unit test restores scene after add; toolbar/menu and Ctrl+Z/Ctrl+Y/Ctrl+Shift+Z |
| Inspector editing and selection highlight | Live screenshot and editor unit tests |
| Project open/save in editor | `Project` paths plus atomic scene writer |
| Play preview with scene restore | Editor unit test |
| Mouse orbit vertical direction | Regression test after user feedback |
| Standalone runtime with Lua and simple dynamics | 12-second Pi 5 run at about 57–59 FPS |
| Lua `on_start`/`on_update` | Scripting unit test |
| MCP tools/resources/prompts over stdio | Real child-process protocol test |
| MCP project sandbox and traversal rejection | Same stdio protocol test |
| Asset checksum and copy into project | Asset unit test |
| glTF 2.0 mesh import and GPU upload | Renderer unit test loads a minimal glTF triangle |
| Imported mesh rendering in editor | Live screenshot: `docs/screenshots/cubes-imported-mesh-editor.png` |
| Imported mesh rendering in runtime | Live screenshot and 6-triangle draw report: `cubes-imported-mesh-runtime.png` |
| Mesh asset field and import auto-assignment | Inspector UI and editor state path |
| UV-aware meshes and texture sampling | Renderer mesh/shader changes and unit tests |
| glTF base-colour texture import | Renderer test loads an external PNG texture |
| GPU texture upload and material texture path | Editor screenshot: `docs/screenshots/cubes-textured-mesh.png` |
| Textured mesh in runtime | Runtime screenshot and ~58–59 FPS report: `cubes-textured-mesh-runtime.png` |
| Material texture field in Inspector | Inspector UI writes `base_color_texture` |
| Compact ECS storage and generation reuse | ECS unit tests |
| Repeatable offscreen Vulkan benchmark | `docs/benchmarks/pi5-vulkan.json` |

## Implemented but not fully tested

| Feature | Limit |
|---|---|
| Instanced PBR-style renderer with primitives, imported glTF geometry, grid, lighting, texture sampling, and metrics | No shadows, normal maps, or advanced PBR maps yet |
| Resizable editor panels | Not true drag-docking; beginner/advanced modes not yet added |
| Profiler panel | Rolling frame times and draw metrics, not GPU timestamp queries |
| Help and About dialogs | In-app text only; no guided tutorial system yet |
| Asset classification, checksum, and copy | glTF geometry and base-colour textures upload; audio and animation files are only stored/listed |
| Lua scripting runtime | Only `on_start` and `on_update` are exposed |
| MCP tools with structured schemas | No component/material mutation tools or rollback history yet |

## In progress

| Feature | Current state |
|---|---|
| Transform gizmos | Move, rotate, and scale are implemented; snapping and multi-axis handles remain |
| Asset pipeline | glTF geometry and base-colour textures work; normal/ORM maps, skins, animations, and import reports remain |
| Beginner templates | Only Basic 3D Starter is selectable; eight gameplay templates are planned |
| Physics | Simple dynamic floor solver; no arbitrary colliders or body-to-body interaction |
| Material editor | Base color/metallic/roughness only; no texture slots or node editor |
| Cross-platform packaging | Pi 5 ARM64 only; other desktop targets not built or tested |

## Planned

First-person/third-person/platformer/racing/puzzle/survival/exploration/top-down templates; visual
scripting; animation timeline, dope sheet, curves, clips, and skeletal animation; 3D modelling;
cinematics; glTF/GLB materials, textures, skins, animation interchange, and FBX; Roblox export
assistant; Blender workflow; advanced PBR maps and texture compression; shadows; ambient occlusion; post-processing;
ray-tracing capability tiers; audio; particles;
navigation; UI authoring; input mapping; multi-selection; true docking; standalone game
export; C++/Rust plugin SDK; additional SBC support; fair Godot comparisons.

## Unsupported

- No second SBC has been tested.
- No shadow, audio, animation, terrain, skeletal mesh, normal/ORM map, or glTF animation rendering
  exists.
- No Roblox or Blender export exists.
- No production build/export or cross-platform package exists.
- No GPU timestamp profiling or dynamic quality scaling exists.
- No custom hardware tweaks or overclocking are applied.
- No benchmark number from an unrun configuration is reported.
