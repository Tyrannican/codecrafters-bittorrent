mod commands;
mod peer;
mod torrent;
mod tracker;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
#[clap(rename_all = "snake_case")]
enum Commands {
    Decode {
        value: String,
    },
    Info {
        file: PathBuf,
    },
    Peers {
        file: PathBuf,
    },
    Handshake {
        file: PathBuf,
        peer: String,
    },
    DownloadPiece {
        #[arg(short)]
        output: PathBuf,
        torrent: PathBuf,
        piece: usize,
    },
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
        Commands::Handshake { file, peer } => commands::handshake::invoke(file, peer)
            .await
            .context("conducting peer handshake")?,

        Commands::DownloadPiece {
            output,
            torrent,
            piece,
        } => commands::download::piece(output, torrent, piece)
            .await
            .context("downloading piece")?,
    }

    Ok(())
}
