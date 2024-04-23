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
    let peer = peer_response.peers.0[1];

    let mut peer = Peer::new(peer, &info_hash).await?;
    let piece_length = calculate_piece_length(piece_id, &torrent);

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
    let peer = peer_response.peers.0[1];
    let mut peer = Peer::new(peer, &info_hash).await?;

    let mut content = Vec::new();
    for (idx, piece_hash) in torrent.info.pieces.0.iter().enumerate() {
        let piece_length = calculate_piece_length(idx, &torrent);
        let piece = peer
            .download_piece(idx, piece_length)
            .await
            .context("downloading piece")?;

        let mut hasher = Sha1::new();
        hasher.update(&piece);
        let result: [u8; 20] = hasher.finalize().try_into().expect("this should work");
        anyhow::ensure!(result == *piece_hash);
        content.extend(piece);
    }

    let mut file = tokio::fs::File::create(&output).await?;
    file.write_all(&content).await?;
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
