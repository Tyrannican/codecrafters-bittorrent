use anyhow::{Context, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::peer::Handshake;
use crate::torrent::Torrent;

use std::net::SocketAddrV4;
use std::path::Path;

pub(crate) async fn invoke(file: impl AsRef<Path>, peer: String) -> Result<()> {
    anyhow::ensure!(peer.split_once(':').is_some());
    let torrent = Torrent::from_file(file)?;
    let info_hash = torrent.info_hash()?;

    let peer_addr = peer
        .parse::<SocketAddrV4>()
        .context("parsing peer address")?;

    let mut peer = TcpStream::connect(peer_addr).await?;
    let mut handshake = Handshake::new(info_hash, *b"00112233445566778899");

    let handshake_bytes = handshake.as_bytes_mut();

    peer.write_all(handshake_bytes)
        .await
        .context("sending handshake")?;
    peer.read_exact(handshake_bytes)
        .await
        .context("receiving handshake")?;

    println!("Peer ID: {}", hex::encode(handshake.peer_id));

    Ok(())
}
