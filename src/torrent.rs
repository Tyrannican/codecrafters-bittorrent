use anyhow::{Context, Result};
use serde::de::{self, Deserializer, Visitor};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

use std::path::Path;

// TODO: Create a dedicated hasher?

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Torrent {
    pub(crate) announce: String,
    pub(crate) info: TorrentInfo,
}

impl Torrent {
    pub(crate) fn from_file(torrent: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read(torrent).context("opening torrent file")?;
        serde_bencode::from_bytes(&content).context("deserializing bytes to torrent")
    }

    pub(crate) fn info_hash(&self) -> Result<[u8; 20]> {
        let encoded = serde_bencode::to_bytes(&self.info).context("serializing torrent info")?;
        let mut hasher = Sha1::new();
        hasher.update(&encoded);
        let hashed = hasher.finalize();

        Ok(hashed.into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TorrentInfo {
    pub(crate) name: String,

    #[serde(rename = "piece length")]
    pub(crate) piece_length: usize,

    pub(crate) pieces: PieceHashes,

    #[serde(flatten)]
    pub(crate) t_class: TorrentClass,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub(crate) enum TorrentClass {
    SingleFile { length: usize },
    MultiFile { files: Vec<TFile> },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct TFile {
    length: usize,
    path: Vec<String>,
}

// NOTE: Tips on Deserialzing from https://serde.rs/impl-deserialize.html
// Also, help from Jon Gjengset implementation
#[derive(Debug, Clone)]
pub(crate) struct PieceHashes(pub(crate) Vec<[u8; 20]>);
struct PieceHashesVisitor;

impl<'de> Visitor<'de> for PieceHashesVisitor {
    type Value = PieceHashes;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("expecting a slice of bytes of at least size multiple of 20")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.len() % 20 != 0 {
            return Err(E::custom(format!("length is {}", v.len())));
        }

        Ok(PieceHashes(
            v.chunks_exact(20)
                .map(|inner_slice| inner_slice.try_into().expect("is of size 20"))
                .collect(),
        ))
    }
}

impl<'de> Deserialize<'de> for PieceHashes {
    fn deserialize<D>(deserializer: D) -> Result<PieceHashes, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_bytes(PieceHashesVisitor)
    }
}

impl Serialize for PieceHashes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let single_slice = self.0.concat();
        serializer.serialize_bytes(&single_slice)
    }
}
