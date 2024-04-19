#[repr(C)]
#[repr(packed)]
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
