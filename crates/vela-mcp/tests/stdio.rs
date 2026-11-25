use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use vela_core::Project;
use vela_scene::Scene;

struct Client {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
    next_id: u64,
}

impl Client {
    fn start(root: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_vela-mcp"))
            .arg(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("vela-mcp binary should start");
        let stdin = child.stdin.take().expect("stdin");
        let stdout = BufReader::new(child.stdout.take().expect("stdout"));
        let mut client = Self {
            child,
            stdin,
            stdout,
            next_id: 0,
        };
        client.request(
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "vela-test", "version": "0.1.0" }
            }),
        );
        client.notify("notifications/initialized", json!({}));
        client
    }

    fn next_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    fn notify(&mut self, method: &str, params: Value) {
        let message = json!({ "jsonrpc": "2.0", "method": method, "params": params });
        writeln!(self.stdin, "{message}").expect("write request");
        self.stdin.flush().expect("flush request");
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id();
        let message = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        writeln!(self.stdin, "{message}").expect("write request");
        self.stdin.flush().expect("flush request");
        loop {
            let mut line = String::new();
            self.stdout
                .read_line(&mut line)
                .expect("read response line");
            assert!(!line.is_empty(), "MCP server closed stdout");
            let Ok(response) = serde_json::from_str::<Value>(line.trim()) else {
                continue;
            };
            if response.get("id").and_then(Value::as_u64) == Some(id) {
                return response;
            }
        }
    }

    fn tool_text(&mut self, name: &str, arguments: Value) -> String {
        let response = self.request(
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
        );
        assert!(response.get("error").is_none(), "tool failed: {response}");
        response["result"]["content"][0]["text"]
            .as_str()
            .expect("tool text")
            .to_owned()
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn stdio_server_exposes_tools_resources_prompts_and_sandbox() {
    let root = std::env::temp_dir().join(format!(
        "vela-mcp-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let project = Project::create(&root, "MCP Test").expect("project");
    let scene_path = project
        .paths()
        .resolve_for_write(&project.manifest().main_scene)
        .expect("scene path");
    Scene::starter_scene().save(scene_path).expect("scene");

    let mut client = Client::start(&root);
    let tools = client.request("tools/list", json!({}));
    let names = tools["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .map(|tool| tool["name"].as_str().unwrap_or_default())
        .collect::<Vec<_>>();
    assert!(names.contains(&"vela_project_info"));
    assert!(names.contains(&"vela_create_entity"));
    assert!(!names.iter().any(|name| name.contains("shell")));

    let info = client.tool_text(
        "vela_project_info",
        json!({ "project": root.to_string_lossy() }),
    );
    assert!(info.contains("MCP Test"));

    let resources = client.request("resources/list", json!({}));
    assert!(
        resources["result"]["resources"]
            .as_array()
            .is_some_and(|items| { items.iter().any(|item| item["uri"] == "vela://hardware") })
    );

    let status = client.request("resources/read", json!({ "uri": "vela://status" }));
    assert!(
        status["result"]["contents"][0]["text"]
            .as_str()
            .is_some_and(|text| text.contains("CUBES"))
    );

    let prompts = client.request("prompts/list", json!({}));
    assert!(
        prompts["result"]["prompts"]
            .as_array()
            .is_some_and(|items| { items.iter().any(|item| item["name"] == "build-scene") })
    );

    let prompt = client.request(
        "prompts/get",
        json!({ "name": "build-scene", "arguments": { "goal": "a test scene" } }),
    );
    assert!(
        serde_json::to_string(&prompt)
            .unwrap_or_default()
            .contains("a test scene")
    );

    let traversal = client.request(
        "tools/call",
        json!({
            "name": "vela_read_asset",
            "arguments": { "project": root.to_string_lossy(), "path": "../outside.txt" }
        }),
    );
    assert!(traversal.get("error").is_some());

    let before = client.tool_text(
        "vela_list_entities",
        json!({ "project": root.to_string_lossy() }),
    );
    let before_count: Value = serde_json::from_str(&before).expect("entity list");
    let before_count = before_count["entities"].as_array().unwrap().len();

    let created = client.tool_text(
        "vela_create_entity",
        json!({ "project": root.to_string_lossy(), "name": "MCP Cube" }),
    );
    assert!(created.contains("MCP Cube"));

    let after = client.tool_text(
        "vela_list_entities",
        json!({ "project": root.to_string_lossy() }),
    );
    let after: Value = serde_json::from_str(&after).expect("entity list");
    assert_eq!(
        after["entities"].as_array().unwrap().len(),
        before_count + 1
    );

    std::fs::remove_dir_all(&root).expect("cleanup");
}
