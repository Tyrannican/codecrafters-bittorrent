use anyhow::{Context, Result};
use serde::de::{Deserializer, Visitor};
use serde::{Deserialize, Serialize};

use std::net::{Ipv4Addr, SocketAddrV4};

use crate::torrent::Torrent;

pub(crate) struct TrackerClient;

#[derive(Debug, Clone, Serialize)]
struct TrackerRequest {
    peer_id: String,
    port: u16,
    uploaded: usize,
    downloaded: usize,
    left: usize,
    compact: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct TrackerResponse {
    pub(crate) peers: Peers,
}

impl TrackerClient {
    pub(crate) async fn peers(torrent: Torrent) -> Result<TrackerResponse> {
        let tracker_request = TrackerRequest {
            peer_id: "00112233445566778899".to_string(),
            port: 6881,
            uploaded: 0,
            downloaded: 0,
            left: torrent.length(),
            compact: 1,
        };
        let info_hash = torrent.info_hash()?;
        let encoded = urlencode(&info_hash);
        let url = format!(
            "{}?{}&info_hash={}",
            torrent.announce,
            serde_urlencoded::to_string(&tracker_request)?,
            encoded
        );

        let response = reqwest::get(&url).await?.bytes().await?;
        let tracker_response: TrackerResponse =
            serde_bencode::from_bytes(&response).context("deserializing tracker response")?;

        Ok(tracker_response)
    }
}

// TODO: Same dance for hashes - implement Visitor etc

#[derive(Debug, Clone)]
pub(crate) struct Peers(pub(crate) Vec<SocketAddrV4>);

struct PeersVisitor;

impl<'de> Visitor<'de> for PeersVisitor {
    type Value = Peers;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("expecting a slice of bytes of at least size multiple of 6")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if v.len() % 6 != 0 {
            return Err(E::custom("expecting 6 bytes"));
        }

        Ok(Peers(
            v.chunks_exact(6)
                .map(|chunk| {
                    let peer_addr = Ipv4Addr::new(chunk[0], chunk[1], chunk[2], chunk[3]);
                    let port = u16::from_be_bytes([chunk[4], chunk[5]]);
                    SocketAddrV4::new(peer_addr, port)
                })
                .collect(),
        ))
    }
}

impl<'de> Deserialize<'de> for Peers {
    fn deserialize<D>(deserializer: D) -> Result<Peers, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_bytes(PeersVisitor)
    }
}

fn urlencode(hash: &[u8; 20]) -> String {
    let mut encoded = String::with_capacity(3 * hash.len());
    for &byte in hash {
        encoded.push('%');
        encoded.push_str(&hex::encode(&[byte]));
    }

    encoded
}
