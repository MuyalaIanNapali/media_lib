//! Just enough EXIF (TIFF) handling to carry the orientation tag across.

const ORIENTATION_TAG: u16 = 0x0112;
const TYPE_SHORT: u16 = 3;

/// The orientation (1..=8) in a TIFF blob's IFD0, if present.
pub fn orientation(tiff: &[u8]) -> Option<u16> {
    let big_endian = match tiff.get(0..2)? {
        b"MM" => true,
        b"II" => false,
        _ => return None,
    };
    let u16_at = |o: usize| -> Option<u16> {
        let b: [u8; 2] = tiff.get(o..o + 2)?.try_into().ok()?;
        Some(if big_endian { u16::from_be_bytes(b) } else { u16::from_le_bytes(b) })
    };
    let u32_at = |o: usize| -> Option<u32> {
        let b: [u8; 4] = tiff.get(o..o + 4)?.try_into().ok()?;
        Some(if big_endian { u32::from_be_bytes(b) } else { u32::from_le_bytes(b) })
    };

    if u16_at(2)? != 42 {
        return None;
    }
    let ifd = u32_at(4)? as usize;
    let count = u16_at(ifd)? as usize;
    (0..count).find_map(|i| {
        let entry = ifd + 2 + i * 12;
        (u16_at(entry)? == ORIENTATION_TAG && u16_at(entry + 2)? == TYPE_SHORT)
            .then(|| u16_at(entry + 8))
            .flatten()
    })
    .filter(|o| (1..=8).contains(o))
}

/// Orientation worth preserving: anything other than the default (1).
pub fn orientation_to_keep(tiff: &[u8]) -> Option<u16> {
    orientation(tiff).filter(|&o| o != 1)
}

/// A big-endian TIFF whose only content is IFD0 with the orientation tag:
/// no GPS, camera, timestamps or thumbnail.
pub fn minimal_tiff(orientation: u16) -> Vec<u8> {
    let mut t = Vec::with_capacity(26);
    t.extend_from_slice(b"MM");
    t.extend_from_slice(&42u16.to_be_bytes());
    t.extend_from_slice(&8u32.to_be_bytes()); // IFD0 offset
    t.extend_from_slice(&1u16.to_be_bytes()); // one entry
    t.extend_from_slice(&ORIENTATION_TAG.to_be_bytes());
    t.extend_from_slice(&TYPE_SHORT.to_be_bytes());
    t.extend_from_slice(&1u32.to_be_bytes()); // count
    t.extend_from_slice(&orientation.to_be_bytes());
    t.extend_from_slice(&[0, 0]); // value padding
    t.extend_from_slice(&0u32.to_be_bytes()); // no next IFD
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strip::testutil;

    #[test]
    fn minimal_tiff_round_trips() {
        for o in 1..=8 {
            assert_eq!(orientation(&minimal_tiff(o)), Some(o));
        }
        assert_eq!(minimal_tiff(6).len(), 26);
    }

    #[test]
    fn reads_little_endian_with_other_tags() {
        let tiff = testutil::exif_tiff_le(6);
        assert_eq!(orientation(&tiff), Some(6));
        assert_eq!(orientation_to_keep(&testutil::exif_tiff_le(1)), None);
    }

    #[test]
    fn ignores_garbage() {
        assert_eq!(orientation(b"nope"), None);
        assert_eq!(orientation(b"MM\0\x2a\xff\xff\xff\xff"), None);
    }
}
