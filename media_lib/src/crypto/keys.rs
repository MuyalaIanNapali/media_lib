//! Media keys and the sub-keys derived from them.

use dryoc::dryocstream::Key;
use rand::random;

use super::hkdf_sha256::HkdfSha256Crypto;
use super::wipe::wipe;
use crate::error::{MediaLibError, MediaResult};

/// The single random 32-byte key stored per media item.
pub type MediaKey = Key;

/// HKDF context for the chunked AES-CTR sub-keys. Never change it: every
/// existing audio/video file was encrypted under keys derived with it.
const VIDEO_KEYS_INFO: &[u8] = b"media-lib ffmpeg strip+encrypt";

pub fn generate_key() -> MediaKey {
    Key::from(random::<[u8; 32]>())
}

/// Rebuilds a key from the 32 raw bytes returned by [`generate_key`].
pub fn key_from_bytes(bytes: &[u8]) -> MediaResult<MediaKey> {
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| MediaLibError::InvalidKeyLength(bytes.len()))?;
    Ok(Key::from(arr))
}

/// Sub-keys of the chunked AES-CTR (CHATMED2) format, derived from one
/// [`MediaKey`] with HKDF-SHA256. Wiped on drop.
pub struct VideoKeys {
    pub header_key: [u8; 32],
    pub chunk_mac_key: [u8; 32],
    pub encryption_key: [u8; 32],
    /// Used when encrypting; decryption reads the IV from the file header.
    pub iv: [u8; 16],
}

impl VideoKeys {
    pub fn derive(key: &MediaKey) -> MediaResult<Self> {
        let prk = HkdfSha256Crypto::extract(&[0u8; 32], key);
        let mut okm = HkdfSha256Crypto::expand(&prk, VIDEO_KEYS_INFO, 112)?;

        let take = |range: std::ops::Range<usize>| okm[range].to_vec();
        let keys = (|| {
            Ok(VideoKeys {
                header_key: take(0..32).try_into().map_err(|_| MediaLibError::KeyDerivation)?,
                chunk_mac_key: take(32..64).try_into().map_err(|_| MediaLibError::KeyDerivation)?,
                encryption_key: take(64..96).try_into().map_err(|_| MediaLibError::KeyDerivation)?,
                iv: take(96..112).try_into().map_err(|_| MediaLibError::KeyDerivation)?,
            })
        })();
        wipe(&mut okm);
        keys
    }
}

impl Drop for VideoKeys {
    fn drop(&mut self) {
        wipe(&mut self.header_key);
        wipe(&mut self.chunk_mac_key);
        wipe(&mut self.encryption_key);
        wipe(&mut self.iv);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_from_bytes_rejects_wrong_length() {
        assert!(matches!(key_from_bytes(&[0u8; 16]), Err(MediaLibError::InvalidKeyLength(16))));
        assert!(matches!(key_from_bytes(&[0u8; 33]), Err(MediaLibError::InvalidKeyLength(33))));
        assert!(matches!(key_from_bytes(&[]), Err(MediaLibError::InvalidKeyLength(0))));
    }

    #[test]
    fn key_bytes_round_trip() {
        let key = generate_key();
        assert_eq!(key_from_bytes(&key[..]).unwrap()[..], key[..]);
    }

    #[test]
    fn derivation_is_deterministic_and_keys_differ() {
        let key = generate_key();
        let (a, b) = (VideoKeys::derive(&key).unwrap(), VideoKeys::derive(&key).unwrap());
        assert_eq!(a.header_key, b.header_key);
        assert_eq!(a.iv, b.iv);
        assert_ne!(a.header_key, a.chunk_mac_key);
        assert_ne!(a.chunk_mac_key, a.encryption_key);
    }
}
