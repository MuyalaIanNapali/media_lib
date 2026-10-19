//! File-level operations for platform callers (the JNI layer).
//!
//! Keeps all file, key and range handling in plain Rust so the FFI adapter
//! only converts types.

use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;

use crate::crypto::chunked_ctr::ChunkedCtr;
use crate::crypto::keys::{generate_key, key_from_bytes, MediaKey};
use crate::crypto::secretstream::decrypt_stream;
use crate::error::{MediaLibError, MediaResult};
use crate::fs_util::write_atomically;
use crate::pipeline::StripEncryptPipeline;

pub struct MediaService<'a> {
    pipeline: StripEncryptPipeline<'a>,
}

impl Default for MediaService<'static> {
    fn default() -> Self {
        Self::new(StripEncryptPipeline::default())
    }
}

impl<'a> MediaService<'a> {
    pub fn new(pipeline: StripEncryptPipeline<'a>) -> Self {
        Self { pipeline }
    }

    /// Strips and encrypts `input` into `dir/name` under a fresh key, which is
    /// returned. The file only appears under its final name once complete.
    pub fn encrypt_to_file<R: Read + Seek + Send>(
        &self,
        input: R,
        mime: &str,
        dir: &Path,
        name: &str,
    ) -> MediaResult<MediaKey> {
        let key = generate_key();
        write_atomically(dir, name, |file| self.pipeline.run(mime, &key, input, file))?;
        Ok(key)
    }

    /// Decrypts a whole image file into memory.
    pub fn decrypt_image(&self, path: &Path, key: &[u8]) -> MediaResult<Vec<u8>> {
        let key = key_from_bytes(key)?;
        decrypt_stream(&key, File::open(path)?)
    }

    /// Decrypts up to `length` plaintext bytes at `position` of an audio or
    /// video file, touching only the chunks that cover that range. Signed
    /// arguments mirror the JVM's `long`.
    pub fn read_range(&self, path: &Path, position: i64, length: i64, key: &[u8]) -> MediaResult<Vec<u8>> {
        let position =
            u64::try_from(position).map_err(|_| MediaLibError::InvalidRange("position must be non-negative"))?;
        let length = usize::try_from(length).map_err(|_| MediaLibError::InvalidRange("length must be non-negative"))?;
        let key = key_from_bytes(key)?;
        ChunkedCtr::from_media_key(&key)?.decrypt_range(position, length, &mut File::open(path)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strip::testutil::{self, contains, Mp4Layout};
    use std::io::Cursor;

    fn scratch() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("media_lib_svc_{}_{:?}", std::process::id(), std::thread::current().id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn image_round_trip_through_files() {
        let dir = scratch();
        let service = MediaService::default();
        let key = service
            .encrypt_to_file(Cursor::new(testutil::jpeg_with_metadata(6)), "image/jpeg", &dir, "a.enc")
            .unwrap();
        let image = service.decrypt_image(&dir.join("a.enc"), &key[..]).unwrap();
        assert!(image.starts_with(&[0xFF, 0xD8]));
        assert!(!contains(&image, b"GPSSECRET"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn video_range_through_files() {
        let dir = scratch();
        let service = MediaService::default();
        let key = service
            .encrypt_to_file(Cursor::new(testutil::mp4(Mp4Layout::FastStart)), "video/mp4", &dir, "v.enc")
            .unwrap();
        let head = service.read_range(&dir.join("v.enc"), 0, 8, &key[..]).unwrap();
        assert_eq!(&head[4..8], b"ftyp");
        assert!(matches!(
            service.read_range(&dir.join("v.enc"), -1, 8, &key[..]),
            Err(MediaLibError::InvalidRange(_))
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn failed_encryption_leaves_no_file() {
        let dir = scratch();
        let result = MediaService::default().encrypt_to_file(Cursor::new(b"OggS....".to_vec()), "audio/ogg", &dir, "o.enc");
        assert!(matches!(result, Err(MediaLibError::UnsupportedContainer(_))));
        assert!(std::fs::read_dir(&dir).unwrap().next().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
