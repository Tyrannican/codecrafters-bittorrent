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
            let start = value
                .chars()
                .nth(0)
                .expect("bencoded value must be non-zero");
            match start {
                '0'..='9' => {
                    let Some((_, rest)) = value.split_once(':') else {
                        anyhow::bail!("not a valid bencoded value")
                    };
                    println!("\"{rest}\"");
                }
                'i' => {
                    let Some((value, _)) = value.split_once('e') else {
                        anyhow::bail!("incomplete integer bencoded value");
                    };

                    let value = value.parse::<i64>().context("converting to integer")?;
                    println!("{value}");
                }
                'l' => {}
                'd' => {}
                _ => anyhow::bail!("invalid bencoded value"),
            }
        }
    }

    Ok(())
}
