//! WAV: keep only `fmt `, `fact` and `data`; drop LIST/INFO, id3, bext,
//! iXML, XMP and every other chunk.

use std::io::Write;

use super::boxio::{copy_range, le_u32, read_at, read_up_to, stream_len};
use super::{MediaFormat, MetadataStripper, ReadSeek};
use crate::media::MediaCategory;
use crate::error::{MediaLibError, MediaResult};
use std::io::SeekFrom;

pub(crate) struct Wav;

impl MediaFormat for Wav {
    fn name(&self) -> &'static str {
        "WAV"
    }

    fn categories(&self) -> &'static [MediaCategory] {
        &[MediaCategory::Audio]
    }

    fn sniff(&self, head: &[u8]) -> bool {
        head.len() >= 12 && &head[0..4] == b"RIFF" && &head[8..12] == b"WAVE"
    }
}

const KEPT: [&[u8; 4]; 3] = [b"fmt ", b"fact", b"data"];

impl MetadataStripper for Wav {
    fn strip(&self, input: &mut dyn ReadSeek, out: &mut dyn Write) -> MediaResult<()> {
        let file_len = stream_len(input)?;
        let mut header = [0u8; 12];
        read_at(input, 0, &mut header)?;
        if &header[0..4] != b"RIFF" || &header[8..12] != b"WAVE" {
            return Err(MediaLibError::MalformedContainer("missing RIFF/WAVE header"));
        }
        // Streaming writers may leave the size as 0 or 0xFFFFFFFF.
        let declared = le_u32(&header, 4)? as u64 + 8;
        let riff_end = if declared <= 8 || declared > file_len { file_len } else { declared };

        let mut chunks: Vec<([u8; 4], u64, u64)> = Vec::new();
        let mut pos = 12u64;
        while pos + 8 <= riff_end {
            let mut h = [0u8; 8];
            input.seek(SeekFrom::Start(pos))?;
            read_up_to(input, &mut h)?;
            let kind: [u8; 4] = h[0..4].try_into().unwrap();
            let data_start = pos + 8;
            let len = (le_u32(&h, 4)? as u64).min(riff_end - data_start);
            if KEPT.contains(&&kind) {
                chunks.push((kind, data_start, len));
            }
            pos = data_start + len + (len & 1);
        }
        let has = |k: &[u8; 4]| chunks.iter().any(|(c, _, _)| c == k);
        if !has(b"fmt ") || !has(b"data") {
            return Err(MediaLibError::MalformedContainer("WAV lacks fmt or data chunk"));
        }

        let size: u64 = 4 + chunks.iter().map(|(_, _, len)| 8 + len + (len & 1)).sum::<u64>();
        let size = u32::try_from(size).map_err(|_| MediaLibError::ContainerTooLarge)?;

        out.write_all(b"RIFF")?;
        out.write_all(&size.to_le_bytes())?;
        out.write_all(b"WAVE")?;
        for (kind, start, len) in chunks {
            out.write_all(&kind)?;
            out.write_all(&(len as u32).to_le_bytes())?;
            copy_range(input, out, start, len)?;
            if len & 1 == 1 {
                out.write_all(&[0])?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::strip::testutil::{self, contains};
    use super::Wav;
    use crate::strip::strip_to_vec;

    #[test]
    fn keeps_only_audio_chunks() {
        let out = strip_to_vec(&Wav, testutil::wav_with_metadata()).unwrap();
        assert!(!contains(&out, b"LIST"));
        assert!(!contains(&out, b"secret"));
        assert!(!contains(&out, b"bext"));
        assert!(contains(&out, b"fmt "));
        assert!(contains(&out, testutil::WAV_SAMPLES));
        assert_eq!(u32::from_le_bytes(out[4..8].try_into().unwrap()) as usize, out.len() - 8);
    }
}
