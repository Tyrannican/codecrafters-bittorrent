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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Decode { value } => {
            let Some((size, rest)) = value.split_once(':') else {
                anyhow::bail!("not a valid bencoded value")
            };
            let _size = size.parse::<usize>().context("extracting size")?;
            println!("\"{rest}\"");
        }
    }

    Ok(())
}
