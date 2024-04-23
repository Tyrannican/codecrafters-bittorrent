use anyhow::{Context, Result};
use tokio::io::AsyncWriteExt;

use std::path::PathBuf;

use crate::{
    peer::Peer,
    torrent::{Torrent, TorrentClass},
    tracker::TrackerClient,
};

pub(crate) async fn piece(output: PathBuf, torrent: PathBuf, piece_id: usize) -> Result<()> {
    let torrent = Torrent::from_file(torrent)?;
    let info_hash = torrent.info_hash()?;
    let length = match torrent.info.t_class {
        TorrentClass::SingleFile { length } => length,
        _ => unimplemented!("someday"),
    };
    let peer_response = TrackerClient::peers(&torrent).await?;
    let peer = peer_response.peers.0[1];

    let mut peer = Peer::new(peer, &info_hash).await?;

    // Piece magic tripping me up...
    let piece_length = if piece_id == &torrent.info.pieces.0.len() - 1 {
        let remainder = length % torrent.info.piece_length;
        if remainder == 0 {
            torrent.info.piece_length
        } else {
            remainder
        }
    } else {
        torrent.info.piece_length
    };

    let downloaded_piece = peer
        .download_piece(piece_id, piece_length)
        .await
        .context("calling peer to download piece")?;

    let mut file = tokio::fs::File::create(&output).await?;
    file.write_all(&downloaded_piece)
        .await
        .with_context(|| format!("writing out piece to {}", output.display()))?;

    println!("Piece {} downloaded to {}", piece_id, output.display());

    Ok(())
}
