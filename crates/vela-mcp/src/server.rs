use rmcp::{
    ErrorData, ServerHandler,
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock,
        GetPromptRequestParams, GetPromptResponse, GetPromptResult, ListPromptsResult,
        ListResourcesResult, ListToolsResult, PaginatedRequestParams, Prompt, PromptMessage,
        ReadResourceRequestParams, ReadResourceResponse, ReadResourceResult, Resource,
        ResourceContents, Role, ServerCapabilities, ServerConfig, Tool,
    },
    service::{RequestContext, RoleServer},
};
use serde_json::{Map, Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use vela_core::{EntityId, Project};
use vela_scene::Scene;

const MAX_ASSET_FILES: usize = 500;
const MAX_ASSET_BYTES: usize = 65_536;

#[derive(Clone, Debug)]
pub struct VelaMcpServer {
    workspace_root: PathBuf,
}

impl VelaMcpServer {
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
        }
    }

    pub fn workspace_root(&self) -> &std::path::Path {
        &self.workspace_root
    }

    fn project_path(&self, arguments: &Map<String, Value>) -> Result<PathBuf, ErrorData> {
        let supplied = string_argument(arguments, "project")?;
        let candidate = PathBuf::from(supplied);
        let canonical = candidate.canonicalize().map_err(|error| {
            ErrorData::invalid_params(format!("invalid project path: {error}"), None)
        })?;
        let workspace = self
            .workspace_root
            .canonicalize()
            .unwrap_or_else(|_| self.workspace_root.clone());
        if !canonical.starts_with(&workspace) {
            return Err(ErrorData::invalid_params(
                "project is outside the MCP workspace root",
                None,
            ));
        }
        Ok(canonical)
    }

    fn project(&self, arguments: &Map<String, Value>) -> Result<Project, ErrorData> {
        let root = self.project_path(arguments)?;
        Project::open(&root).map_err(|error| ErrorData::invalid_params(error.to_string(), None))
    }
}

