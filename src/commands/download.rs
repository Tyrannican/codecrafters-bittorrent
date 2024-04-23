use anyhow::{Context, Result};
use sha1::{Digest, Sha1};
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
    let peer_response = TrackerClient::peers(&torrent).await?;

    let piece_length = calculate_piece_length(piece_id, &torrent);

    // This is a stupid hack because for some reason, some of the peers
    // have an issue with the handshake when it works fine other times
    let mut peers = vec![];
    for peer in peer_response.peers.0.into_iter() {
        if let Ok(peer) = Peer::new(peer, &info_hash).await {
            peers.push(peer);
        }
    }

    anyhow::ensure!(peers.len() > 0, "no available peers");
    let mut peer = peers.remove(0);
    drop(peers);
    let downloaded_piece = peer
        .download_piece(piece_id, piece_length)
        .await
        .context("calling peer to download piece")?;

    let mut hasher = Sha1::new();
    hasher.update(&downloaded_piece);
    let result: [u8; 20] = hasher.finalize().try_into().expect("this should work");
    anyhow::ensure!(result == torrent.info.pieces.0[piece_id]);

    let mut file = tokio::fs::File::create(&output).await?;
    file.write_all(&downloaded_piece)
        .await
        .with_context(|| format!("writing out piece to {}", output.display()))?;

    println!("Piece {} downloaded to {}", piece_id, output.display());

    Ok(())
}

pub(crate) async fn full(output: PathBuf, torrent_file: PathBuf) -> Result<()> {
    let torrent = Torrent::from_file(&torrent_file)?;
    let info_hash = torrent.info_hash()?;
    let peer_response = TrackerClient::peers(&torrent)
        .await
        .context("fetching peer list")?;

    let mut peers = Vec::new();
    for peer in peer_response.peers.0.into_iter() {
        if let Ok(peer) = Peer::new(peer, &info_hash).await {
            peers.push(peer);
        }
    }

    anyhow::ensure!(peers.len() > 0, "should have at least one peer");
    let mut peer = peers.remove(0);
    let mut full_content = Vec::new();
    //let mut peer = Peer::new(peer_response.peers.0[0], &info_hash).await?;
    for (id, piece) in torrent.info.pieces.0.iter().enumerate() {
        let piece_length = calculate_piece_length(id, &torrent);
        let content = peer
            .download_piece(id, piece_length)
            .await
            .with_context(|| format!("downloading piece {id}"))?;

        let mut hasher = Sha1::new();
        hasher.update(&content);
        let result: [u8; 20] = hasher.finalize().try_into().expect("this should work");
        anyhow::ensure!(result == *piece);
        full_content.extend(content);
    }

    let mut file = tokio::fs::File::create(&output)
        .await
        .context("creating output file")?;

    file.write_all(&full_content)
        .await
        .context("writing file content")?;

    println!(
        "Downloaded {} to {}",
        torrent_file.display(),
        output.display()
    );

    Ok(())
}

fn calculate_piece_length(piece_id: usize, torrent: &Torrent) -> usize {
    let length = match torrent.info.t_class {
        TorrentClass::SingleFile { length } => length,
        _ => unimplemented!("someday"),
    };

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

    piece_length
}
