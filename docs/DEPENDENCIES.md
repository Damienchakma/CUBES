# Dependencies

Pinned workspace versions. `Cargo.lock` records the exact resolved graph.

| Crate | Version | Used for |
|---|---:|---|
| `wgpu` | 30.0.2 | Vulkan/OpenGL compute and rendering through one API |
| `winit` | 0.30.12 (resolves to 0.30.13) | Cross-platform window and event loop |
| `egui`, `egui-wgpu`, `egui-winit` | 0.37.0 | Editor panels and overlay rendering |
| `glam` | 0.34.3 | SIMD-friendly math and serde support |
| `serde`, `serde_json` | 1.0 | Scene/project formats and MCP payloads |
| `rmcp` | 3.11.0 | Maintained Rust MCP SDK with server and stdio transport |
| `mlua` | 0.12.3 | Embedded Lua 5.4 with vendored source |
| `clap` | 4.6.9 | CLI argument parsing |
| `sha2` | 0.10.9 | Asset checksum |
| `pollster` | 1.0.3 | Blocking on async GPU initialization in desktop apps |
| `tokio` | 1.48+ | MCP stdio async runtime |
| `anyhow`, `thiserror` | 1.0, 2.0 | Error handling |

`egui` enables `default_fonts`; without it the Pi build showed tofu boxes for every label. Vulkan is
requested first, OpenGL second. Adapter selection uses `compatible_surface` for windowed apps so the
chosen device can actually present to the surface.

The workspace release profile uses `opt-level = 3`, thin LTO, one codegen unit, panic abort, and
symbol stripping to keep aarch64 binaries small.
