use anyhow::{Context, Result};

use std::path::Path;

use crate::torrent::Torrent;
use crate::tracker::TrackerClient;

pub async fn invoke(file: impl AsRef<Path>) -> Result<()> {
    let torrent = Torrent::from_file(file).context("loading torrent file")?;
    TrackerClient::peers(torrent)
        .await
        .context("calling tracker endpoint")?;

    Ok(())
}
