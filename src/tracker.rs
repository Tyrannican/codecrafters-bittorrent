use anyhow::{Context, Result};
use reqwest::Client;

use crate::torrent::Torrent;

pub(crate) struct TrackerClient;

impl TrackerClient {
    pub(crate) async fn peers(torrent: Torrent) -> Result<()> {
        Ok(())
    }
}