impl ServerHandler for VelaMcpServer {
    #[allow(deprecated)]
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_prompts()
                .enable_resources()
                .build(),
        )
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let project = json!({
            "type": "object",
            "properties": { "project": { "type": "string", "description": "CUBES project directory" } },
            "required": ["project"]
        });
        let tools = vec![
            Tool::new(
                "vela_hardware",
                "Read the host CPU, memory, thermal, and graphics capability report.",
                schema(json!({ "type": "object", "properties": {} })),
            ),
            Tool::new(
                "vela_project_info",
                "Read a sandboxed CUBES project manifest and scene summary.",
                schema(project.clone()),
            ),
            Tool::new(
                "vela_list_entities",
                "List every entity in a sandboxed project's main scene.",
                schema(project.clone()),
            ),
            Tool::new(
                "vela_get_entity",
                "Read one entity record by numeric id.",
                schema(json!({
                    "type": "object",
                    "properties": {
                        "project": { "type": "string" },
                        "entityId": { "type": "integer", "minimum": 1 }
                    },
                    "required": ["project", "entityId"]
                })),
            ),
            Tool::new(
                "vela_list_assets",
                "List files inside a project's assets directory without following symlinks out of it.",
                schema(project.clone()),
            ),
            Tool::new(
                "vela_read_asset",
                "Read a UTF-8 text asset from inside a project's assets directory.",
                schema(json!({
                    "type": "object",
                    "properties": {
                        "project": { "type": "string" },
                        "path": { "type": "string", "description": "Project-relative path under assets" }
                    },
                    "required": ["project", "path"]
                })),
            ),
            Tool::new(
                "vela_create_entity",
                "Create and save a new empty entity in a project's main scene.",
                schema(json!({
                    "type": "object",
                    "properties": {
                        "project": { "type": "string" },
                        "name": { "type": "string", "minLength": 1 }
                    },
                    "required": ["project", "name"]
                })),
            ),
        ];
        Ok(ListToolsResult::with_all_items(tools))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let arguments = request.arguments.as_ref();
        let value = match request.name.as_ref() {
            "vela_hardware" => {
                let report = vela_platform::probe().map_err(|error| {
                    ErrorData::internal_error(format!("hardware probe failed: {error}"), None)
                })?;
                serde_json::to_value(report).map_err(json_error)?
            }
            "vela_project_info" => {
                let arguments = require_arguments(arguments)?;
                let project = self.project(arguments)?;
                let scene_path = project
                    .paths()
                    .resolve_existing(&project.manifest().main_scene)
                    .map_err(|error| ErrorData::invalid_params(error.to_string(), None))?;
                let scene = Scene::load(scene_path)
                    .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
                json!({
                    "manifest": project.manifest(),
                    "entityCount": scene.entities.len(),
                    "activeEntities": scene.entities.values().filter(|entity| entity.visible).count(),
                })
            }
            "vela_list_entities" => {
                let arguments = require_arguments(arguments)?;
                let project = self.project(arguments)?;
                let scene_path = project
                    .paths()
                    .resolve_existing(&project.manifest().main_scene)
                    .map_err(|error| ErrorData::invalid_params(error.to_string(), None))?;
                let scene = Scene::load(scene_path)
                    .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
                let entities = scene
                    .entities
                    .values()
                    .map(|entity| {
                        json!({
                            "id": entity.id.0,
                            "name": entity.name,
                            "parent": entity.parent.map(|parent| parent.0),
                            "visible": entity.visible,
                            "components": entity.components.keys().map(|kind| kind.as_str()).collect::<Vec<_>>(),
                        })
                    })
                    .collect::<Vec<_>>();
                json!({ "entities": entities })
            }
            "vela_get_entity" => {
                let arguments = require_arguments(arguments)?;
                let project = self.project(arguments)?;
                let id = u64_argument(arguments, "entityId")?;
                let scene_path = project
                    .paths()
                    .resolve_existing(&project.manifest().main_scene)
                    .map_err(|error| ErrorData::invalid_params(error.to_string(), None))?;
                let scene = Scene::load(scene_path)
                    .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
                let entity = scene
                    .entities
                    .get(&EntityId(id))
                    .ok_or_else(|| ErrorData::invalid_params("entity does not exist", None))?;
                serde_json::to_value(entity).map_err(json_error)?
            }
            "vela_list_assets" => {
                let arguments = require_arguments(arguments)?;
                let project = self.project(arguments)?;
                let assets = project
                    .paths()
                    .assets_dir()
                    .canonicalize()
                    .map_err(|error| ErrorData::invalid_params(error.to_string(), None))?;
                let mut paths = Vec::new();
                collect_assets(&assets, &assets, &mut paths, 0);
                paths.sort();
                json!({ "assets": paths })
            }
            "vela_read_asset" => {
                let arguments = require_arguments(arguments)?;
                let project = self.project(arguments)?;
                let relative = string_argument(arguments, "path")?;
                let path = project
                    .paths()
                    .resolve_existing(Path::new("assets").join(relative))
                    .map_err(|error| ErrorData::invalid_params(error.to_string(), None))?;
                let assets = project
                    .paths()
                    .assets_dir()
                    .canonicalize()
                    .map_err(|error| ErrorData::invalid_params(error.to_string(), None))?;
                if !path.starts_with(assets) {
                    return Err(ErrorData::invalid_params(
                        "asset path is outside the project assets directory",
                        None,
                    ));
                }
                let bytes = std::fs::read(&path)
                    .map_err(|error| ErrorData::invalid_params(error.to_string(), None))?;
                if bytes.len() > MAX_ASSET_BYTES {
                    return Err(ErrorData::invalid_params(
                        format!("asset is larger than {MAX_ASSET_BYTES} bytes"),
                        None,
                    ));
                }
                let text = String::from_utf8(bytes).map_err(|error| {
                    ErrorData::invalid_params(format!("asset is not UTF-8: {error}"), None)
                })?;
                json!({ "path": relative, "contents": text })
            }
            "vela_create_entity" => {
                let arguments = require_arguments(arguments)?;
                let project = self.project(arguments)?;
                let name = string_argument(arguments, "name")?;
                if name.trim().is_empty() {
                    return Err(ErrorData::invalid_params("name cannot be empty", None));
                }
                let scene_path = project
                    .paths()
                    .resolve_existing(&project.manifest().main_scene)
                    .map_err(|error| ErrorData::invalid_params(error.to_string(), None))?;
                let mut scene = Scene::load(scene_path)
                    .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
                let id = scene.create_entity(name);
                scene
                    .save(
                        project
                            .paths()
                            .resolve_for_write(&project.manifest().main_scene)
                            .map_err(|error| ErrorData::invalid_params(error.to_string(), None))?,
                    )
                    .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
                json!({ "id": id.0, "name": name })
            }
            other => {
                return Err(ErrorData::invalid_params(
                    format!("unknown tool: {other}"),
                    None,
                ));
            }
        };
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&value).map_err(json_error)?,
        )])
        .into())
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(ListResourcesResult::with_all_items(vec![
            Resource::new("vela://hardware", "Hardware Report")
                .with_description("Live CPU, memory, thermal, and graphics capability report"),
            Resource::new("vela://status", "Engine Status")
                .with_description("Honest implementation status of the CUBES engine"),
            Resource::new("vela://schema/project", "Project Schema")
                .with_description("CUBES project directory and file layout"),
        ]))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        let contents = match request.uri.as_ref() {
            "vela://hardware" => {
                let report = vela_platform::probe().map_err(|error| {
                    ErrorData::internal_error(format!("hardware probe failed: {error}"), None)
                })?;
                serde_json::to_string_pretty(&report).map_err(json_error)?
            }
            "vela://status" => serde_json::to_string_pretty(&json!({
                "engine": "CUBES",
                "status": "development",
                "implemented": ["editor", "hardware probe", "scene format", "reasoned renderer", "Lua runtime", "asset checksums", "MCP stdio server"],
                "notVerified": ["multi-SBC compatibility", "production export", "full physics"],
            }))
            .map_err(json_error)?,
            "vela://schema/project" => serde_json::to_string_pretty(&json!({
                "manifest": "project.vela.json",
                "scene": "scenes/main.velascene",
                "directories": ["assets", "scripts", "builds", ".vela"],
            }))
            .map_err(json_error)?,
            other => {
                return Err(ErrorData::invalid_params(
                    format!("unknown resource: {other}"),
                    None,
                ));
            }
        };
        Ok(ReadResourceResult::new(vec![ResourceContents::text(contents, request.uri)]).into())
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, ErrorData> {
        Ok(ListPromptsResult::with_all_items(vec![Prompt::new(
            "build-scene",
            Some("Guide an agent through building a CUBES scene with sandboxed MCP tools"),
            None,
        )]))
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResponse, ErrorData> {
        if request.name.as_str() != "build-scene" {
            return Err(ErrorData::invalid_params(
                format!("unknown prompt: {}", request.name),
                None,
            ));
        }
        let goal = request
            .arguments
            .as_ref()
            .and_then(|arguments| arguments.get("goal"))
            .and_then(Value::as_str)
            .unwrap_or("a small playable scene");
        Ok(GetPromptResult::new(vec![
            PromptMessage::new_text(
                Role::User,
                format!("Build {goal} in a CUBES project using only the CUBES MCP tools."),
            ),
            PromptMessage::new_text(
                Role::Assistant,
                "Call vela_project_info to verify the project, vela_list_entities to inspect the scene, vela_create_entity for each new object, and vela_list_assets before referencing assets. Never request shell access and never write outside the project root.",
            ),
        ])
        .into())
    }
}

