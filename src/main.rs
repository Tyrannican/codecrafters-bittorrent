mod torrent;

use serde_bencode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use torrent::Torrent;

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
}

fn decode_bencoded_value(value: &str) -> (serde_json::Value, &str) {
    match value.chars().next() {
        Some('0'..='9') => {
            if let Some((size, rest)) = value.split_once(':') {
                if let Ok(size) = size.parse::<usize>() {
                    return (rest[..size].to_string().into(), &rest[size..]);
                }
            }
        }
        Some('i') => {
            let value = &value[1..];
            if let Some((val, rest)) = value.split_once('e').and_then(|(digits, rest)| {
                let n = digits.parse::<i64>().ok()?;
                Some((n, rest))
            }) {
                return (val.into(), rest);
            }
        }
        Some('l') => {
            let mut values = Vec::new();
            let mut rest = &value[1..];
            while !rest.is_empty() && !rest.starts_with('e') {
                let (v, remainder) = decode_bencoded_value(rest);
                values.push(v);
                rest = remainder;
            }

            return (values.into(), &rest[1..]);
        }
        Some('d') => {
            let mut dict = serde_json::Map::new();
            let mut rest = &value[1..];
            while !rest.is_empty() && !rest.starts_with('e') {
                let (key, remainder) = decode_bencoded_value(rest);

                let key = match key {
                    serde_json::Value::String(key) => key,
                    key => {
                        panic!("dict strings must be keys, not {key:?}");
                    }
                };

                let (v, remainder) = decode_bencoded_value(remainder);
                dict.insert(key, v);
                rest = remainder;
            }

            return (dict.into(), &rest[1..]);
        }
        _ => {}
    }

    panic!("unrecognized value: {value}");
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Decode { value } => {
            let (output, _) = decode_bencoded_value(&value);
            println!("{output}");
        }
        Commands::Info { file } => {
            let torrent = std::fs::read(&file).context("parsing torrent file")?;
            let torrent: Torrent =
                serde_bencode::from_bytes(&torrent).context("decoding bencoded stream")?;
            println!("Tracker URL: {}", torrent.announce);
            match torrent.info.t_class {
                torrent::TorrentClass::SingleFile { length } => println!("Length: {length}"),
                torrent::TorrentClass::MultiFile { files: _ } => unimplemented!("not yet ready"),
            }
        }
    }

    Ok(())
}
