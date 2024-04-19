use anyhow::{Context, Result};
use serde::Deserialize;
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

    // Help for this came from: https://github.com/jonhoo/codecrafters-bittorrent-rust/blob/master/src/main.rs#L128
    // Cheers Jon, always teaching me the low-level stuff!
    let handshake_bytes =
        &mut handshake as *mut Handshake as *mut [u8; std::mem::size_of::<Handshake>()];

    // Safety: Repr C and packed makes this safe
    let handshake_bytes = unsafe { &mut *handshake_bytes };

    peer.write_all(handshake_bytes).await?;
    peer.read_exact(handshake_bytes).await?;

    Ok(())
}
