# Architecture

CUBES is a Rust workspace rather than a single crate. The split keeps the editor, runtime, tools,
and reusable engine systems separate while allowing one release profile for the whole project.

## Data flow

1. `vela-core` opens `project.vela.json`, canonicalizes the root, and resolves every project path
   through validation that rejects absolute paths, `..`, and symlink escapes.
2. `vela-scene` deserializes `scenes/main.velascene` into entities, transforms, and components.
3. The editor or runtime walks the scene graph, computes `world_transform`, and builds
   `RenderInstance` values grouped by primitive in `vela-renderer`.
4. `SceneRenderer` uploads one instance buffer and issues one indexed draw per present primitive
   kind plus a grid draw. This keeps draw calls bounded independently of instance count.
5. WGSL computes a simple GGX-style direct light, ambient term, gamma correction, and selection
   emissive in the editor. There are no shadow passes yet.

## Editor

`apps/vela-editor` uses winit for the window and event loop, wgpu for the surface, and egui for
panels. The 3D scene is rendered into the central viewport scissor rectangle before egui draws its
panels over the rest of the surface. egui output is tessellated and submitted in a second render
pass. Play mode snapshots the scene and restores it on stop, so preview motion cannot silently
alter the saved scene.

Editor state is intentionally plain Rust (`state.rs`) so it can be unit tested without a window.
The UI reads snapshots from it and writes back only on actual widget changes. Viewport picking casts
a ray from the orbit camera through the pointer and tests visible mesh bounds in local space; the
transform gizmos project X/Y/Z axes into screen space and convert pointer drag pixels into move,
rotate, or scale deltas. Undo/redo keeps bounded scene snapshots; discrete edits checkpoint before
mutation and gizmo drags checkpoint once on drag start. First launch shows a welcome window,
tracked by a small marker under the user config directory.

## Runtime

`apps/vela-runtime` opens a project directly, loads all `scripts/*.lua`, calls `on_start`, then calls
`on_update(entity, delta)` for every entity while rendering on wgpu. Dynamic physics bodies use a
small semi-implicit Euler integrator with floor collision, friction, and restitution. Live FPS and
draw metrics are logged every two seconds.

## MCP

`vela-mcp` implements `rmcp::ServerHandler` over stdio. It exposes tools, resources, and prompts and
never exposes shell execution. Project tools require a project path inside the MCP workspace root,
then use `vela-core` path validation for every scene or asset access.

## ECS

`vela-ecs` stores components in dense type-specific arrays with sparse entity lookup and generation
reuse. It is tested independently. The current editor/runtime scene path still uses the
serializable scene graph directly; the ECS is available for gameplay systems but is not yet the
authoritative runtime world.

## Asset geometry pipeline

`vela-renderer::load_gltf_mesh` imports the first usable glTF 2.0 primitive, bakes the first node
transform into vertex data, and returns an `ImportedMesh` with geometry, base colour, and an optional
decoded texture. `SceneRenderer::load_asset_mesh` and `load_texture` upload resources under
project-relative keys. Vertices carry UVs and the WGSL shader samples a material texture through a
second bind group; untextured batches use a white default texture. `RenderInstance` carries optional
mesh and texture paths, so batches are grouped by both. Editor and runtime resolve asset paths
through `ProjectPaths`, persist embedded images as PNG files, and load each unique resource once.
 glTF skins, animation tracks, normal maps, and ORM maps are not yet applied.

## Deliberate non-goals so far

No Bevy dependency, no physics middleware, no asset loader threads, no shadow maps, no audio device,
no animation system, and no build/export pipeline. These are listed as planned or in development in
[STATUS.md](STATUS.md) rather than presented as working features.
