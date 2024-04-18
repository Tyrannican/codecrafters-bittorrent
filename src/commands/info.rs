use anyhow::{Context, Result};
use std::path::Path;

use crate::torrent::{Torrent, TorrentClass};

pub(crate) fn invoke(file: impl AsRef<Path>) -> Result<()> {
    let torrent = std::fs::read(&file).context("parsing torrent file")?;
    let torrent: Torrent =
        serde_bencode::from_bytes(&torrent).context("decoding bencoded stream")?;
    println!("Tracker URL: {}", torrent.announce);
    let info = torrent.info;
    match info.t_class {
        TorrentClass::SingleFile { length } => println!("Length: {length}"),
        TorrentClass::MultiFile { files: _ } => unimplemented!("not yet ready"),
    }
    println!(
        "Info Hash: {}",
        info.hash().context("hashing torrent info")?
    );
    println!("Piece Length: {}", info.piece_length);
    println!("Piece Hashes:");
    for piece in info.piece_hashes().context("hashing pieces")? {
        println!("{piece}");
    }

    Ok(())
}
