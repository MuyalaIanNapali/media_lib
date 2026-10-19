//! Strip → encrypt.
//!
//! [`StripEncryptPipeline`] is composed from a [`FormatRegistry`] (which
//! formats exist and how to strip them) and a [`SchemeSelector`] (how each
//! media category is encrypted); it depends only on those abstractions.
//!
//! The stripper runs on a scoped thread writing into an OS pipe while the
//! encryptor reads the other end, so plaintext is never written to disk and
//! memory stays bounded (images and MP4 `moov` boxes are the only things
//! buffered whole).

use std::io::{self, BufWriter, Read, Seek, Write};
use std::thread;

use crate::crypto::keys::MediaKey;
use crate::crypto::{SchemeSelector, StandardSchemes};
use crate::error::{MediaLibError, MediaResult};
use crate::media::MediaCategory;
use crate::strip::{FormatRegistry, MetadataStripper};

/// Plaintext in flight between the threads: this buffer plus the OS pipe.
const PIPE_BUFFER: usize = 64 * 1024;

pub struct StripEncryptPipeline<'a> {
    formats: &'a FormatRegistry,
    schemes: &'a dyn SchemeSelector,
}

impl Default for StripEncryptPipeline<'static> {
    fn default() -> Self {
        Self::new(FormatRegistry::standard(), &StandardSchemes)
    }
}

impl<'a> StripEncryptPipeline<'a> {
    pub fn new(formats: &'a FormatRegistry, schemes: &'a dyn SchemeSelector) -> Self {
        Self { formats, schemes }
    }

    /// Removes `input`'s metadata and encrypts the result to `output`.
    ///
    /// Fails with `UnsupportedContainer` when the format can't be stripped or
    /// doesn't match `mime`'s category, so nothing is stored with metadata.
    /// On error `output` may be partially written and must be discarded.
    pub fn run<R, W>(&self, mime: &str, key: &MediaKey, mut input: R, mut output: W) -> MediaResult<()>
    where
        R: Read + Seek + Send,
        W: Write + Seek,
    {
        let category = MediaCategory::from_mime(mime)?;
        let format = self.formats.detect(&mut input)?;
        if !format.categories().contains(&category) {
            return Err(MediaLibError::UnsupportedContainer(format!("{} declared as {mime}", format.name())));
        }
        let scheme = self.schemes.scheme_for(category);

        stream_stripped(format, input, |plaintext| scheme.encrypt(key, plaintext, &mut output))
    }
}

/// Runs `stripper` over `input` on a scoped thread and hands its output to
/// `consume` through an OS pipe.
///
/// A failing stripper closes the pipe early, which `consume` would see as a
/// (shorter) complete input — so the stripper's error always wins, except for
/// the broken pipe it gets when `consume` fails first.
fn stream_stripped<R: Read + Seek + Send>(
    stripper: &dyn MetadataStripper,
    mut input: R,
    consume: impl FnOnce(&mut dyn Read) -> MediaResult<()>,
) -> MediaResult<()> {
    let (mut reader, writer) = io::pipe()?;

    thread::scope(|scope| {
        let stripping = scope.spawn(move || -> MediaResult<()> {
            let mut writer = BufWriter::with_capacity(PIPE_BUFFER, writer);
            stripper.strip(&mut input, &mut writer)?;
            writer.flush()?;
            Ok(())
            // Dropping the writer closes the pipe: the consumer sees EOF.
        });

        let consumed = consume(&mut reader);
        // Unblocks a stripper still writing (it gets EPIPE) before joining.
        drop(reader);

        let stripped = stripping
            .join()
            .map_err(|_| MediaLibError::EncryptionFailed("metadata stripper panicked".into()))?;

        match (stripped, consumed) {
            (Err(MediaLibError::Io(e)), Err(consumer_error)) if e.kind() == io::ErrorKind::BrokenPipe => {
                Err(consumer_error)
            }
            (Err(strip_error), _) => Err(strip_error),
            (Ok(()), consumed) => consumed,
        }
    })
}

