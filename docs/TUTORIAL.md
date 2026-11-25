# Tutorial: build and play a small scene

## 1. Create a project

```bash
. "$HOME/.cargo/env"
cargo run -p vela-cli -- new-project /tmp/vela-demo "CUBES Demo"
cargo run -p vela-cli -- inspect-project /tmp/vela-demo
```

The starter scene has a camera, a sun light, a blue cube with a physics body, and a ground plane.

## 2. Edit it

For a double-clickable desktop app, run `desktop/install-desktop-app.sh` once and find **CUBES
Editor** in your application menu. It opens the starter scene; pass `--project /path/to/project` via
`~/.local/bin/cubes-editor-desktop` to open a specific project.

From a terminal:

```bash
cargo run -p vela-editor -- --project /tmp/vela-demo
```

- Click an entity in the left hierarchy or click an object in the viewport to select it. The selected
  mesh is highlighted and its data appears in the Inspector.
- Use Move, Rotate, or Scale in the top bar. Drag the red X, green Y, or blue Z gizmo arrow or ring
  in the viewport to transform the selection. Drag position/rotation/scale values in the Inspector
  for precise numbers.
- Undo with Ctrl+Z or Edit > Undo. Redo with Ctrl+Y, Ctrl+Shift+Z, or Edit > Redo.
- Use the Mesh dropdown to switch cube, sphere, plane, or cylinder.
- Use the Material color picker and sliders to edit base color, metallic, and roughness.
- Use Add in the hierarchy to create an empty object, cube, plane, camera, or light.
- Right-drag in the viewport to pan; drag to orbit; scroll to zoom. `F` focuses the selection and
  `Delete` removes it.
- The bottom tabs show Assets, Console, and Profiler. The profiler reports actual FPS, frame time,
  draw calls, triangles, instances, and graphics adapter.
- `Ctrl+S` or File → Save scene writes `scenes/main.velascene` atomically. Saving is disabled while
  play preview is active.

## 3. Add a script

Create `/tmp/vela-demo/scripts/player.lua`:

```lua
function on_start()
    print("CUBES demo started")
end

function on_update(entity, delta)
    _G.last_delta = delta
end
```

Run the runtime from the project directory or pass the path:

```bash
cargo run -p vela-runtime -- /tmp/vela-demo
```

The runtime loads sorted `scripts/*.lua`, calls `on_start`, calls `on_update` for each entity, and
logs measured FPS every two seconds. The physics cube falls and settles on the starter ground.

## 4. Import an asset

Type an absolute file path into the editor's Assets → Import path field and press Import. Supported
extensions are `png`, `jpg`, `jpeg`, `webp`, `ktx2`, `gltf`, `glb`, `obj`, `wav`, `ogg`, `mp3`,
`velascene`, and `lua`. The file is checksummed and copied into `assets/`.

For `gltf` and `glb`, the first mesh primitive is loaded to the GPU and rendered. Importing a model
while an object is selected assigns the new asset to that object's Mesh component. You can also type
a project-relative path such as `assets/model.gltf` into the Mesh → Asset field in the Inspector.
glTF geometry and base-colour textures are applied automatically. The Inspector also has a Material
→ Texture field for project-relative texture paths. glTF skins, animations, normal maps, and ORM maps
are not imported yet. Audio files are stored and listed but not yet used by the runtime.

## 5. Use MCP

Start the server with the workspace root that may contain projects:

```bash
cargo run -p vela-mcp -- /tmp
```

Ask an MCP client to call `vela_project_info` with `project: /tmp/vela-demo`, then
`vela_list_entities`, `vela_create_entity`, or `vela_read_asset`. See [MCP.md](MCP.md).
