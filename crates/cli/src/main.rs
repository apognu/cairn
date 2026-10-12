//! The yourdocs binary.

#![forbid(unsafe_code)]

use std::{net::SocketAddr, path::PathBuf};

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "yourdocs", version)]
struct Cli {
  #[command(subcommand)]
  command: Command,
}

#[derive(Subcommand)]
enum Command {
  /// Run the editor locally with no login. Listens on localhost only.
  Dev {
    #[arg(long, default_value = "127.0.0.1:3000")]
    bind: SocketAddr,
    /// Where the database and uploads live.
    #[arg(long, default_value = ".yourdocs")]
    data_dir: PathBuf,
  },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  tracing_subscriber::fmt().with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into())).init();

  match Cli::parse().command {
    Command::Dev { bind, data_dir } => yourdocs_server::run_dev(bind, data_dir).await,
  }
}
