use std::net::SocketAddrV4;

use anyhow::{Context, Result};
use bytes::{Buf, BufMut, BytesMut};
use futures_util::{SinkExt, StreamExt};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tokio_util::codec::{Decoder, Encoder, Framed};

const BLOCK_SIZE: usize = 1 << 14;
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
    _address: SocketAddrV4,
    stream: Framed<TcpStream, PeerMessageCodec>,
    choked: bool,
}

impl Peer {
    pub(crate) async fn new(addr: SocketAddrV4, info_hash: &[u8; 20]) -> Result<Self> {
        let mut stream = establish_connection(addr, info_hash)
            .await
            .context("establishing connection with peer")?;

        let bitfield = stream
            .next()
            .await
            .expect("always start with a bitfield")
            .context("bitfield message was invalid")?;

        anyhow::ensure!(bitfield.id == MessageId::Bitfield);

        Ok(Self {
            _address: addr,
            stream,
            choked: true,
        })
    }

    pub(crate) async fn download_piece(
        &mut self,
        piece_id: usize,
        length: usize,
    ) -> Result<Vec<u8>> {
        self.interested()
            .await
            .context("sending interested message")?;

        let mut piece = Vec::with_capacity(length);

        let blocks = length / BLOCK_SIZE;
        let remainder = length % BLOCK_SIZE;
        let blocks = if remainder > 0 { blocks + 1 } else { blocks };

        println!("Total blocks: {blocks}");
        for block in 0..blocks {
            println!("Block number: {block}");
            let mut payload = Vec::new();
            let idx = block * BLOCK_SIZE;

            payload.extend((piece_id as u32).to_be_bytes());
            payload.extend((idx as u32).to_be_bytes());

            // Use remaining bytes if we're on the last block and there is remaining bytes
            if block == blocks - 1 && remainder > 0 {
                payload.extend((remainder as u32).to_be_bytes());
            } else {
                payload.extend((BLOCK_SIZE as u32).to_be_bytes());
            }

            let request = PeerMessage {
                id: MessageId::Request,
                payload,
            };

            self.stream
                .send(request)
                .await
                .context("sending piece request")?;

            let response = self
                .stream
                .next()
                .await
                .expect("should be a piece response")
                .context("invalid peer message")?;

            anyhow::ensure!(response.id == MessageId::Piece);

            let payload = response.payload;
            let block = &payload[8..];

            piece.extend_from_slice(block);
        }
        // Send Interested Message
        // Receive Unchoke
        // Split into 1 << 14 size blocks
        // Send request
        // Calcualte size of last block which will be <= to 1 << 14
        // Combine result to form piece
        // Save to disk
        Ok(piece)
    }

    async fn interested(&mut self) -> Result<()> {
        let interested = PeerMessage {
            id: MessageId::Interested,
            payload: Vec::new(),
        };

        self.stream
            .send(interested)
            .await
            .context("sending interested message")?;

        let unchoke = self
            .stream
            .next()
            .await
            .expect("always sends an unchoke")
            .context("invalid peer message")?;

        anyhow::ensure!(unchoke.id == MessageId::Unchoke);
        self.choked = false;

        Ok(())
    }
}

async fn establish_connection(
    address: SocketAddrV4,
    info_hash: &[u8; 20],
) -> Result<Framed<TcpStream, PeerMessageCodec>> {
    let mut peer = TcpStream::connect(address)
        .await
        .context("connecting to peer")?;

    let mut handshake = Handshake::new(*info_hash, *b"00112233445566778899");

    // Help for this came from: https://github.com/jonhoo/codecrafters-bittorrent-rust/blob/master/src/main.rs#L128
    // Cheers Jon, always teaching me the low-level stuff!
    let handshake_bytes =
        &mut handshake as *mut Handshake as *mut [u8; std::mem::size_of::<Handshake>()];

    // Safety: Repr C and packed makes this safe
    let handshake_bytes = unsafe { &mut *handshake_bytes };

    peer.write_all(handshake_bytes)
        .await
        .context("sending handshake")?;
    peer.read_exact(handshake_bytes)
        .await
        .context("receiving handshake response")?;

    anyhow::ensure!(handshake.length == 19);
    anyhow::ensure!(&handshake.protocol == b"BitTorrent protocol");

    Ok(Framed::new(peer, PeerMessageCodec))
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

        src.advance(4 + length);

        Ok(Some(PeerMessage {
            id: message_id,
            payload,
        }))
    }
}

impl Encoder<PeerMessage> for PeerMessageCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: PeerMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        if item.payload.len() + 1 > MAX {
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
