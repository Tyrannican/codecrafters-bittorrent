use std::net::SocketAddrV4;

use anyhow::{Context, Result};
use bytes::{Buf, BufMut, BytesMut};
use futures_util::StreamExt;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tokio_util::codec::{Decoder, Encoder, Framed};

const BLOCK_SIZE: u16 = 1 << 14;
const MAX: usize = 1 << 16;

#[allow(dead_code)]
#[repr(u8)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MessageId {
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have,
    Bitfield,
    Request,
    Piece,
    Cancel,
}

#[repr(C)]
#[repr(packed)]
#[derive(Debug, Clone)]
pub(crate) struct Handshake {
    pub(crate) length: u8,
    pub(crate) protocol: [u8; 19],
    pub(crate) reserved: [u8; 8],
    pub(crate) info_hash: [u8; 20],
    pub(crate) peer_id: [u8; 20],
}

impl Handshake {
    pub(crate) fn new(info_hash: [u8; 20], peer_id: [u8; 20]) -> Self {
        Self {
            length: 19,
            protocol: *b"BitTorrent protocol",
            reserved: [0; 8],
            info_hash,
            peer_id,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PeerMessage {
    pub(crate) id: MessageId,
    pub(crate) payload: Vec<u8>,
}

pub(crate) struct Peer {
    address: SocketAddrV4,
    stream: Framed<TcpStream, PeerMessageCodec>,
}

impl Peer {
    pub(crate) async fn new(addr: SocketAddrV4, info_hash: &[u8; 20]) -> Result<Self> {
        let mut peer = TcpStream::connect(addr).await?;
        let mut handshake = Handshake::new(*info_hash, *b"00112233445566778899");

        // Help for this came from: https://github.com/jonhoo/codecrafters-bittorrent-rust/blob/master/src/main.rs#L128
        // Cheers Jon, always teaching me the low-level stuff!
        let handshake_bytes =
            &mut handshake as *mut Handshake as *mut [u8; std::mem::size_of::<Handshake>()];

        // Safety: Repr C and packed makes this safe
        let handshake_bytes = unsafe { &mut *handshake_bytes };

        peer.write_all(handshake_bytes).await?;
        peer.read_exact(handshake_bytes).await?;

        anyhow::ensure!(handshake.length == 19);
        anyhow::ensure!(&handshake.protocol == b"BitTorrent protocol");

        let mut stream = Framed::new(peer, PeerMessageCodec);

        let bitfield = stream
            .next()
            .await
            .expect("always start with a bitfield")
            .context("bitfield message was invalid")?;

        anyhow::ensure!(bitfield.id == MessageId::Bitfield);

        Ok(Self {
            address: addr,
            stream,
        })
    }
}

// Again, idea for using codec comes from Jon Gjengset implementation
// but going to give it a go myself
// Good resource here: https://docs.rs/tokio-util/latest/tokio_util/codec/index.html
pub(crate) struct PeerMessageCodec;

impl Decoder for PeerMessageCodec {
    type Item = PeerMessage;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Need the length parameter
        if src.len() < 4 {
            return Ok(None);
        }

        let mut length_bytes = [0u8; 4];
        length_bytes.copy_from_slice(&src[..4]);
        let length = u32::from_be_bytes(length_bytes) as usize;

        if length == 0 {
            // heartbeat apparently
            src.advance(4);
            return self.decode(src);
        }

        // Need to read the id, not enough bytes
        if src.len() < 5 {
            return Ok(None);
        }

        if length > MAX {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Frame of length {length} is too large"),
            ));
        }

        if src.len() < 4 + length {
            // Full data has not arrived yet
            //
            // Reserve more space in the buffer
            src.reserve(4 + length - src.len());
            return Ok(None);
        }

        let message_id = match src[4] {
            0 => MessageId::Choke,
            1 => MessageId::Unchoke,
            2 => MessageId::Interested,
            3 => MessageId::NotInterested,
            4 => MessageId::Have,
            5 => MessageId::Bitfield,
            6 => MessageId::Request,
            7 => MessageId::Piece,
            8 => MessageId::Cancel,
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("invalid message id {} received", src[4]),
                ))
            }
        };

        let payload = if src.len() > 5 {
            src[5..4 + length].to_vec()
        } else {
            Vec::new()
        };

        Ok(Some(PeerMessage {
            id: message_id,
            payload,
        }))
    }
}

impl Encoder<PeerMessage> for PeerMessageCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: PeerMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        if item.payload.len() > MAX {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Frame length {} is too large", item.payload.len()),
            ));
        }

        let length = u32::to_be_bytes(item.payload.len() as u32 + 1);

        dst.reserve(4 + 1 + item.payload.len());

        dst.extend_from_slice(&length);
        dst.put_u8(item.id as u8);
        dst.extend_from_slice(&item.payload);

        Ok(())
    }
}
