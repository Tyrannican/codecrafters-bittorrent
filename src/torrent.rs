use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Torrent {
    pub(crate) announce: String,
    pub(crate) info: TorrentInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TorrentInfo {
    pub(crate) name: String,

    #[serde(rename = "piece length")]
    pub(crate) piece_length: usize,
}
