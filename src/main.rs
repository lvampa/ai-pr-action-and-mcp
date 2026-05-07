mod config;
mod github;
mod mcp;
mod models;
mod reviewer;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "ai-cr-action", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Review a pull request and post a comment (GitHub Actions mode)
    Review,
    /// Start the MCP server over stdio
    Serve,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let config = config::Config::load()?;

    match cli.command {
        Command::Review => reviewer::run(config).await,
        Command::Serve => mcp::serve(config).await,
    }
}
