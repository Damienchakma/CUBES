# Hardware verification

All commands below were run on the development machine, not inferred from documentation.

## Verified machine

- Board: Raspberry Pi 5 Model B Rev 1.0
- SoC: Broadcom BCM2712, four Cortex-A76 cores
- RAM: 8 GiB
- OS: Kali GNU/Linux Rolling 2026.2
- Kernel: `6.12.34+rpt-rpi-2712`
- Render node: `/dev/dri/renderD128`
- Vulkan ICD: Broadcom V3D (`/usr/share/vulkan/icd.d/broadcom_icd.json`)
- wgpu adapter: `V3D 7.1.7.0`, `V3DV Mesa`, vendor `0x14e4`, device `0x55701c33`, backend Vulkan

## Verification commands

```bash
. "$HOME/.cargo/env"
cargo run -p vela-cli -- hardware --backend vulkan
cargo run -p vela-editor -- --project /tmp/my-game
cargo run -p vela-runtime -- /tmp/my-game
cargo run --release -p vela-bench -- --backend vulkan
```

`vela hardware` reads `/proc/device-tree`, `/proc/cpuinfo`, `/proc/meminfo`, `/sys/class/drm`, the
Vulkan ICD directory, and `/sys/class/thermal`. It reports NEON-related CPU flags, available memory,
DRM render node, preferred API, temperature, trip points, cooling-device state, and a quality
recommendation. Pi 5 and Compute Module 5 map to `balanced`; machines with at least 12 GiB map to
`quality`; smaller systems map to `performance`.

Thermal safety is observational: temperatures at or above 80 °C set `throttlingDetected` and a
warning. The engine does not force a lower clock or change system settings.

## Capability-detected backends

The renderer requests an adapter compatible with the actual window surface. Vulkan is preferred;
OpenGL is a fallback. The OpenGL EGL path produced context-creation warnings from wgpu's probe, while
Vulkan selected the V3D adapter successfully. No OpenGL performance result is claimed.

## Not verified

A second single-board computer was not available, so multi-SBC compatibility is **not verified**.
The code path is written generically around wgpu and `/proc`, but no other device result is claimed.
