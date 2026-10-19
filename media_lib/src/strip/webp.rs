//! WebP: drop EXIF and XMP chunks (keeping orientation) and fix VP8X flags.

use std::io::Write;

use super::boxio::{bytes, le_u32};
use super::{IMAGE_CAP, MediaFormat, MetadataStripper, ReadSeek, exif, read_whole};
use crate::media::MediaCategory;
use crate::error::{MediaLibError, MediaResult};

pub(crate) struct WebP;

impl MediaFormat for WebP {
    fn name(&self) -> &'static str {
        "WebP"
    }

    fn categories(&self) -> &'static [MediaCategory] {
        &[MediaCategory::Image]
    }

    fn sniff(&self, head: &[u8]) -> bool {
        head.len() >= 12 && &head[0..4] == b"RIFF" && &head[8..12] == b"WEBP"
    }
}

impl MetadataStripper for WebP {
    fn strip(&self, input: &mut dyn ReadSeek, out: &mut dyn Write) -> MediaResult<()> {
        let data = read_whole(input, IMAGE_CAP)?;
        out.write_all(&strip_webp(&data)?)?;
        Ok(())
    }
}

const FLAG_EXIF: u8 = 0x08;
const FLAG_XMP: u8 = 0x04;
const KEPT: [&[u8; 4]; 7] = [b"VP8 ", b"VP8L", b"VP8X", b"ALPH", b"ANIM", b"ANMF", b"ICCP"];

fn write_chunk(out: &mut Vec<u8>, kind: &[u8], data: &[u8]) {
    out.extend_from_slice(kind);
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(data);
    if data.len() % 2 == 1 {
        out.push(0);
    }
}

pub(crate) fn strip_webp(data: &[u8]) -> MediaResult<Vec<u8>> {
    if data.len() < 12 || &data[0..4] != b"RIFF" || &data[8..12] != b"WEBP" {
        return Err(MediaLibError::MalformedContainer("missing WebP header"));
    }
    let riff_end = (8 + le_u32(data, 4)? as usize).min(data.len());

    let mut chunks: Vec<([u8; 4], Vec<u8>)> = Vec::new();
    let mut orientation = None;
    let mut pos = 12;
    while pos + 8 <= riff_end {
        let kind: [u8; 4] = data[pos..pos + 4].try_into().unwrap();
        let len = le_u32(data, pos + 4)? as usize;
        let body = bytes(data, pos + 8, len)?;
        pos += 8 + len + (len % 2);

        if &kind == b"EXIF" {
            let tiff = body.strip_prefix(b"Exif\0\0").unwrap_or(body);
            orientation = orientation.or(exif::orientation_to_keep(tiff));
        } else if KEPT.contains(&&kind) {
            chunks.push((kind, body.to_vec()));
        }
    }

    let has_vp8x = chunks.first().is_some_and(|(k, _)| k == b"VP8X");
    if let Some((_, vp8x)) = chunks.first_mut().filter(|_| has_vp8x) {
        let flags = vp8x.first_mut().ok_or(MediaLibError::MalformedContainer("empty VP8X"))?;
        *flags &= !(FLAG_EXIF | FLAG_XMP);
        if orientation.is_some() {
            *flags |= FLAG_EXIF;
        }
    }

    let mut body = Vec::with_capacity(data.len());
    body.extend_from_slice(b"WEBP");
    for (kind, chunk) in &chunks {
        write_chunk(&mut body, kind, chunk);
    }
    // EXIF requires the extended (VP8X) format; simple files can't carry it.
    if let (Some(o), true) = (orientation, has_vp8x) {
        write_chunk(&mut body, b"EXIF", &exif::minimal_tiff(o));
    }

    let mut out = Vec::with_capacity(body.len() + 8);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strip::testutil::{self, contains};

    #[test]
    fn drops_exif_and_xmp_and_keeps_orientation() {
        let out = strip_webp(&testutil::webp_with_metadata(3)).unwrap();
        assert!(!contains(&out, b"GPSSECRET"));
        assert!(!contains(&out, b"xmpmeta"));
        assert!(!contains(&out, b"XMP "));
        assert!(contains(&out, b"VP8L"));
        assert_eq!(le_u32(&out, 4).unwrap() as usize, out.len() - 8);

        let flags = out[testutil::find(&out, b"VP8X").unwrap() + 8];
        assert_eq!(flags & FLAG_XMP, 0);
        assert_eq!(flags & FLAG_EXIF, FLAG_EXIF);
        let exif_at = testutil::find(&out, b"EXIF").unwrap();
        assert_eq!(exif::orientation(&out[exif_at + 8..]), Some(3));
    }

    #[test]
    fn clears_exif_flag_when_orientation_is_default() {
        let out = strip_webp(&testutil::webp_with_metadata(1)).unwrap();
        let flags = out[testutil::find(&out, b"VP8X").unwrap() + 8];
        assert_eq!(flags & (FLAG_EXIF | FLAG_XMP), 0);
        assert!(!contains(&out, b"EXIF"));
    }
}
