//! Lossless, container-level metadata removal.
//!
//! Every supported container is a [`MediaFormat`]: it names itself, says
//! which media categories it can carry, recognises its own magic bytes and
//! strips its own metadata (no re-encoding). A [`FormatRegistry`] holds the
//! formats and detects which one a file is, so adding a container means
//! writing one module and listing it in [`FormatRegistry::standard`].
//!
//! Strippers stream to a writer, so they can feed the encryptor directly and
//! plaintext never touches disk. Formats without a stripper are refused with
//! [`MediaLibError::UnsupportedContainer`] rather than stored with metadata.

mod boxio;
mod crc32;
mod exif;
pub(crate) mod flac;
pub(crate) mod gif;
pub(crate) mod isobmff;
pub(crate) mod jpeg;
pub(crate) mod mpeg_audio;
pub(crate) mod png;
mod unsupported;
pub(crate) mod wav;
pub(crate) mod webp;

#[cfg(test)]
pub(crate) mod testutil;

use std::io::{SeekFrom, Write};

use crate::error::{MediaLibError, MediaResult};
use crate::media::MediaCategory;

pub use boxio::ReadSeek;

/// Removes metadata from one container format.
pub trait MetadataStripper: Sync {
    /// Reads the whole of `input` (from the start) and writes a
    /// metadata-free copy to `out`.
    fn strip(&self, input: &mut dyn ReadSeek, out: &mut dyn Write) -> MediaResult<()>;
}

/// A container format the library knows how to recognise and strip.
pub trait MediaFormat: MetadataStripper {
    fn name(&self) -> &'static str;

    /// The media categories this container can legitimately carry.
    fn categories(&self) -> &'static [MediaCategory];

    /// Whether `head`, the first bytes of the file, identify this format.
    fn sniff(&self, head: &[u8]) -> bool;

    /// Whether this format may be preceded by an ID3v2 tag (audio only).
    fn may_follow_id3(&self) -> bool {
        false
    }
}

/// An ordered set of formats used to identify files.
pub struct FormatRegistry {
    formats: &'static [&'static dyn MediaFormat],
}

static STANDARD: FormatRegistry = FormatRegistry::new(&[
    &jpeg::Jpeg,
    &png::Png,
    &gif::Gif,
    &webp::WebP,
    &wav::Wav,
    &flac::Flac,
    &isobmff::IsoBmff,
    // Last: an MPEG frame sync is the loosest signature.
    &mpeg_audio::MpegAudio,
]);

impl FormatRegistry {
    pub const fn new(formats: &'static [&'static dyn MediaFormat]) -> Self {
        Self { formats }
    }

    /// Every format this library supports.
    pub fn standard() -> &'static FormatRegistry {
        &STANDARD
    }

    /// Identifies `input` from its magic bytes (never trusting a MIME type)
    /// and rewinds it.
    pub fn detect(&self, input: &mut dyn ReadSeek) -> MediaResult<&'static dyn MediaFormat> {
        let start = mpeg_audio::skip_id3v2(input)?;

        let mut head = [0u8; 16];
        input.seek(SeekFrom::Start(start))?;
        let n = boxio::read_up_to(input, &mut head)?;
        input.seek(SeekFrom::Start(0))?;
        let head = &head[..n];

        self.formats
            .iter()
            .copied()
            .find(|format| (start == 0 || format.may_follow_id3()) && format.sniff(head))
            .ok_or_else(|| {
                let name = if start > 0 { None } else { unsupported::identify(head) };
                MediaLibError::UnsupportedContainer(name.unwrap_or("unknown").to_string())
            })
    }
}

/// Strips an in-memory file.
pub fn strip_to_vec(stripper: &dyn MetadataStripper, data: Vec<u8>) -> MediaResult<Vec<u8>> {
    let mut input = std::io::Cursor::new(data);
    let mut out = Vec::new();
    stripper.strip(&mut input, &mut out)?;
    Ok(out)
}

/// Reads all of `input`; used by strippers that buffer their input.
pub(crate) fn read_whole(input: &mut dyn ReadSeek, cap: u64) -> MediaResult<Vec<u8>> {
    let len = boxio::stream_len(input)?;
    if len > cap {
        return Err(MediaLibError::ContainerTooLarge);
    }
    input.seek(SeekFrom::Start(0))?;
    let mut data = Vec::with_capacity(len as usize);
    input.read_to_end(&mut data)?;
    Ok(data)
}

