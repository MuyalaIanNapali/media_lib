//! Per-chunk authentication: HMAC-SHA256 over the ciphertext, bound to its
//! chunk index and length so chunks can't be reordered, dropped or resized.

use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::error::{MediaLibError, MediaResult};

type HmacSha256 = Hmac<Sha256>;

pub(crate) const TAG_SIZE: usize = 32;
const DOMAIN: &[u8] = b"CHATMED2-CHUNK";

pub(crate) struct ChunkAuthenticator<'k> {
    key: &'k [u8; 32],
}

impl<'k> ChunkAuthenticator<'k> {
    pub fn new(key: &'k [u8; 32]) -> Self {
        Self { key }
    }

    fn mac(&self, index: u64, ciphertext: &[u8]) -> HmacSha256 {
        let mut mac = HmacSha256::new_from_slice(self.key).expect("HMAC accepts keys of any length");
        mac.update(DOMAIN);
        mac.update(&index.to_le_bytes());
        mac.update(&(ciphertext.len() as u64).to_le_bytes());
        mac.update(ciphertext);
        mac
    }

    pub fn tag(&self, index: u64, ciphertext: &[u8]) -> [u8; TAG_SIZE] {
        let mut tag = [0u8; TAG_SIZE];
        tag.copy_from_slice(&self.mac(index, ciphertext).finalize().into_bytes());
        tag
    }

    /// Constant-time verification.
    pub fn verify(&self, index: u64, ciphertext: &[u8], tag: &[u8; TAG_SIZE]) -> MediaResult<()> {
        self.mac(index, ciphertext)
            .verify_slice(tag)
            .map_err(|_| MediaLibError::ChunkAuthenticationFailed(index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_is_bound_to_index_and_content() {
        let auth = ChunkAuthenticator::new(&[7u8; 32]);
        let tag = auth.tag(3, b"chunk");
        assert!(auth.verify(3, b"chunk", &tag).is_ok());
        assert!(matches!(auth.verify(4, b"chunk", &tag), Err(MediaLibError::ChunkAuthenticationFailed(4))));
        assert!(auth.verify(3, b"chunK", &tag).is_err());
    }
}
