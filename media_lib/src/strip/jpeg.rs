//! JPEG: keep only the segments needed to decode and colour-manage the image.

use std::io::Write;

use super::boxio::{be_u16, bytes};
use super::{IMAGE_CAP, MediaFormat, MetadataStripper, ReadSeek, exif, read_whole};
use crate::media::MediaCategory;
use crate::error::{MediaLibError, MediaResult};

pub(crate) struct Jpeg;

impl MediaFormat for Jpeg {
    fn name(&self) -> &'static str {
        "JPEG"
    }

    fn categories(&self) -> &'static [MediaCategory] {
        &[MediaCategory::Image]
    }

    fn sniff(&self, head: &[u8]) -> bool {
        head.starts_with(&[0xFF, 0xD8, 0xFF])
    }
}

impl MetadataStripper for Jpeg {
    fn strip(&self, input: &mut dyn ReadSeek, out: &mut dyn Write) -> MediaResult<()> {
        let data = read_whole(input, IMAGE_CAP)?;
        out.write_all(&strip_jpeg(&data)?)?;
        Ok(())
    }
}

const SOI: u8 = 0xD8;
const EOI: u8 = 0xD9;
const SOS: u8 = 0xDA;
const APP0: u8 = 0xE0;
const APP1: u8 = 0xE1;
const APP2: u8 = 0xE2;
const APP14: u8 = 0xEE;
const COM: u8 = 0xFE;

fn malformed(what: &'static str) -> MediaLibError {
    MediaLibError::MalformedContainer(what)
}

/// Whether a marker segment is kept. Everything that only carries metadata
/// (EXIF, XMP, IPTC/Photoshop, MPF, comments, JFXX thumbnails) is dropped.
fn keep_segment(marker: u8, payload: &[u8]) -> bool {
    match marker {
        APP0 => payload.starts_with(b"JFIF\0"),
        APP2 => payload.starts_with(b"ICC_PROFILE\0"),
        APP14 => payload.starts_with(b"Adobe"),
        0xE1..=0xEF | COM => false,
        _ => true,
    }
}

pub(crate) fn strip_jpeg(data: &[u8]) -> MediaResult<Vec<u8>> {
    if !data.starts_with(&[0xFF, SOI]) {
        return Err(malformed("missing JPEG SOI"));
    }

    let mut body = Vec::with_capacity(data.len());
    let mut orientation = None;
    // Where to insert the minimal EXIF segment: after a leading JFIF APP0.
    let mut exif_insert_at = 0usize;
    let mut pos = 2usize;

    loop {
        if data.get(pos) != Some(&0xFF) {
            return Err(malformed("expected JPEG marker"));
        }
        while data.get(pos) == Some(&0xFF) {
            pos += 1; // fill bytes
        }
        let marker = *data.get(pos).ok_or(malformed("missing JPEG EOI"))?;
        pos += 1;

        match marker {
            EOI => {
                body.extend_from_slice(&[0xFF, EOI]);
                // Anything after EOI (motion-photo video, vendor trailers) is dropped.
                break;
            }
            0x01 | 0xD0..=0xD7 => body.extend_from_slice(&[0xFF, marker]),
            SOI => return Err(malformed("unexpected SOI")),
            _ => {
                let len = be_u16(data, pos)? as usize;
                if len < 2 {
                    return Err(malformed("invalid JPEG segment length"));
                }
                let segment = bytes(data, pos, len)?;
                let payload = &segment[2..];

                if marker == APP1 && payload.starts_with(b"Exif\0\0") {
                    orientation = orientation.or(exif::orientation_to_keep(&payload[6..]));
                }
                if keep_segment(marker, payload) {
                    body.extend_from_slice(&[0xFF, marker]);
                    body.extend_from_slice(segment);
                    if marker == APP0 && exif_insert_at == 0 && body.len() == segment.len() + 2 {
                        exif_insert_at = body.len();
                    }
                }
                pos += len;

                if marker == SOS {
                    let end = scan_end(data, pos)?;
                    body.extend_from_slice(&data[pos..end]);
                    pos = end;
                }
            }
        }
    }

    let mut out = Vec::with_capacity(body.len() + 64);
    out.extend_from_slice(&[0xFF, SOI]);
    out.extend_from_slice(&body[..exif_insert_at]);
    if let Some(o) = orientation {
        let tiff = exif::minimal_tiff(o);
        out.extend_from_slice(&[0xFF, APP1]);
        out.extend_from_slice(&((2 + 6 + tiff.len()) as u16).to_be_bytes());
        out.extend_from_slice(b"Exif\0\0");
        out.extend_from_slice(&tiff);
    }
    out.extend_from_slice(&body[exif_insert_at..]);
    Ok(out)
}

/// End of the entropy-coded data that follows an SOS header: the next marker
/// that isn't a stuffed byte (FF00), a restart marker (FFD0-D7) or fill.
fn scan_end(data: &[u8], mut pos: usize) -> MediaResult<usize> {
    loop {
        let ff = data[pos..]
            .iter()
            .position(|&b| b == 0xFF)
            .map(|i| pos + i)
            .ok_or(malformed("unterminated JPEG scan"))?;
        match data.get(ff + 1) {
            None => return Err(malformed("unterminated JPEG scan")),
            Some(0x00) | Some(0xD0..=0xD7) => pos = ff + 2,
            Some(0xFF) => pos = ff + 1,
            Some(_) => return Ok(ff),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strip::testutil::{self, contains};

    #[test]
    fn removes_metadata_and_keeps_orientation() {
        let input = testutil::jpeg_with_metadata(6);
        let out = strip_jpeg(&input).unwrap();

        for secret in [&b"GPSSECRET"[..], b"PixelSecret", b"xmpmeta", b"a comment", b"TRAILER", b"Photoshop"] {
            assert!(!contains(&out, secret), "{}", String::from_utf8_lossy(secret));
        }
        // Decoding-relevant segments and the scan survive byte-for-byte.
        assert!(contains(&out, testutil::JPEG_SCAN));
        assert!(contains(&out, b"JFIF\0"));
        assert!(contains(&out, b"ICC_PROFILE\0"));
        assert!(out.ends_with(&[0xFF, EOI]));

        // JFIF first, then the minimal EXIF carrying orientation 6.
        assert_eq!(&out[2..4], &[0xFF, APP0]);
        let exif_at = testutil::find(&out, b"Exif\0\0").unwrap();
        assert_eq!(exif::orientation(&out[exif_at + 6..]), Some(6));
    }

    #[test]
    fn default_orientation_needs_no_exif() {
        let out = strip_jpeg(&testutil::jpeg_with_metadata(1)).unwrap();
        assert!(!contains(&out, b"Exif"));
    }

    #[test]
    fn stuffed_bytes_and_restart_markers_stay_in_scan() {
        let out = strip_jpeg(&testutil::jpeg_with_metadata(1)).unwrap();
        assert!(contains(&out, &[0xFF, 0x00, 0x12, 0xFF, 0xD3, 0x34]));
    }

    #[test]
    fn rejects_truncated_input() {
        let mut input = testutil::jpeg_with_metadata(1);
        let eoi = input.len() - testutil::JPEG_TRAILER.len() - 2;
        input.truncate(eoi - 3);
        assert!(matches!(strip_jpeg(&input), Err(MediaLibError::MalformedContainer(_))));
    }
}
