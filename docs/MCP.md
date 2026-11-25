# MCP interface

`vela-mcp` is a stdio server built on `rmcp` 3.11.0. Its single argument is the workspace root that
project paths must live under. It writes protocol messages to stdout only; diagnostics go to stderr.

The `vela_` tool names, `vela://` resource URIs, and `project.vela.json` format are deliberately
kept for backward compatibility with existing MCP clients and projects.

```json
{
  "mcpServers": {
    "vela": {
      "command": "/path/to/target/debug/vela-mcp",
      "args": ["/absolute/workspace/root"]
    }
  }
}
```

## Tools

| Tool | Arguments | Purpose |
|---|---|---|
| `vela_hardware` | none | Live CPU/memory/thermal/graphics probe |
| `vela_project_info` | `project` | Manifest and scene summary |
| `vela_list_entities` | `project` | Entity ids, names, parents, visibility, components |
| `vela_get_entity` | `project`, `entityId` | One full entity record |
| `vela_list_assets` | `project` | Files under the project's assets directory |
| `vela_read_asset` | `project`, `path` | UTF-8 asset text, limited to 64 KiB |
| `vela_create_entity` | `project`, `name` | Create and atomically save a new empty entity |

## Resources

- `vela://hardware` — JSON hardware report
- `vela://status` — concise implementation status
- `vela://schema/project` — project directory and file layout

## Prompt

`build-scene` accepts an optional `goal` argument and returns instructions that use only the CUBES
tools above.

## Sandbox rules

- There is no shell, command, code-execution, or arbitrary-path tool.
- A project path must canonicalize inside the workspace root.
- Scene and asset paths are resolved through `ProjectPaths`, which rejects absolute paths, `..`,
  and symlink escapes.
- Asset reads are restricted to the canonical assets directory and capped in size.

`cargo test -p vela-mcp` starts the real binary over stdio and verifies tool/resource/prompt listing,
a sandboxed write, and rejection of a traversal attempt.