/// [`StripEncryptPipeline::run`] with the standard formats and schemes.
pub fn strip_and_encrypt<R, W>(mime: &str, key: &MediaKey, input: R, output: W) -> MediaResult<()>
where
    R: Read + Seek + Send,
    W: Write + Seek,
{
    StripEncryptPipeline::default().run(mime, key, input, output)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use crate::crypto::chunked_ctr::ChunkedCtr;
    use crate::crypto::keys::generate_key;
    use crate::crypto::secretstream::decrypt_stream;
    use crate::crypto::{EncryptionScheme, WriteSeek};

    /// A scheme that copies plaintext through, to test the pipeline alone.
    struct Plain;
    impl EncryptionScheme for Plain {
        fn encrypt(&self, _: &MediaKey, input: &mut dyn Read, output: &mut dyn WriteSeek) -> MediaResult<()> {
            io::copy(input, output)?;
            Ok(())
        }
    }
    struct AlwaysPlain;
    impl SchemeSelector for AlwaysPlain {
        fn scheme_for(&self, _: MediaCategory) -> &dyn EncryptionScheme {
            &Plain
        }
    }

    #[test]
    fn injected_scheme_receives_stripped_plaintext() {
        let pipeline = StripEncryptPipeline::new(FormatRegistry::standard(), &AlwaysPlain);
        let mut out = Cursor::new(Vec::new());
        pipeline
            .run("image/jpeg", &generate_key(), Cursor::new(crate::strip::testutil::jpeg_with_metadata(6)), &mut out)
            .unwrap();
        let out = out.into_inner();
        assert!(out.starts_with(&[0xFF, 0xD8]));
        assert!(!crate::strip::testutil::contains(&out, b"GPSSECRET"));
    }

    #[test]
    fn injected_registry_limits_formats() {
        static ONLY_JPEG: FormatRegistry = FormatRegistry::new(&[&crate::strip::jpeg::Jpeg]);
        let pipeline = StripEncryptPipeline::new(&ONLY_JPEG, &AlwaysPlain);
        let png = crate::strip::testutil::png_with_metadata(1);
        let result = pipeline.run("image/png", &generate_key(), Cursor::new(png), Cursor::new(Vec::new()));
        assert!(matches!(result, Err(MediaLibError::UnsupportedContainer(_))));
    }

    #[test]
    fn pipeline_rejects_unsupported_mime() {
        let key = generate_key();
        let result = strip_and_encrypt(
            "text/plain",
            &key,
            Cursor::new(vec![1u8, 2, 3]),
            Cursor::new(Vec::new()),
        );
        assert!(matches!(result, Err(MediaLibError::UnsupportedMimeType(m)) if m == "text/plain"));
    }

    /// A 300 KB MP4 (spans two AES-CTR chunks) with metadata to strip.
    fn large_mp4() -> Vec<u8> {
        use crate::strip::testutil::{self, Mp4Layout};
        let mut file = testutil::mp4(Mp4Layout::MoovAtEnd);
        // Grow mdat's payload; its samples are at the start, so offsets hold.
        let mdat = testutil::find(&file, b"mdat").unwrap() - 4;
        let old = u32::from_be_bytes(file[mdat..mdat + 4].try_into().unwrap()) as usize;
        let filler: Vec<u8> = (0..300_000).map(|i| (i % 249) as u8).collect();
        let end = mdat + old;
        file.splice(end..end, filler.iter().copied());
        file[mdat..mdat + 4].copy_from_slice(&((old + filler.len()) as u32).to_be_bytes());
        // moov sits after mdat; its stco offsets point before the filler.
        file
    }

    #[test]
    fn video_pipeline_roundtrips_through_range_reads() {
        // Exercises VideoKeys::derive on both the encrypt and decrypt sides.
        let key = generate_key();
        let input = large_mp4();
        let data = crate::strip::strip_to_vec(&crate::strip::isobmff::IsoBmff, input.clone()).unwrap();
        assert!(!crate::strip::testutil::contains(&data, b"secret"));

        let mut ciphertext = Cursor::new(Vec::new());
        strip_and_encrypt("video/mp4", &key, Cursor::new(input), &mut ciphertext)
            .unwrap();

        let range = ChunkedCtr::from_media_key(&key)
            .unwrap()
            .decrypt_range(262_000, 1_000, &mut Cursor::new(ciphertext.into_inner()))
            .unwrap();

        assert_eq!(range, data[262_000..263_000]);
    }

    #[test]
    fn image_pipeline_strips_before_encrypting() {
        let key = generate_key();
        let jpeg = crate::strip::testutil::jpeg_with_metadata(6);
        let mut ciphertext = Cursor::new(Vec::new());
        strip_and_encrypt("image/jpeg", &key, Cursor::new(jpeg), &mut ciphertext).unwrap();

        let plaintext = decrypt_stream(&key, Cursor::new(ciphertext.into_inner())).unwrap();
        assert!(!crate::strip::testutil::contains(&plaintext, b"GPSSECRET"));
        assert!(crate::strip::testutil::contains(&plaintext, b"Exif\0\0"));
    }

    #[test]
    fn pipeline_rejects_unsupported_and_mismatched_containers() {
        let key = generate_key();
        let run = |mime: &str, data: Vec<u8>| {
            strip_and_encrypt(mime, &key, Cursor::new(data), Cursor::new(Vec::new()))
        };
        let ogg = b"OggS\0\x02\0\0\0\0\0\0\0\0".to_vec();
        let err = run("audio/ogg", ogg).unwrap_err();
        assert!(matches!(err, MediaLibError::UnsupportedContainer(_)));
        assert_eq!(err.code(), 16);

        // A JPEG claiming to be video.
        let err = run("video/mp4", crate::strip::testutil::jpeg_with_metadata(1)).unwrap_err();
        assert!(matches!(err, MediaLibError::UnsupportedContainer(_)));
    }

    #[test]
    fn strip_failure_is_reported_not_truncated() {
        // A JPEG cut off mid-scan: the stripper fails after the encryptor has
        // started; the pipeline must surface that rather than succeed.
        let key = generate_key();
        let mut jpeg = crate::strip::testutil::jpeg_with_metadata(1);
        let cut = crate::strip::testutil::find(&jpeg, crate::strip::testutil::JPEG_SCAN).unwrap() + 3;
        jpeg.truncate(cut);
        let result =
            strip_and_encrypt("image/jpeg", &key, Cursor::new(jpeg), Cursor::new(Vec::new()));
        assert!(matches!(result, Err(MediaLibError::MalformedContainer(_))));
    }

    // ------------------------------------------------------------------
    // Real files: need ffmpeg and ffprobe on PATH; `cargo test -- --ignored`.
    // ------------------------------------------------------------------

    const TAGS: [&str; 6] = [
        "-metadata", "title=secret title",
        "-metadata", "artist=secret artist",
        "-metadata", "location=+12.3456+045.6789/",
    ];

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("media_lib_strip_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    /// Runs ffmpeg writing to a file named `name` and returns its bytes.
    fn ffmpeg_make(name: &str, args: &[&str]) -> Vec<u8> {
        let path = scratch(name);
        let out = std::process::Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-y"])
            .args(args)
            .arg(&path)
            .output()
            .expect("failed to run ffmpeg");
        assert!(out.status.success(), "ffmpeg failed: {}", String::from_utf8_lossy(&out.stderr));
        let data = std::fs::read(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        data
    }

    /// strip → encrypt → decrypt, returning the stripped plaintext.
    fn strip_encrypt_decrypt(mime: &str, input: Vec<u8>) -> Vec<u8> {
        let key = generate_key();
        let mut ciphertext = Cursor::new(Vec::new());
        strip_and_encrypt(mime, &key, Cursor::new(input), &mut ciphertext).unwrap();
        let ciphertext = ciphertext.into_inner();

        if mime.starts_with("image/") {
            decrypt_stream(&key, Cursor::new(ciphertext)).unwrap()
        } else {
            let len = ciphertext.len();
            ChunkedCtr::from_media_key(&key).unwrap().decrypt_range(0, len, &mut Cursor::new(ciphertext)).unwrap()
        }
    }

    /// The stripped file has no tags left and still decodes cleanly.
    fn assert_clean_and_decodable(name: &str, plaintext: &[u8]) {
        use crate::strip::testutil::contains;
        assert!(!contains(plaintext, b"secret"), "{name}: tag bytes remain");
        assert!(!contains(plaintext, b"+12.3456"), "{name}: location remains");

        let path = scratch(name);
        std::fs::write(&path, plaintext).unwrap();

        let probe = std::process::Command::new("ffprobe")
            .args(["-v", "error", "-show_entries", "format_tags:stream_tags", "-of", "default=nw=1"])
            .arg(&path)
            .output()
            .unwrap();
        let tags = String::from_utf8_lossy(&probe.stdout).to_lowercase();
        assert!(probe.status.success(), "{name}: ffprobe failed");
        assert!(!tags.contains("secret") && !tags.contains("location"), "{name}: tags remain:\n{tags}");

        let decode = std::process::Command::new("ffmpeg")
            .args(["-v", "error", "-i"])
            .arg(&path)
            .args(["-f", "null", "-"])
            .output()
            .unwrap();
        let _ = std::fs::remove_file(&path);
        assert!(
            decode.status.success() && decode.stderr.is_empty(),
            "{name}: decode errors: {}",
            String::from_utf8_lossy(&decode.stderr)
        );
    }

    fn real_file_roundtrip(name: &str, mime: &str, args: &[&str]) {
        let mut all: Vec<&str> = args.to_vec();
        all.extend_from_slice(&TAGS);
        let input = ffmpeg_make(name, &all);
        assert!(crate::strip::testutil::contains(&input, b"secret"), "{name}: fixture lacks tags");
        let plaintext = strip_encrypt_decrypt(mime, input);
        assert_clean_and_decodable(name, &plaintext);
    }

    const AV_SOURCE: [&str; 8] = [
        "-f", "lavfi", "-i", "testsrc=duration=2:size=160x120:rate=15",
        "-f", "lavfi", "-i", "sine=frequency=440:duration=2",
    ];
    const AUDIO_SOURCE: [&str; 4] = ["-f", "lavfi", "-i", "sine=frequency=440:duration=2"];

    fn with(base: &[&'static str], extra: &[&'static str]) -> Vec<&'static str> {
        [base, extra].concat()
    }

    #[test]
    #[ignore = "requires ffmpeg and ffprobe on PATH"]
    fn real_mp4_faststart() {
        real_file_roundtrip("a.mp4", "video/mp4", &with(&AV_SOURCE, &["-c:v", "libx264", "-c:a", "aac", "-movflags", "+faststart"]));
    }

    #[test]
    #[ignore = "requires ffmpeg and ffprobe on PATH"]
    fn real_mp4_moov_at_end() {
        real_file_roundtrip("b.mp4", "video/mp4", &with(&AV_SOURCE, &["-c:v", "libx264", "-c:a", "aac"]));
    }

    #[test]
    #[ignore = "requires ffmpeg and ffprobe on PATH"]
    fn real_mov() {
        real_file_roundtrip("c.mov", "video/quicktime", &with(&AV_SOURCE, &["-c:v", "libx264", "-c:a", "aac"]));
    }

    #[test]
    #[ignore = "requires ffmpeg and ffprobe on PATH"]
    fn real_m4a() {
        real_file_roundtrip("d.m4a", "audio/mp4", &with(&AUDIO_SOURCE, &["-c:a", "aac"]));
    }

    #[test]
    #[ignore = "requires ffmpeg and ffprobe on PATH"]
    fn real_fragmented_m4a() {
        real_file_roundtrip(
            "e.m4a",
            "audio/mp4",
            &with(&AUDIO_SOURCE, &["-c:a", "aac", "-movflags", "+frag_keyframe+empty_moov+default_base_moof"]),
        );
    }

    #[test]
    #[ignore = "requires ffmpeg and ffprobe on PATH"]
    fn real_mp3_with_id3v2_and_id3v1() {
        real_file_roundtrip(
            "f.mp3",
            "audio/mpeg",
            &with(&AUDIO_SOURCE, &["-c:a", "libmp3lame", "-id3v2_version", "3", "-write_id3v1", "1"]),
        );
    }

    #[test]
    #[ignore = "requires ffmpeg and ffprobe on PATH"]
    fn real_flac() {
        real_file_roundtrip("g.flac", "audio/flac", &with(&AUDIO_SOURCE, &["-c:a", "flac"]));
    }

    #[test]
    #[ignore = "requires ffmpeg and ffprobe on PATH"]
    fn real_wav() {
        real_file_roundtrip("h.wav", "audio/wav", &with(&AUDIO_SOURCE, &["-c:a", "pcm_s16le"]));
    }

    #[test]
    #[ignore = "requires ffmpeg and ffprobe on PATH"]
    fn real_jpeg_and_png_still_decode() {
        for (name, mime, codec) in [("i.jpg", "image/jpeg", "mjpeg"), ("j.png", "image/png", "png")] {
            let input = ffmpeg_make(
                name,
                &["-f", "lavfi", "-i", "testsrc=size=320x240", "-frames:v", "1", "-c:v", codec, "-update", "1"],
            );
            assert_clean_and_decodable(name, &strip_encrypt_decrypt(mime, input));
        }
    }

    #[test]
    #[ignore = "requires ffmpeg and ffprobe on PATH"]
    fn real_ogg_opus_is_rejected() {
        let input = ffmpeg_make("k.ogg", &with(&AUDIO_SOURCE, &["-c:a", "libopus"]));
        let err = strip_and_encrypt(
            "audio/ogg", &generate_key(), Cursor::new(input), Cursor::new(Vec::new()),
        )
        .unwrap_err();
        assert_eq!(err.code(), 16);
    }
}