fn schema(value: Value) -> Arc<Map<String, Value>> {
    Arc::new(value.as_object().cloned().unwrap_or_default())
}

fn require_arguments(
    arguments: Option<&Map<String, Value>>,
) -> Result<&Map<String, Value>, ErrorData> {
    arguments.ok_or_else(|| ErrorData::invalid_params("tool arguments are required", None))
}

fn string_argument<'a>(
    arguments: &'a Map<String, Value>,
    name: &str,
) -> Result<&'a str, ErrorData> {
    arguments
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| ErrorData::invalid_params(format!("missing or invalid '{name}'"), None))
}

fn u64_argument(arguments: &Map<String, Value>, name: &str) -> Result<u64, ErrorData> {
    arguments
        .get(name)
        .and_then(Value::as_u64)
        .ok_or_else(|| ErrorData::invalid_params(format!("missing or invalid '{name}'"), None))
}

fn json_error(error: serde_json::Error) -> ErrorData {
    ErrorData::internal_error(format!("serialization failed: {error}"), None)
}

fn collect_assets(root: &Path, directory: &Path, output: &mut Vec<String>, depth: usize) {
    if depth > 8 || output.len() >= MAX_ASSET_FILES {
        return;
    }
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_assets(root, &path, output, depth + 1);
            continue;
        }
        let Ok(canonical) = path.canonicalize() else {
            continue;
        };
        if canonical.starts_with(root)
            && let Ok(relative) = canonical.strip_prefix(root)
        {
            output.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
}
