use anyhow::{Context, Result};
use serde::Serialize;

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

impl TrackerClient {
    pub(crate) async fn peers(torrent: Torrent) -> Result<()> {
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

        let response = reqwest::get(&url).await?.text().await?;
        println!("Response: {response}");
        Ok(())
    }
}

// TODO: Same dance for hashes - implement Visitor etc

fn urlencode(hash: &[u8; 20]) -> String {
    let mut encoded = String::with_capacity(3 * hash.len());
    for &byte in hash {
        encoded.push('%');
        encoded.push_str(&hex::encode(&[byte]));
    }

    encoded
}
