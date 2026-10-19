//! Seekable authenticated encryption for audio and video (CHATMED2).
//!
//! ```text
//! [sealed header: 64 bytes + 32-byte MAC]
//! [chunk 0 ciphertext][chunk 0 tag]
//! [chunk 1 ciphertext][chunk 1 tag]
//! ...
//! ```
//!
//! AES-256-CTR runs continuously across the plaintext, so any chunk can be
//! decrypted on its own by seeking the counter; each chunk carries its own
//! HMAC. A range read therefore authenticates and decrypts only the chunks it
//! touches.

mod header;
mod mac;

use std::io::{Read, Seek, SeekFrom, Write};

use aes::cipher::{KeyIvInit, StreamCipher, StreamCipherSeek};
use aes::Aes256;
use ctr::Ctr128BE;

use super::keys::{MediaKey, VideoKeys};
use super::wipe::WipeOnDrop;
use super::{EncryptionScheme, WriteSeek};
use crate::error::{MediaLibError, MediaResult};
use crate::io_util::{fill, read_exact_or_truncated};
use mac::{ChunkAuthenticator, TAG_SIZE};

type Aes256Ctr = Ctr128BE<Aes256>;

const CHUNK: u64 = header::CHUNK_SIZE as u64;

/// The audio/video [`EncryptionScheme`].
pub struct ChunkedCtrScheme;

impl EncryptionScheme for ChunkedCtrScheme {
    fn encrypt(&self, key: &MediaKey, input: &mut dyn Read, output: &mut dyn WriteSeek) -> MediaResult<()> {
        ChunkedCtr::from_media_key(key)?.encrypt(input, output)
    }
}

/// The CHATMED2 cipher for one set of [`VideoKeys`].
pub struct ChunkedCtr {
    keys: VideoKeys,
}

impl ChunkedCtr {
    pub fn new(keys: VideoKeys) -> Self {
        Self { keys }
    }

    pub fn from_media_key(key: &MediaKey) -> MediaResult<Self> {
        Ok(Self::new(VideoKeys::derive(key)?))
    }

    fn cipher_at(&self, iv: &[u8; 16], plaintext_offset: u64) -> Aes256Ctr {
        let mut cipher = Aes256Ctr::new((&self.keys.encryption_key).into(), iv.into());
        cipher.seek(plaintext_offset as u128);
        cipher
    }

    /// Encrypts all of `input`. The header is written first with a
    /// placeholder size and rewritten with the real size at the end.
    pub fn encrypt<R: Read + ?Sized, W: Write + Seek + ?Sized>(
        &self,
        input: &mut R,
        output: &mut W,
    ) -> MediaResult<()> {
        let iv = self.keys.iv;
        let write_header = |output: &mut W, size: u64| -> MediaResult<()> {
            let sealed = header::seal(size, &iv, &self.keys.header_key);
            output.seek(SeekFrom::Start(0))?;
            output.write_all(&sealed.bytes)?;
            output.write_all(&sealed.mac)?;
            Ok(())
        };

        write_header(output, 0)?;

        let mut cipher = self.cipher_at(&iv, 0);
        let auth = ChunkAuthenticator::new(&self.keys.chunk_mac_key);
        // Holds plaintext before it is encrypted in place.
        let mut buffer = WipeOnDrop::new(vec![0u8; CHUNK as usize]);
        let mut index = 0u64;
        let mut total = 0u64;

        loop {
            let n = fill(input, &mut buffer)?;
            if n == 0 {
                break;
            }
            total = total
                .checked_add(n as u64)
                .ok_or(MediaLibError::InvalidRange("input size overflow"))?;

            cipher.apply_keystream(&mut buffer[..n]);
            output.write_all(&buffer[..n])?;
            output.write_all(&auth.tag(index, &buffer[..n]))?;
            index = index
                .checked_add(1)
                .ok_or(MediaLibError::InvalidRange("chunk index overflow"))?;
        }

        write_header(output, total)?;
        output.seek(SeekFrom::End(0))?;
        output.flush()?;
        Ok(())
    }

