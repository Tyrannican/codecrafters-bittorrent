use anyhow::{Context, Result};
use tokio::io::AsyncWriteExt;

use std::path::PathBuf;

use crate::{peer::Peer, torrent::Torrent, tracker::TrackerClient};

pub(crate) async fn piece(output: PathBuf, torrent: PathBuf, piece_id: usize) -> Result<()> {
    let torrent = Torrent::from_file(torrent)?;
    let info_hash = torrent.info_hash()?;
    let piece_length = torrent.info.piece_length;
    let peer_response = TrackerClient::peers(&torrent).await?;
    let peer = peer_response.peers.0[0];

    let mut peer = Peer::new(peer, &info_hash).await?;
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
