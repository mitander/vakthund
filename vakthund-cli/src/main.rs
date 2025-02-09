pub mod commands;

use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = commands::Cli::parse();
    commands::run_command(cli).await
}
