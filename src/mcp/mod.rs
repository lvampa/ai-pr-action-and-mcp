use crate::{config::Config, reviewer};
use anyhow::Result;
use rmcp::{
    model::{Implementation, ProtocolVersion, ServerCapabilities, ServerInfo},
    tool, ServerHandler, ServiceExt,
    transport::stdio,
    Error as McpError,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct ReviewServer {
    config: Arc<Config>,
}

impl ReviewServer {
    fn new(config: Config) -> Self {
        Self { config: Arc::new(config) }
    }
}

#[tool(tool_box)]
impl ReviewServer {
    #[tool(description = "Review a GitHub pull request and return the review as markdown")]
    async fn review_pr(
        &self,
        #[tool(description = "Repository in owner/repo format, e.g. octocat/hello-world")]
        repo: String,
        #[tool(description = "Pull request number")]
        pr_number: u64,
    ) -> Result<String, McpError> {
        reviewer::review_pr(&self.config, &repo, pr_number)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))
    }
}

impl ServerHandler for ReviewServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: env!("CARGO_PKG_NAME").into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
            instructions: Some("AI-powered GitHub pull request reviewer".into()),
        }
    }
}

pub async fn serve(config: Config) -> Result<()> {
    tracing::info!("Starting MCP server over stdio");
    let service = ReviewServer::new(config)
        .serve(stdio())
        .await
        .map_err(|e| anyhow::anyhow!("MCP transport error: {e}"))?;
    service
        .waiting()
        .await
        .map_err(|e| anyhow::anyhow!("MCP server error: {e}"))?;
    Ok(())
}
