use anyhow::Result;
use tokio::io::AsyncWriteExt;

use std::path::PathBuf;

use crate::{peer::Peer, torrent::Torrent, tracker::TrackerClient};

pub(crate) async fn piece(output: PathBuf, torrent: PathBuf, piece: usize) -> Result<()> {
    let torrent = Torrent::from_file(torrent)?;
    let info_hash = torrent.info_hash()?;
    let piece_length = torrent.info.piece_length;
    let peer_response = TrackerClient::peers(&torrent).await?;
    let peer = peer_response.peers.0[0];
    let mut peer = Peer::new(peer, &info_hash).await?;
    let downloaded_piece = peer.download_piece(piece, piece_length).await?;
    let mut file = tokio::fs::File::create(&output).await?;
    file.write_all(&downloaded_piece).await?;
    println!("Piece {piece} downloaded to {}", output.display());

    Ok(())
}