    /// Returns up to `length` plaintext bytes starting at `position`,
    /// authenticating every chunk it touches before decrypting it. Nothing is
    /// written to disk, and only the requested range is returned.
    pub fn decrypt_range<R: Read + Seek + ?Sized>(
        &self,
        position: u64,
        length: usize,
        input: &mut R,
    ) -> MediaResult<Vec<u8>> {
        let info = header::open(input, &self.keys.header_key)?;
        let size = info.plaintext_size;

        if position > size {
            return Err(MediaLibError::InvalidRange("position exceeds plaintext file size"));
        }
        if length == 0 || position == size {
            return Ok(Vec::new());
        }
        let end = position.saturating_add(length as u64).min(size);
        let capacity =
            usize::try_from(end - position).map_err(|_| MediaLibError::InvalidRange("requested range is too large"))?;

        // The physical size must match the header exactly (catches truncation
        // and appended data before any chunk is read).
        let chunk_count = if size == 0 { 0 } else { (size - 1) / CHUNK + 1 };
        let expected = chunk_count
            .checked_mul(TAG_SIZE as u64)
            .and_then(|tags| tags.checked_add(size))
            .and_then(|n| n.checked_add(header::SEALED_SIZE))
            .ok_or(MediaLibError::FileSizeMismatch)?;
        if input.seek(SeekFrom::End(0))? != expected {
            return Err(MediaLibError::FileSizeMismatch);
        }

        let auth = ChunkAuthenticator::new(&self.keys.chunk_mac_key);
        // Both buffers end up holding plaintext and are wiped on every exit.
        let mut output = WipeOnDrop::new(Vec::with_capacity(capacity));
        let mut buffer = WipeOnDrop::new(vec![0u8; CHUNK as usize]);

        for index in position / CHUNK..=(end - 1) / CHUNK {
            let start = index * CHUNK;
            let len = (size - start).min(CHUNK) as usize;
            let offset = index
                .checked_mul(CHUNK + TAG_SIZE as u64)
                .and_then(|o| o.checked_add(header::SEALED_SIZE))
                .ok_or(MediaLibError::InvalidRange("chunk offset overflow"))?;

            input.seek(SeekFrom::Start(offset))?;
            read_exact_or_truncated(input, &mut buffer[..len])?;
            let mut tag = [0u8; TAG_SIZE];
            read_exact_or_truncated(input, &mut tag)?;

            auth.verify(index, &buffer[..len], &tag)?;
            self.cipher_at(&info.iv, start).apply_keystream(&mut buffer[..len]);

            let from = (position.max(start) - start) as usize;
            let to = (end.min(start + len as u64) - start) as usize;
            output.extend_from_slice(&buffer[from..to]);
        }

        Ok(output.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn keys(header_key: u8) -> VideoKeys {
        VideoKeys { header_key: [header_key; 32], chunk_mac_key: [2; 32], encryption_key: [3; 32], iv: [4; 16] }
    }

    fn encrypt(plaintext: &[u8]) -> Vec<u8> {
        let mut out = Cursor::new(Vec::new());
        ChunkedCtr::new(keys(1)).encrypt(&mut Cursor::new(plaintext), &mut out).unwrap();
        out.into_inner()
    }

    fn read(ciphertext: Vec<u8>, position: u64, length: usize) -> MediaResult<Vec<u8>> {
        ChunkedCtr::new(keys(1)).decrypt_range(position, length, &mut Cursor::new(ciphertext))
    }

    fn pattern(n: usize) -> Vec<u8> {
        (0..n).map(|i| (i % 251) as u8).collect()
    }

    #[test]
    fn decrypts_entire_small_file() {
        let plaintext = b"Hello from authenticated AES-CTR!";
        assert_eq!(read(encrypt(plaintext), 0, plaintext.len()).unwrap(), plaintext);
    }

    #[test]
    fn decrypts_range_inside_chunk() {
        let plaintext = pattern(1000);
        assert_eq!(read(encrypt(&plaintext), 123, 200).unwrap(), &plaintext[123..323]);
    }

    #[test]
    fn decrypts_across_chunk_boundary() {
        let chunk = CHUNK as usize;
        let plaintext = pattern(chunk * 2 + 100);
        let start = chunk - 50;
        assert_eq!(read(encrypt(&plaintext), start as u64, 200).unwrap(), &plaintext[start..start + 200]);
    }

    #[test]
    fn scheme_derives_keys_from_media_key() {
        let key = crate::crypto::keys::generate_key();
        let mut out = Cursor::new(Vec::new());
        ChunkedCtrScheme.encrypt(&key, &mut Cursor::new(pattern(5000)), &mut out).unwrap();
        let cipher = ChunkedCtr::from_media_key(&key).unwrap();
        assert_eq!(cipher.decrypt_range(0, 5000, &mut Cursor::new(out.into_inner())).unwrap(), pattern(5000));
    }

    #[test]
    fn rejects_modified_ciphertext() {
        let mut ciphertext = encrypt(&[42u8; 1000]);
        ciphertext[header::SEALED_SIZE as usize] ^= 0x01;
        assert!(matches!(read(ciphertext, 0, 10), Err(MediaLibError::ChunkAuthenticationFailed(0))));
    }

    #[test]
    fn rejects_wrong_header_key() {
        let ciphertext = encrypt(b"Test data");
        let result = ChunkedCtr::new(keys(99)).decrypt_range(0, 9, &mut Cursor::new(ciphertext));
        assert!(matches!(result, Err(MediaLibError::HeaderAuthenticationFailed)));
    }

    #[test]
    fn returns_empty_at_end_of_file() {
        assert!(read(encrypt(b"Test data"), 9, 100).unwrap().is_empty());
    }

    #[test]
    fn rejects_position_past_end_of_file() {
        assert!(matches!(read(encrypt(b"Test data"), 10, 10), Err(MediaLibError::InvalidRange(_))));
    }

    /// Pins the on-disk format: ciphertext produced before the module was
    /// restructured, for a fixed key and plaintext. Existing files on devices
    /// must keep decrypting, so this must never change.
    #[test]
    fn format_is_stable() {
        use sha2::{Digest, Sha256};
        let key = crate::crypto::keys::key_from_bytes(&[9u8; 32]).unwrap();
        let plaintext: Vec<u8> = (0..600_000u32).map(|i| (i % 251) as u8).collect();
        let mut out = Cursor::new(Vec::new());
        ChunkedCtr::from_media_key(&key).unwrap().encrypt(&mut Cursor::new(plaintext), &mut out).unwrap();
        let digest: String = Sha256::digest(out.into_inner()).iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(digest, "5cb2318c4cba6b6547c1cbadac0c66f92a85a7ca8570e54bc08df2bcfc6054d7");
    }

    #[test]
    fn rejects_truncated_file() {
        let mut ciphertext = encrypt(&vec![1u8; CHUNK as usize + 10]);
        ciphertext.truncate(ciphertext.len() - 5);
        assert!(matches!(read(ciphertext, 0, 10), Err(MediaLibError::FileSizeMismatch)));
    }
}
