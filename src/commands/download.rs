use anyhow::Result;

use std::path::PathBuf;

use crate::{peer::Peer, torrent::Torrent, tracker::TrackerClient};

pub(crate) async fn piece(output: PathBuf, torrent: PathBuf, piece: usize) -> Result<()> {
    let torrent = Torrent::from_file(torrent)?;
    let info_hash = torrent.info_hash()?;
    let peer_response = TrackerClient::peers(&torrent).await?;
    let peer = peer_response.peers.0[0];
    let peer = Peer::new(peer, &info_hash).await?;

    Ok(())
}
