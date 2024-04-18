mod commands;
mod torrent;
mod tracker;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Decode { value: String },
    Info { file: String },
    Peers { file: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Decode { value } => {
            let (output, _) =
                commands::decode::invoke(&value).context("decoding bencoded value")?;
            println!("{output}");
        }
        Commands::Info { file } => commands::info::invoke(file).context("parsing torrent info")?,
        Commands::Peers { file } => commands::peers::invoke(file)
            .await
            .context("getting torrent peers")?,
    }

    Ok(())
}
