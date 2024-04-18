use anyhow::{Context, Result};
use std::path::Path;

use crate::torrent::{Torrent, TorrentClass};

pub(crate) fn invoke(file: impl AsRef<Path>) -> Result<()> {
    let torrent = Torrent::from_file(file).context("loading torrent file")?;
    let info_hash = torrent.info_hash().context("generating info hash")?;
    let info = torrent.info;
    match info.t_class {
        TorrentClass::SingleFile { length } => println!("Length: {length}"),
        TorrentClass::MultiFile { files: _ } => unimplemented!("not yet ready"),
    }

    println!("Tracker URL: {}", torrent.announce);
    println!("Info Hash: {}", hex::encode(&info_hash));
    println!("Piece Length: {}", info.piece_length);
    println!("Piece Hashes:");
    for piece in info.pieces.0 {
        println!("{}", hex::encode(piece));
    }

    Ok(())
}
