//! FLAC: keep STREAMINFO and SEEKTABLE; drop comments, pictures, cue sheets,
//! application blocks and padding, plus any ID3 tags around the stream.

use std::io::Write;

use super::boxio::{copy_range, read_at};
use super::mpeg_audio::{audio_end, skip_id3v2};
use super::{MediaFormat, MetadataStripper, ReadSeek};
use crate::media::MediaCategory;
use crate::error::{MediaLibError, MediaResult};

pub(crate) struct Flac;

impl MediaFormat for Flac {
    fn name(&self) -> &'static str {
        "FLAC"
    }

    fn categories(&self) -> &'static [MediaCategory] {
        &[MediaCategory::Audio]
    }

    fn sniff(&self, head: &[u8]) -> bool {
        head.starts_with(b"fLaC")
    }

    fn may_follow_id3(&self) -> bool {
        true
    }
}

const STREAMINFO: u8 = 0;
const SEEKTABLE: u8 = 3;

impl MetadataStripper for Flac {
    fn strip(&self, input: &mut dyn ReadSeek, out: &mut dyn Write) -> MediaResult<()> {
        let start = skip_id3v2(input)?;
        let mut magic = [0u8; 4];
        read_at(input, start, &mut magic)?;
        if &magic != b"fLaC" {
            return Err(MediaLibError::MalformedContainer("missing fLaC marker"));
        }

        let mut kept: Vec<(u8, Vec<u8>)> = Vec::new();
        let mut pos = start + 4;
        loop {
            let mut header = [0u8; 4];
            read_at(input, pos, &mut header)?;
            let last = header[0] & 0x80 != 0;
            let kind = header[0] & 0x7F;
            let len = u32::from_be_bytes([0, header[1], header[2], header[3]]) as usize;
            if kind == 127 {
                return Err(MediaLibError::MalformedContainer("invalid FLAC metadata block"));
            }
            if kind == STREAMINFO || kind == SEEKTABLE {
                let mut body = vec![0u8; len];
                read_at(input, pos + 4, &mut body)?;
                kept.push((kind, body));
            }
            pos += 4 + len as u64;
            if last {
                break;
            }
        }
        if kept.first().map(|(k, _)| *k) != Some(STREAMINFO) {
            return Err(MediaLibError::MalformedContainer("FLAC stream lacks STREAMINFO"));
        }

        let frames_end = audio_end(input, pos)?;

        out.write_all(b"fLaC")?;
        let count = kept.len();
        for (i, (kind, body)) in kept.iter().enumerate() {
            let last_flag = if i + 1 == count { 0x80 } else { 0 };
            let len = (body.len() as u32).to_be_bytes();
            out.write_all(&[kind | last_flag, len[1], len[2], len[3]])?;
            out.write_all(body)?;
        }
        copy_range(input, out, pos, frames_end - pos)
    }
}

#[cfg(test)]
mod tests {
    use crate::strip::testutil::{self, contains};
    use super::Flac;
    use crate::strip::strip_to_vec;

    #[test]
    fn keeps_streaminfo_and_seektable_only() {
        let out = strip_to_vec(&Flac, testutil::flac_with_metadata()).unwrap();
        assert!(out.starts_with(b"fLaC"));
        assert!(!contains(&out, b"ARTIST=secret"));
        assert!(!contains(&out, b"PICTUREDATA"));
        assert!(!contains(&out, b"ID3"));
        assert!(!contains(&out, b"TAG"));
        assert!(out.ends_with(testutil::FLAC_FRAMES));

        // STREAMINFO (not last), then SEEKTABLE flagged last.
        assert_eq!(out[4], 0x00);
        let seektable = 4 + 4 + 34;
        assert_eq!(out[seektable], 0x80 | 3);
    }
}
