use anyhow::{Context, Result};
use rmcp::ServiceExt;
use std::path::PathBuf;
use vela_mcp::VelaMcpServer;

#[tokio::main]
async fn main() -> Result<()> {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let root = root
        .canonicalize()
        .context("MCP workspace root does not exist")?;
    let server = VelaMcpServer::new(root);
    let service = server
        .serve(rmcp::transport::stdio())
        .await
        .context("MCP stdio service failed to start")?;
    service
        .waiting()
        .await
        .context("MCP stdio service failed")?;
    Ok(())
}
