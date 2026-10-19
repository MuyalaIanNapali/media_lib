//! MPEG audio (MP3) and AAC ADTS: cut away ID3v2, ID3v1, APEv2 and Lyrics3
//! tags and stream the audio frames between them unchanged.

use std::io::Write;

use super::boxio::{copy_range, le_u32, read_at, read_up_to, stream_len};
use super::{MediaFormat, MetadataStripper, ReadSeek};
use crate::media::MediaCategory;
use crate::error::{MediaLibError, MediaResult};
use std::io::SeekFrom;

pub(crate) struct MpegAudio;

impl MediaFormat for MpegAudio {
    fn name(&self) -> &'static str {
        "MPEG audio (MP3/AAC)"
    }

    fn categories(&self) -> &'static [MediaCategory] {
        &[MediaCategory::Audio]
    }

    fn sniff(&self, head: &[u8]) -> bool {
        is_frame_sync(head)
    }

    fn may_follow_id3(&self) -> bool {
        true
    }
}

impl MetadataStripper for MpegAudio {
    fn strip(&self, input: &mut dyn ReadSeek, out: &mut dyn Write) -> MediaResult<()> {
        let start = skip_id3v2(input)?;
        let end = audio_end(input, start)?;

        let mut sync = [0u8; 2];
        input.seek(SeekFrom::Start(start))?;
        if end - start < 2 || read_up_to(input, &mut sync)? < 2 || !is_frame_sync(&sync) {
            return Err(MediaLibError::MalformedContainer("no audio frame after tags"));
        }
        copy_range(input, out, start, end - start)
    }
}

/// An MPEG audio / ADTS frame header starts with 11 set bits.
pub(crate) fn is_frame_sync(head: &[u8]) -> bool {
    head.len() >= 2 && head[0] == 0xFF && head[1] & 0xE0 == 0xE0
}

fn syncsafe(b: &[u8]) -> u64 {
    b.iter().fold(0u64, |acc, &x| (acc << 7) | (x & 0x7F) as u64)
}

/// Offset just past any leading ID3v2 tags (there may be several).
pub(crate) fn skip_id3v2(input: &mut dyn ReadSeek) -> MediaResult<u64> {
    let len = stream_len(input)?;
    let mut pos = 0u64;
    loop {
        let mut header = [0u8; 10];
        input.seek(SeekFrom::Start(pos))?;
        if read_up_to(input, &mut header)? < 10 || &header[0..3] != b"ID3" || header[3] == 0xFF {
            break;
        }
        if header[6..10].iter().any(|&b| b & 0x80 != 0) {
            return Err(MediaLibError::MalformedContainer("invalid ID3v2 size"));
        }
        let footer = if header[5] & 0x10 != 0 { 10 } else { 0 };
        pos += 10 + syncsafe(&header[6..10]) + footer;
        if pos > len {
            return Err(MediaLibError::MalformedContainer("ID3v2 tag extends past end of file"));
        }
    }
    input.seek(SeekFrom::Start(0))?;
    Ok(pos)
}

/// End of the audio data: the file length minus any trailing tags.
pub(crate) fn audio_end(input: &mut dyn ReadSeek, start: u64) -> MediaResult<u64> {
    let mut end = stream_len(input)?;
    loop {
        let before = end;
        let room = end - start;

        // ID3v2 appended at the end (v2.4 footer "3DI").
        if room >= 10 {
            let mut f = [0u8; 10];
            read_at(input, end - 10, &mut f)?;
            if &f[0..3] == b"3DI" && f[6..10].iter().all(|&b| b & 0x80 == 0) {
                let total = syncsafe(&f[6..10]) + 20;
                if total <= room {
                    end -= total;
                }
            }
        }
        // APEv2 (footer, plus header when flagged).
        if end - start >= 32 {
            let mut f = [0u8; 32];
            read_at(input, end - 32, &mut f)?;
            if &f[0..8] == b"APETAGEX" {
                let size = le_u32(&f, 12)? as u64;
                let has_header = le_u32(&f, 20)? & 0x8000_0000 != 0;
                let total = size + if has_header { 32 } else { 0 };
                if total <= end - start {
                    end -= total;
                }
            }
        }
        // ID3v1, optionally preceded by the extended "TAG+" block.
        if end - start >= 128 {
            let mut t = [0u8; 3];
            read_at(input, end - 128, &mut t)?;
            if &t == b"TAG" {
                end -= 128;
                if end - start >= 227 {
                    let mut t = [0u8; 4];
                    read_at(input, end - 227, &mut t)?;
                    if &t == b"TAG+" {
                        end -= 227;
                    }
                }
            }
        }
        // Lyrics3v2: "...LYRICSBEGIN<fields><6-digit size>LYRICS200".
        if end - start >= 15 {
            let mut f = [0u8; 15];
            read_at(input, end - 15, &mut f)?;
            if &f[6..15] == b"LYRICS200" {
                let size = std::str::from_utf8(&f[0..6]).ok().and_then(|s| s.parse::<u64>().ok());
                if let Some(total) = size.map(|s| s + 15).filter(|&t| t <= end - start) {
                    end -= total;
                }
            }
        }

        if end == before {
            return Ok(end);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strip::testutil;
    use crate::strip::strip_to_vec;

    #[test]
    fn strips_leading_and_trailing_tags() {
        let frames = testutil::mp3_frames();
        let input = testutil::mp3_with_tags(&frames);
        let out = strip_to_vec(&MpegAudio, input).unwrap();
        assert_eq!(out, frames);
    }

    #[test]
    fn untagged_file_is_unchanged() {
        let frames = testutil::mp3_frames();
        assert_eq!(strip_to_vec(&MpegAudio, frames.clone()).unwrap(), frames);
    }

    #[test]
    fn rejects_tags_without_audio() {
        let input = testutil::id3v2(b"TIT2secret");
        assert!(matches!(
            strip_to_vec(&MpegAudio, input),
            Err(MediaLibError::MalformedContainer(_))
        ));
    }
}
