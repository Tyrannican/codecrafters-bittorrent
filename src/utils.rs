use sha1::{Digest, Sha1};

pub(crate) fn hash_content(content: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(&content);
    let hashed = hasher.finalize();
    hex::encode(hashed)
}
