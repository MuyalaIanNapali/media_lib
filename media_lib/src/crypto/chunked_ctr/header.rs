//! The authenticated CHATMED2 file header.
//!
//! ```text
//! 0..8    magic "CHATMED2"
//! 8..12   format version (LE u32, = 2)
//! 12..16  plaintext chunk size (LE u32)
//! 16..24  plaintext size (LE u64)
//! 24..40  AES-CTR IV
//! 40..64  reserved (zero)
//! 64..96  HMAC-SHA256 over bytes 0..64
//! ```

use std::io::{Read, Seek, SeekFrom};

use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::error::{MediaLibError, MediaResult};
use crate::io_util::read_exact_or_truncated;

type HmacSha256 = Hmac<Sha256>;

const HEADER_SIZE: usize = 64;
const MAC_SIZE: usize = 32;
/// Header plus its MAC: where the first chunk starts.
pub(crate) const SEALED_SIZE: u64 = (HEADER_SIZE + MAC_SIZE) as u64;
/// Plaintext bytes per chunk (a multiple of the AES block size).
pub(crate) const CHUNK_SIZE: u32 = 256 * 1024;

const MAGIC: &[u8; 8] = b"CHATMED2";
const FORMAT_VERSION: u32 = 2;

/// A header and its MAC, ready to write.
pub(crate) struct SealedHeader {
    pub bytes: [u8; HEADER_SIZE],
    pub mac: [u8; MAC_SIZE],
}

/// The fields a reader needs, from a header whose MAC has been verified.
pub(crate) struct HeaderInfo {
    pub plaintext_size: u64,
    pub iv: [u8; 16],
}

fn mac_for(mac_key: &[u8; 32], bytes: &[u8; HEADER_SIZE]) -> HmacSha256 {
    let mut mac = HmacSha256::new_from_slice(mac_key).expect("HMAC accepts keys of any length");
    mac.update(bytes);
    mac
}

/// Builds and authenticates a header.
pub(crate) fn seal(plaintext_size: u64, iv: &[u8; 16], mac_key: &[u8; 32]) -> SealedHeader {
    let mut bytes = [0u8; HEADER_SIZE];
    bytes[0..8].copy_from_slice(MAGIC);
    bytes[8..12].copy_from_slice(&FORMAT_VERSION.to_le_bytes());
    bytes[12..16].copy_from_slice(&CHUNK_SIZE.to_le_bytes());
    bytes[16..24].copy_from_slice(&plaintext_size.to_le_bytes());
    bytes[24..40].copy_from_slice(iv);

    let mut mac = [0u8; MAC_SIZE];
    mac.copy_from_slice(&mac_for(mac_key, &bytes).finalize().into_bytes());
    SealedHeader { bytes, mac }
}

/// Reads the header at the start of `input`, verifies its MAC (wrong key or
/// tampering fails here) and only then trusts its fields.
pub(crate) fn open<R: Read + Seek + ?Sized>(input: &mut R, mac_key: &[u8; 32]) -> MediaResult<HeaderInfo> {
    input.seek(SeekFrom::Start(0))?;
    let mut bytes = [0u8; HEADER_SIZE];
    let mut stored_mac = [0u8; MAC_SIZE];
    read_exact_or_truncated(input, &mut bytes)?;
    read_exact_or_truncated(input, &mut stored_mac)?;

    mac_for(mac_key, &bytes)
        .verify_slice(&stored_mac)
        .map_err(|_| MediaLibError::HeaderAuthenticationFailed)?;

    let u32_at = |o: usize| u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
    if &bytes[0..8] != MAGIC || u32_at(8) != FORMAT_VERSION || u32_at(12) != CHUNK_SIZE {
        return Err(MediaLibError::UnsupportedFormat);
    }

    Ok(HeaderInfo {
        plaintext_size: u64::from_le_bytes(bytes[16..24].try_into().unwrap()),
        iv: bytes[24..40].try_into().unwrap(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn sealed_bytes(size: u64, key: &[u8; 32]) -> Vec<u8> {
        let header = seal(size, &[4u8; 16], key);
        [header.bytes.as_slice(), header.mac.as_slice()].concat()
    }

    #[test]
    fn seal_then_open() {
        let info = open(&mut Cursor::new(sealed_bytes(12345, &[1u8; 32])), &[1u8; 32]).unwrap();
        assert_eq!(info.plaintext_size, 12345);
        assert_eq!(info.iv, [4u8; 16]);
    }

    #[test]
    fn wrong_key_or_tampering_fails_authentication() {
        let bytes = sealed_bytes(1, &[1u8; 32]);
        assert!(matches!(
            open(&mut Cursor::new(bytes.clone()), &[2u8; 32]),
            Err(MediaLibError::HeaderAuthenticationFailed)
        ));
        let mut tampered = bytes;
        tampered[17] ^= 1;
        assert!(matches!(
            open(&mut Cursor::new(tampered), &[1u8; 32]),
            Err(MediaLibError::HeaderAuthenticationFailed)
        ));
    }

    #[test]
    fn short_header_is_truncated() {
        assert!(matches!(open(&mut Cursor::new(vec![0u8; 50]), &[1u8; 32]), Err(MediaLibError::Truncated)));
    }
}
