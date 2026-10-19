//! GIF: drop comment and non-animation application extensions (e.g. XMP).

use std::io::Write;

use super::boxio::bytes;
use super::{IMAGE_CAP, MediaFormat, MetadataStripper, ReadSeek, read_whole};
use crate::media::MediaCategory;
use crate::error::{MediaLibError, MediaResult};

pub(crate) struct Gif;

impl MediaFormat for Gif {
    fn name(&self) -> &'static str {
        "GIF"
    }

    fn categories(&self) -> &'static [MediaCategory] {
        &[MediaCategory::Image]
    }

    fn sniff(&self, head: &[u8]) -> bool {
        head.starts_with(b"GIF87a") || head.starts_with(b"GIF89a")
    }
}

impl MetadataStripper for Gif {
    fn strip(&self, input: &mut dyn ReadSeek, out: &mut dyn Write) -> MediaResult<()> {
        let data = read_whole(input, IMAGE_CAP)?;
        out.write_all(&strip_gif(&data)?)?;
        Ok(())
    }
}

fn malformed(what: &'static str) -> MediaLibError {
    MediaLibError::MalformedContainer(what)
}

/// End offset of a chain of data sub-blocks starting at `pos`.
fn sub_blocks_end(data: &[u8], mut pos: usize) -> MediaResult<usize> {
    loop {
        let len = *data.get(pos).ok_or(malformed("truncated GIF sub-block"))? as usize;
        pos += 1;
        if len == 0 {
            return Ok(pos);
        }
        bytes(data, pos, len)?;
        pos += len;
    }
}

fn color_table_len(packed: u8) -> usize {
    if packed & 0x80 != 0 { 3 << ((packed & 0x07) + 1) } else { 0 }
}

pub(crate) fn strip_gif(data: &[u8]) -> MediaResult<Vec<u8>> {
    if !(data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a")) {
        return Err(malformed("missing GIF header"));
    }
    let screen = bytes(data, 6, 7)?;
    let header_end = 13 + color_table_len(screen[4]);
    bytes(data, 0, header_end)?;

    let mut out = Vec::with_capacity(data.len());
    out.extend_from_slice(&data[..header_end]);
    let mut pos = header_end;

    loop {
        match *data.get(pos).ok_or(malformed("missing GIF trailer"))? {
            0x3B => {
                out.push(0x3B);
                return Ok(out);
            }
            0x2C => {
                let descriptor = bytes(data, pos, 10)?;
                let table_end = pos + 10 + color_table_len(descriptor[9]);
                let end = sub_blocks_end(data, table_end + 1)?; // +1: LZW code size
                out.extend_from_slice(&data[pos..end]);
                pos = end;
            }
            0x21 => {
                let label = *data.get(pos + 1).ok_or(malformed("truncated GIF extension"))?;
                let end = sub_blocks_end(data, pos + 2)?;
                let keep = match label {
                    0xFE => false, // comment
                    0xFF => {
                        let id = data.get(pos + 3..pos + 14).unwrap_or_default();
                        id == b"NETSCAPE2.0" || id == b"ANIMEXTS1.0"
                    }
                    _ => true, // graphic control, plain text
                };
                if keep {
                    out.extend_from_slice(&data[pos..end]);
                }
                pos = end;
            }
            _ => return Err(malformed("unknown GIF block")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strip::testutil::{self, contains};

    #[test]
    fn drops_comments_and_xmp_keeps_animation() {
        let out = strip_gif(&testutil::gif_with_metadata()).unwrap();
        assert!(!contains(&out, b"a comment"));
        assert!(!contains(&out, b"XMP DataXMP"));
        assert!(contains(&out, b"NETSCAPE2.0"));
        assert!(contains(&out, &[0x21, 0xF9]));
        assert_eq!(out.last(), Some(&0x3B));
        assert!(!contains(&out, b"TRAILER"));
    }
}
