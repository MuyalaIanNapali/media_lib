//! Whole-file authenticated encryption for images, built on libsodium's
//! `crypto_secretstream_xchacha20poly1305` (via dryoc).
//!
//! Layout: `[24-byte header]` then, per chunk, `[4-byte BE length][ciphertext]`.
//! The last chunk is tagged FINAL, so truncation and appended data are
//! detected as well as tampering.

use std::io::{self, Read, Write};

use dryoc::dryocstream::{DryocStream, Header, Tag};

use super::keys::MediaKey;
use super::wipe::{wipe, WipeOnDrop};
use super::{EncryptionScheme, WriteSeek};
use crate::error::{MediaLibError, MediaResult};
use crate::io_util::{fill, read_exact_or_truncated};

/// Plaintext bytes per chunk. The overhead is [`ABYTES`] per chunk.
pub const CHUNK_SIZE: usize = 64 * 1024;
/// Per-chunk authentication tag + stream byte overhead.
const ABYTES: usize = 17;
const HEADER_LEN: usize = 24;

/// The image [`EncryptionScheme`].
pub struct SecretStream;

impl EncryptionScheme for SecretStream {
    fn encrypt(&self, key: &MediaKey, input: &mut dyn Read, output: &mut dyn WriteSeek) -> MediaResult<()> {
        encrypt_stream(key, input, output)
    }
}

/// Encrypts `input` to `output` as a chained secretstream.
pub fn encrypt_stream<R: Read, W: Write>(key: &MediaKey, mut input: R, mut output: W) -> MediaResult<()> {
    let (mut stream, header): (_, Header) = DryocStream::init_push(key);
    output.write_all(&header[..])?;

    // Double-buffer to look one chunk ahead and know which one is FINAL.
    let mut cur = WipeOnDrop::new(vec![0u8; CHUNK_SIZE]);
    let mut nxt = WipeOnDrop::new(vec![0u8; CHUNK_SIZE]);
    let mut cur_len = fill(&mut input, &mut cur)?;

    loop {
        let nxt_len = fill(&mut input, &mut nxt)?;
        let is_last = nxt_len == 0;
        let tag = if is_last { Tag::Final } else { Tag::Message };

        let ct = stream
            .push_to_vec(&cur[..cur_len], None, tag)
            .map_err(|e| MediaLibError::EncryptionFailed(e.to_string()))?;
        let len = u32::try_from(ct.len()).map_err(|_| MediaLibError::InvalidChunkLength(ct.len()))?;
        output.write_all(&len.to_be_bytes())?;
        output.write_all(&ct)?;

        if is_last {
            break;
        }
        std::mem::swap(&mut cur, &mut nxt);
        cur_len = nxt_len;
    }

    output.flush()?;
    Ok(())
}