/// Images are buffered whole; they are decrypted whole later anyway.
pub(crate) const IMAGE_CAP: u64 = 64 * 1024 * 1024;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn detect(bytes: &[u8]) -> MediaResult<&'static str> {
        FormatRegistry::standard().detect(&mut Cursor::new(bytes.to_vec())).map(|f| f.name())
    }

    #[test]
    fn detects_by_magic() {
        assert_eq!(detect(&[0xFF, 0xD8, 0xFF, 0xE0, 0, 0]).unwrap(), "JPEG");
        assert_eq!(detect(b"\x89PNG\r\n\x1a\n....").unwrap(), "PNG");
        assert_eq!(detect(b"GIF89a......").unwrap(), "GIF");
        assert_eq!(detect(b"RIFF\0\0\0\0WEBPVP8 ").unwrap(), "WebP");
        assert_eq!(detect(b"RIFF\0\0\0\0WAVEfmt ").unwrap(), "WAV");
        assert_eq!(detect(b"fLaC\0\0\0\x22").unwrap(), "FLAC");
        assert_eq!(detect(b"\0\0\0\x18ftypisom").unwrap(), "ISO-BMFF (MP4/MOV/M4A)");
        assert_eq!(detect(&[0xFF, 0xFB, 0x90, 0x00]).unwrap(), "MPEG audio (MP3/AAC)");
        assert_eq!(detect(&[0xFF, 0xF1, 0x50, 0x80]).unwrap(), "MPEG audio (MP3/AAC)");
    }

    #[test]
    fn detects_audio_behind_id3() {
        let mut mp3 = testutil::id3v2(b"TIT2secret");
        mp3.extend_from_slice(&[0xFF, 0xFB, 0x90, 0x00]);
        assert_eq!(detect(&mp3).unwrap(), "MPEG audio (MP3/AAC)");

        let mut flac = testutil::id3v2(b"TIT2secret");
        flac.extend_from_slice(b"fLaC");
        assert_eq!(detect(&flac).unwrap(), "FLAC");

        // Only audio formats may follow an ID3 tag.
        let mut jpeg = testutil::id3v2(b"TIT2secret");
        jpeg.extend_from_slice(&[0xFF, 0xD8, 0xFF]);
        assert!(matches!(detect(&jpeg), Err(MediaLibError::UnsupportedContainer(_))));
    }

    #[test]
    fn rejects_formats_without_a_stripper() {
        for (bytes, name) in [
            (&b"OggS\0\x02...."[..], "Ogg"),
            (&[0x1A, 0x45, 0xDF, 0xA3, 0, 0], "Matroska/WebM"),
            (b"#!AMR\n", "AMR"),
            (b"hello world", "unknown"),
        ] {
            match detect(bytes) {
                Err(MediaLibError::UnsupportedContainer(n)) => assert_eq!(n, name),
                other => panic!("expected rejection, got {other:?}"),
            }
        }
    }

    #[test]
    fn categories_constrain_formats() {
        let accepts = |format: &dyn MediaFormat, c| format.categories().contains(&c);
        assert!(accepts(&jpeg::Jpeg, MediaCategory::Image));
        assert!(!accepts(&jpeg::Jpeg, MediaCategory::Video));
        assert!(accepts(&isobmff::IsoBmff, MediaCategory::Video));
        assert!(accepts(&isobmff::IsoBmff, MediaCategory::Audio));
        // A HEIF/AVIF image is ISO-BMFF too, and is refused this way.
        assert!(!accepts(&isobmff::IsoBmff, MediaCategory::Image));
        assert!(!accepts(&flac::Flac, MediaCategory::Video));
    }

    #[test]
    fn custom_registries_are_supported() {
        static ONLY_PNG: FormatRegistry = FormatRegistry::new(&[&png::Png]);
        let jpeg = Cursor::new(vec![0xFF, 0xD8, 0xFF, 0xE0]);
        assert!(ONLY_PNG.detect(&mut jpeg.clone()).is_err());
        assert!(FormatRegistry::standard().detect(&mut jpeg.clone()).is_ok());
    }
}
