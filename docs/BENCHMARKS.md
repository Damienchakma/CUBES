# Benchmarks

## Method

`vela-bench` creates deterministic instance matrices (LCG seed `0x9e3779b97f4a7c15`) across four
primitive kinds, renders to an offscreen 1280×720 RGBA8 texture with depth, waits for GPU completion
after every measured frame, and reports wall-clock CPU encoding plus GPU completion time. This is
throughput under backpressure, not a vsynced presentation FPS and not a pure GPU timestamp query.

```bash
cargo run --release -p vela-bench -- \
  --width 1280 --height 720 --instances 512 \
  --frames 300 --warmup 30 --backend vulkan \
  --output docs/benchmarks/pi5-vulkan.json
```

## Measured result on the development Pi 5

From [pi5-vulkan.json](benchmarks/pi5-vulkan.json):

| Metric | Value |
|---|---:|
| Adapter | V3D 7.1.7.0, Vulkan, V3DV Mesa |
| Resolution | 1280 × 720 |
| Instances | 512 |
| Measured frames | 300 after 30 warmup frames |
| Average frame time | 39.51 ms |
| Throughput | 25.31 FPS |
| p95 frame time | 78.05 ms |
| Draw calls | 5 |
| Triangles | 50,944 |
| Thermal state at run | 82.1 °C, throttling detected |

The thermal warning is part of the result. The run was not cooled or repeated until a colder number
appeared. The p95 is far above the mean, so this is not a stable 25 FPS frame budget; sustained load
should use a lower instance count or reduced resolution on this board.

Separately, the two-instance starter scene in the windowed runtime reported about 57–59 FPS over
12-second runs. That observation is not a formal benchmark and is not substituted for the table
above.
