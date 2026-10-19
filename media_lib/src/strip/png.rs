//! PNG: drop text, time and unknown ancillary chunks; keep colour and APNG.

use std::io::Write;

use super::boxio::{be_u32, bytes};
use super::crc32::crc32;
use super::{IMAGE_CAP, MediaFormat, MetadataStripper, ReadSeek, exif, read_whole};
use crate::media::MediaCategory;
use crate::error::{MediaLibError, MediaResult};

pub(crate) struct Png;

impl MediaFormat for Png {
    fn name(&self) -> &'static str {
        "PNG"
    }

    fn categories(&self) -> &'static [MediaCategory] {
        &[MediaCategory::Image]
    }

    fn sniff(&self, head: &[u8]) -> bool {
        head.starts_with(SIGNATURE)
    }
}

impl MetadataStripper for Png {
    fn strip(&self, input: &mut dyn ReadSeek, out: &mut dyn Write) -> MediaResult<()> {
        let data = read_whole(input, IMAGE_CAP)?;
        out.write_all(&strip_png(&data)?)?;
        Ok(())
    }
}

const SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

/// Ancillary chunks that affect how the image looks (or animates).
const KEPT_ANCILLARY: [&[u8; 4]; 11] = [
    b"gAMA", b"cHRM", b"sRGB", b"iCCP", b"sBIT", b"pHYs", b"bKGD", b"tRNS", b"acTL", b"fcTL", b"fdAT",
];

fn write_chunk(out: &mut Vec<u8>, kind: &[u8], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    out.extend_from_slice(&crc32(&[kind, data]).to_be_bytes());
}

pub(crate) fn strip_png(data: &[u8]) -> MediaResult<Vec<u8>> {
    if !data.starts_with(SIGNATURE) {
        return Err(MediaLibError::MalformedContainer("missing PNG signature"));
    }

    // Orientation lives in eXIf, which must precede the first IDAT.
    let mut chunks = Vec::new();
    let mut pos = SIGNATURE.len();
    let mut orientation = None;
    loop {
        let len = be_u32(data, pos)? as usize;
        let kind: [u8; 4] = bytes(data, pos + 4, 4)?.try_into().unwrap();
        let body = bytes(data, pos + 8, len)?;
        bytes(data, pos + 8 + len, 4)?; // CRC present
        pos += 12 + len;

        if &kind == b"eXIf" {
            orientation = orientation.or(exif::orientation_to_keep(body));
        }
        let critical = kind[0].is_ascii_uppercase();
        if critical || KEPT_ANCILLARY.contains(&&kind) {
            chunks.push((kind, body));
        }
        if &kind == b"IEND" {
            break; // trailing bytes are dropped
        }
    }

    let mut out = Vec::with_capacity(data.len());
    out.extend_from_slice(SIGNATURE);
    let mut exif_written = orientation.is_none();
    for (kind, body) in chunks {
        if !exif_written && (&kind == b"IDAT" || &kind == b"fdAT" || &kind == b"IEND") {
            write_chunk(&mut out, b"eXIf", &exif::minimal_tiff(orientation.unwrap()));
            exif_written = true;
        }
        write_chunk(&mut out, &kind, body);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strip::testutil::{self, contains};

    #[test]
    fn drops_text_and_keeps_image_chunks() {
        let out = strip_png(&testutil::png_with_metadata(8)).unwrap();
        for secret in [&b"GPSSECRET"[..], b"Author", b"tIME", b"zzZz", b"TRAILER"] {
            assert!(!contains(&out, secret), "{}", String::from_utf8_lossy(secret));
        }
        for kept in [&b"IHDR"[..], b"sRGB", b"IDAT", b"IEND"] {
            assert!(contains(&out, kept));
        }
        let exif_at = testutil::find(&out, b"eXIf").unwrap();
        assert!(exif_at < testutil::find(&out, b"IDAT").unwrap());
        assert_eq!(exif::orientation(&out[exif_at + 4..]), Some(8));
    }

    #[test]
    fn crcs_are_valid() {
        let out = strip_png(&testutil::png_with_metadata(8)).unwrap();
        let mut pos = 8;
        while pos < out.len() {
            let len = be_u32(&out, pos).unwrap() as usize;
            let crc = be_u32(&out, pos + 8 + len).unwrap();
            assert_eq!(crc, crc32(&[&out[pos + 4..pos + 8 + len]]));
            pos += 12 + len;
        }
    }
}