/// Decrypts a stream produced by [`encrypt_stream`]. Fails on tampering or
/// reordering, on truncation before FINAL, and on data after FINAL.
pub fn decrypt_stream<R: Read>(key: &MediaKey, mut input: R) -> MediaResult<Vec<u8>> {
    let mut header_buf = [0u8; HEADER_LEN];
    read_exact_or_truncated(&mut input, &mut header_buf)?;
    let header = Header::try_from(&header_buf[..]).map_err(|_| MediaLibError::InvalidHeader)?;
    let mut stream = DryocStream::init_pull(key, &header);

    // Partial plaintext is wiped if a later chunk fails.
    let mut output = WipeOnDrop::new(Vec::new());
    let mut chunk_index = 0u64;

    loop {
        let mut len_buf = [0u8; 4];
        read_exact_or_truncated(&mut input, &mut len_buf)?;

        // The length prefix is outside the AEAD; bound it so a corrupt prefix
        // can't force a huge allocation. Tampering is caught by the AEAD.
        let clen = u32::from_be_bytes(len_buf) as usize;
        if clen == 0 || clen > CHUNK_SIZE + ABYTES {
            return Err(MediaLibError::InvalidChunkLength(clen));
        }

        let mut cbuf = vec![0u8; clen];
        read_exact_or_truncated(&mut input, &mut cbuf)?;

        let (mut pt, tag) = stream
            .pull_to_vec(&cbuf, None)
            .map_err(|_| MediaLibError::ChunkAuthenticationFailed(chunk_index))?;
        output.extend_from_slice(&pt);
        wipe(&mut pt);
        chunk_index += 1;

        if tag == Tag::Final {
            let mut extra = [0u8; 1];
            match input.read(&mut extra) {
                Ok(0) => {}
                Ok(_) => return Err(MediaLibError::TrailingData),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e.into()),
            }
            return Ok(output.into_inner());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use crate::crypto::keys::{generate_key, key_from_bytes};

    fn encrypt(key: &MediaKey, data: &[u8]) -> Vec<u8> {
        let mut ciphertext = Vec::new();
        encrypt_stream(key, Cursor::new(data.to_vec()), &mut ciphertext).unwrap();
        ciphertext
    }

    #[test]
    fn round_trips_across_chunk_boundaries() {
        for n in [0usize, 1, CHUNK_SIZE - 1, CHUNK_SIZE, CHUNK_SIZE + 1, CHUNK_SIZE * 3 + 7] {
            let key = generate_key();
            let data: Vec<u8> = (0..n).map(|i| (i % 251) as u8).collect();
            assert_eq!(decrypt_stream(&key, Cursor::new(encrypt(&key, &data))).unwrap(), data);
        }
    }

    #[test]
    fn scheme_matches_free_function() {
        let key = generate_key();
        let mut out = Cursor::new(Vec::new());
        SecretStream.encrypt(&key, &mut Cursor::new(b"hello".to_vec()), &mut out).unwrap();
        assert_eq!(decrypt_stream(&key, Cursor::new(out.into_inner())).unwrap(), b"hello");
    }

    #[test]
    fn detects_tampering() {
        let key = generate_key();
        let mut ciphertext = encrypt(&key, &vec![7u8; CHUNK_SIZE * 2]);
        // Past the 24-byte header and the 4-byte length prefix.
        ciphertext[24 + 4 + 10] ^= 0xff;
        assert!(matches!(
            decrypt_stream(&key, Cursor::new(ciphertext)),
            Err(MediaLibError::ChunkAuthenticationFailed(0))
        ));
    }

    #[test]
    fn detects_truncation() {
        let key = generate_key();
        let mut ciphertext = encrypt(&key, &vec![3u8; CHUNK_SIZE * 3]);
        ciphertext.truncate(ciphertext.len() - 20);
        assert!(matches!(decrypt_stream(&key, Cursor::new(ciphertext)), Err(MediaLibError::Truncated)));
    }

    #[test]
    fn detects_trailing_data() {
        let key = generate_key();
        let mut ciphertext = encrypt(&key, &[9u8; 100]);
        ciphertext.push(0x00);
        assert!(matches!(decrypt_stream(&key, Cursor::new(ciphertext)), Err(MediaLibError::TrailingData)));
    }

    #[test]
    fn wrong_key_fails() {
        let ciphertext = encrypt(&generate_key(), &vec![5u8; CHUNK_SIZE]);
        assert!(matches!(
            decrypt_stream(&generate_key(), Cursor::new(ciphertext)),
            Err(MediaLibError::ChunkAuthenticationFailed(0))
        ));
    }

    #[test]
    fn rebuilt_key_decrypts() {
        // Mimics storing the key bytes and reloading them later.
        let key = generate_key();
        let data = vec![42u8; CHUNK_SIZE + 123];
        let ciphertext = encrypt(&key, &data);
        let rebuilt = key_from_bytes(&key[..]).unwrap();
        assert_eq!(decrypt_stream(&rebuilt, Cursor::new(ciphertext)).unwrap(), data);
    }

    #[test]
    fn file_to_file_round_trip() {
        use std::fs::{self, File};
        use std::io::BufWriter;

        let path = std::env::temp_dir().join(format!("media_lib_ss_{}.enc", std::process::id()));
        let key = generate_key();
        let data: Vec<u8> = (0..CHUNK_SIZE * 2 + 500).map(|i| (i % 253) as u8).collect();

        encrypt_stream(&key, Cursor::new(data.clone()), BufWriter::new(File::create(&path).unwrap())).unwrap();
        let decrypted = decrypt_stream(&key, File::open(&path).unwrap()).unwrap();
        let _ = fs::remove_file(&path);
        assert_eq!(decrypted, data);
    }
}
