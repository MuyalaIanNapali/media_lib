//! ISO base media files (MP4, MOV, 3GP, M4A).
//!
//! Metadata lives in boxes (`udta`, `meta`, `uuid`/XMP), in header timestamps
//! (`mvhd`/`tkhd`/`mdhd`) and sometimes in whole timed-metadata tracks (GPS,
//! camera motion). The media samples live in `mdat` and are addressed by
//! absolute file offsets (`stco`/`co64`, `tfhd`), so removing bytes means
//! rewriting those offsets.
//!
//! Pass 1 indexes the top-level boxes and rewrites `moov` in memory (its size
//! doesn't depend on the offsets it holds, since entries are fixed-width).
//! The new layout then gives each kept box a shift, offsets are patched, and
//! pass 2 streams everything out. `mdat` is never held in memory, and
//! fragment headers (`moof`) are loaded, patched and written one at a time.

use std::collections::HashMap;
use std::io::{self, Write};

use super::boxio::{be_u32, be_u64, copy_range, read_at, read_up_to, stream_len};
use super::{MediaFormat, MetadataStripper, ReadSeek};
use crate::media::MediaCategory;
use crate::error::{MediaLibError, MediaResult};
use std::io::{Read, SeekFrom};

pub(crate) struct IsoBmff;

impl MediaFormat for IsoBmff {
    fn name(&self) -> &'static str {
        "ISO-BMFF (MP4/MOV/M4A)"
    }

    fn categories(&self) -> &'static [MediaCategory] {
        &[MediaCategory::Video, MediaCategory::Audio]
    }

    fn sniff(&self, head: &[u8]) -> bool {
        head.len() >= 8 && is_top_level_kind(&head[4..8])
    }
}

/// Upper bound for boxes processed in memory (`moov`, one `moof` at a time).
const IN_MEMORY_CAP: u64 = 64 * 1024 * 1024;

type Kind = [u8; 4];

/// Top-level boxes kept. Everything else — `meta`, `udta`, `uuid` (XMP),
/// `free`/`skip`/`wide` padding (which can hold stale data) and `mfra`
/// (absolute offsets, optional) — is dropped.
const KEPT_TOP: [&Kind; 9] = [b"ftyp", b"styp", b"pdin", b"moov", b"mdat", b"moof", b"sidx", b"ssix", b"prft"];

/// Boxes recursed into while rewriting.
const CONTAINERS: [&Kind; 10] =
    [b"moov", b"trak", b"mdia", b"minf", b"stbl", b"edts", b"dinf", b"mvex", b"moof", b"traf"];

/// Boxes removed wherever they appear inside `moov`.
const DROPPED: [&Kind; 5] = [b"udta", b"meta", b"uuid", b"free", b"skip"];

/// Handler types of tracks that carry metadata rather than media.
const METADATA_HANDLERS: [&Kind; 4] = [b"meta", b"gpmd", b"camm", b"text"];

/// Box kinds that may open an ISO-BMFF / QuickTime file.
pub(crate) fn is_top_level_kind(kind: &[u8]) -> bool {
    [&b"ftyp"[..], b"styp", b"moov", b"mdat", b"wide", b"free", b"skip", b"pnot"].contains(&kind)
}

fn malformed(what: &'static str) -> MediaLibError {
    MediaLibError::MalformedContainer(what)
}

// ---------------------------------------------------------------------------
// In-memory box parsing
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub(crate) struct BoxPos {
    pub kind: Kind,
    pub start: usize,
    pub header: usize,
    pub end: usize,
}

impl BoxPos {
    pub fn payload<'a>(&self, data: &'a [u8]) -> &'a [u8] {
        &data[self.start + self.header..self.end]
    }
}

pub(crate) fn children(data: &[u8]) -> MediaResult<Vec<BoxPos>> {
    let mut boxes = Vec::new();
    let mut pos = 0;
    while pos < data.len() {
        let remaining = data.len() - pos;
        if remaining < 8 {
            return Err(malformed("truncated box header"));
        }
        let kind: Kind = data[pos + 4..pos + 8].try_into().unwrap();
        let (header, size) = match be_u32(data, pos)? {
            1 => (16, be_u64(data, pos + 8)?),
            0 => (8, remaining as u64),
            s => (8, s as u64),
        };
        if size < header as u64 || size > remaining as u64 {
            return Err(malformed("box size out of range"));
        }
        let end = pos + size as usize;
        boxes.push(BoxPos { kind, start: pos, header, end });
        pos = end;
    }
    Ok(boxes)
}

pub(crate) fn find_child<'a>(data: &'a [u8], kind: &Kind) -> MediaResult<Option<&'a [u8]>> {
    Ok(children(data)?.into_iter().find(|b| &b.kind == kind).map(|b| b.payload(data)))
}

pub(crate) fn find_path<'a>(mut data: &'a [u8], path: &[&Kind]) -> MediaResult<Option<&'a [u8]>> {
    for kind in path {
        match find_child(data, kind)? {
            Some(inner) => data = inner,
            None => return Ok(None),
        }
    }
    Ok(Some(data))
}

fn box_header(kind: &Kind, payload_len: usize) -> Vec<u8> {
    let size = payload_len as u64 + 8;
    let mut h = Vec::with_capacity(16);
    if let Ok(size) = u32::try_from(size) {
        h.extend_from_slice(&size.to_be_bytes());
        h.extend_from_slice(kind);
    } else {
        h.extend_from_slice(&1u32.to_be_bytes());
        h.extend_from_slice(kind);
        h.extend_from_slice(&(size + 8).to_be_bytes());
    }
    h
}

fn write_box(out: &mut Vec<u8>, kind: &Kind, payload: &[u8]) {
    out.extend_from_slice(&box_header(kind, payload.len()));
    out.extend_from_slice(payload);
}

// ---------------------------------------------------------------------------
// moov rewriting
// ---------------------------------------------------------------------------

/// Rewrites a container's children, dropping metadata boxes and tracks and
/// zeroing timestamps. Sample ranges of dropped tracks go into `zero`.
fn rewrite_container(data: &[u8], zero: &mut Vec<(u64, u64)>) -> MediaResult<Vec<u8>> {
    let mut out = Vec::with_capacity(data.len());
    for b in children(data)? {
        let payload = b.payload(data);
        match &b.kind {
            k if DROPPED.contains(&k) => {}
            b"trak" if is_metadata_track(payload)? => zero.extend(sample_ranges(payload)?),
            k if CONTAINERS.contains(&k) => write_box(&mut out, k, &rewrite_container(payload, zero)?),
            b"mvhd" | b"tkhd" | b"mdhd" => {
                let mut p = payload.to_vec();
                zero_times(&mut p)?;
                write_box(&mut out, &b.kind, &p);
            }
            _ => out.extend_from_slice(&data[b.start..b.end]),
        }
    }
    Ok(out)
}

/// Zeroes creation and modification time in a version 0 or 1 header box.
fn zero_times(p: &mut [u8]) -> MediaResult<()> {
    let range = match p.first() {
        Some(1) => 4..20,
        Some(0) => 4..12,
        _ => return Err(malformed("unsupported header box version")),
    };
    p.get_mut(range).ok_or(malformed("truncated header box"))?.fill(0);
    Ok(())
}

fn is_metadata_track(trak: &[u8]) -> MediaResult<bool> {
    Ok(match find_path(trak, &[b"mdia", b"hdlr"])? {
        Some(hdlr) if hdlr.len() >= 12 => METADATA_HANDLERS.iter().any(|h| &hdlr[8..12] == *h),
        _ => false,
    })
}

/// Byte ranges (absolute, in the original file) of a track's samples, from
/// its chunk offsets, sample-to-chunk map and sample sizes.
fn sample_ranges(trak: &[u8]) -> MediaResult<Vec<(u64, u64)>> {
    let Some(stbl) = find_path(trak, &[b"mdia", b"minf", b"stbl"])? else {
        return Ok(Vec::new());
    };

    let table = |p: &[u8], width: usize, header: usize| -> MediaResult<usize> {
        let count = be_u32(p, 4)? as usize;
        if count.saturating_mul(width) > p.len().saturating_sub(header) {
            return Err(malformed("sample table count out of range"));
        }
        Ok(count)
    };

    let offsets: Vec<u64> = if let Some(stco) = find_child(stbl, b"stco")? {
        (0..table(stco, 4, 8)?).map(|i| be_u32(stco, 8 + 4 * i).map(u64::from)).collect::<MediaResult<_>>()?
    } else if let Some(co64) = find_child(stbl, b"co64")? {
        (0..table(co64, 8, 8)?).map(|i| be_u64(co64, 8 + 8 * i)).collect::<MediaResult<_>>()?
    } else {
        return Ok(Vec::new());
    };

    let stsc: Vec<(u32, u32)> = match find_child(stbl, b"stsc")? {
        Some(p) => (0..table(p, 12, 8)?)
            .map(|i| Ok((be_u32(p, 8 + 12 * i)?, be_u32(p, 12 + 12 * i)?)))
            .collect::<MediaResult<_>>()?,
        None => return Ok(Vec::new()),
    };

    let Some(stsz) = find_child(stbl, b"stsz")? else {
        return if find_child(stbl, b"stz2")?.is_some() {
            Err(malformed("compact sample sizes (stz2) in metadata track"))
        } else {
            Ok(Vec::new())
        };
    };
    let fixed_size = be_u32(stsz, 4)?;
    let sample_count = be_u32(stsz, 8)? as usize;
    if fixed_size == 0 && sample_count.saturating_mul(4) > stsz.len().saturating_sub(12) {
        return Err(malformed("sample size table out of range"));
    }
    let size_of = |i: usize| -> MediaResult<u64> {
        Ok(if fixed_size != 0 { fixed_size as u64 } else { be_u32(stsz, 12 + 4 * i)? as u64 })
    };

    let mut ranges = Vec::with_capacity(offsets.len());
    let mut sample = 0usize;
    let mut entry = 0usize;
    for (index, &offset) in offsets.iter().enumerate() {
        let chunk = index as u32 + 1;
        while entry + 1 < stsc.len() && stsc[entry + 1].0 <= chunk {
            entry += 1;
        }
        let per_chunk = stsc.get(entry).map_or(0, |e| e.1) as usize;
        let mut len = 0u64;
        for _ in 0..per_chunk {
            if sample >= sample_count {
                break;
            }
            len += size_of(sample)?;
            sample += 1;
        }
        ranges.push((offset, offset.saturating_add(len)));
    }
    Ok(ranges)
}

// ---------------------------------------------------------------------------
// Offset translation
// ---------------------------------------------------------------------------

/// Maps original file offsets to output offsets for every kept top-level box.
struct OffsetMap(Vec<(u64, u64, u64)>); // (old_start, old_end, new_start)

impl OffsetMap {
    fn translate(&self, offset: u64) -> MediaResult<u64> {
        self.0
            .iter()
            .find(|(start, end, _)| (*start..=*end).contains(&offset))
            .map(|(start, _, new_start)| offset - start + new_start)
            .ok_or(malformed("sample offset points outside the kept data"))
    }
}

/// Rewrites every `stco`/`co64` entry inside a (rewritten) `moov` payload.
fn patch_chunk_offsets(data: &mut [u8], map: &OffsetMap) -> MediaResult<()> {
    for b in children(data)? {
        let p = &mut data[b.start + b.header..b.end];
        match &b.kind {
            k if CONTAINERS.contains(&k) => patch_chunk_offsets(p, map)?,
            b"stco" => {
                let count = be_u32(p, 4)? as usize;
                for i in 0..count {
                    let at = 8 + 4 * i;
                    let moved = map.translate(be_u32(p, at)? as u64)?;
                    let moved = u32::try_from(moved).map_err(|_| MediaLibError::ContainerTooLarge)?;
                    p[at..at + 4].copy_from_slice(&moved.to_be_bytes());
                }
            }
            b"co64" => {
                let count = be_u32(p, 4)? as usize;
                for i in 0..count {
                    let at = 8 + 8 * i;
                    let moved = map.translate(be_u64(p, at)?)?;
                    p[at..at + 8].copy_from_slice(&moved.to_be_bytes());
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// Rewrites explicit `tfhd.base_data_offset` values inside a `moof` payload.
fn patch_fragment_offsets(data: &mut [u8], map: &OffsetMap) -> MediaResult<()> {
    for b in children(data)? {
        let p = &mut data[b.start + b.header..b.end];
        match &b.kind {
            b"traf" => patch_fragment_offsets(p, map)?,
            b"tfhd" if be_u32(p, 0)? & 0x01 != 0 => {
                let moved = map.translate(be_u64(p, 8)?)?;
                p[8..16].copy_from_slice(&moved.to_be_bytes());
            }
            _ => {}
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Streaming top level
// ---------------------------------------------------------------------------

struct TopBox {
    kind: Kind,
    start: u64,
    header: u64,
    size: u64,
}

fn index_top_level(input: &mut dyn ReadSeek, len: u64) -> MediaResult<Vec<TopBox>> {
    let mut boxes = Vec::new();
    let mut pos = 0u64;
    while pos < len {
        let remaining = len - pos;
        let mut h = [0u8; 16];
        input.seek(SeekFrom::Start(pos))?;
        let n = read_up_to(input, &mut h)?;
        if n < 8 {
            return Err(malformed("truncated top-level box"));
        }
        let kind: Kind = h[4..8].try_into().unwrap();
        let (header, size) = match be_u32(&h, 0)? {
            1 if n == 16 => (16, be_u64(&h, 8)?),
            1 => return Err(malformed("truncated top-level box")),
            0 => (8, remaining),
            s => (8, s as u64),
        };
        if size < header || size > remaining {
            return Err(malformed("top-level box size out of range"));
        }
        boxes.push(TopBox { kind, start: pos, header, size });
        pos += size;
    }
    Ok(boxes)
}

fn load_payload(input: &mut dyn ReadSeek, b: &TopBox) -> MediaResult<Vec<u8>> {
    let len = b.size - b.header;
    if len > IN_MEMORY_CAP {
        return Err(MediaLibError::ContainerTooLarge);
    }
    let mut payload = vec![0u8; len as usize];
    read_at(input, b.start + b.header, &mut payload)?;
    Ok(payload)
}

/// Copies `[start, start + len)` while writing zeros over `zero` ranges.
fn copy_zeroing(
    input: &mut dyn ReadSeek,
    out: &mut dyn Write,
    start: u64,
    len: u64,
    zero: &[(u64, u64)],
) -> MediaResult<()> {
    let end = start + len;
    let mut pos = start;
    for &(zs, ze) in zero {
        if ze <= pos || zs >= end {
            continue;
        }
        let (zs, ze) = (zs.max(pos), ze.min(end));
        copy_range(input, out, pos, zs - pos)?;
        io::copy(&mut io::repeat(0).take(ze - zs), out)?;
        pos = ze;
    }
    copy_range(input, out, pos, end - pos)
}

fn merge_ranges(mut ranges: Vec<(u64, u64)>) -> Vec<(u64, u64)> {
    ranges.retain(|(s, e)| e > s);
    ranges.sort_unstable();
    let mut merged: Vec<(u64, u64)> = Vec::with_capacity(ranges.len());
    for (s, e) in ranges {
        match merged.last_mut() {
            Some(last) if s <= last.1 => last.1 = last.1.max(e),
            _ => merged.push((s, e)),
        }
    }
    merged
}

impl MetadataStripper for IsoBmff {
    fn strip(&self, input: &mut dyn ReadSeek, out: &mut dyn Write) -> MediaResult<()> {
        let len = stream_len(input)?;
        let kept: Vec<TopBox> = index_top_level(input, len)?
            .into_iter()
            .filter(|b| KEPT_TOP.contains(&&b.kind))
            .collect();
        if !kept.iter().any(|b| &b.kind == b"moov") {
            return Err(malformed("missing moov box"));
        }

        // Pass 1: rewrite moov in memory, keyed by its original start. A
        // moof's size never changes, so fragments wait until pass 2.
        let mut zero = Vec::new();
        let mut rewritten: HashMap<u64, Vec<u8>> = HashMap::new();
        for b in kept.iter().filter(|b| &b.kind == b"moov") {
            let payload = load_payload(input, b)?;
            rewritten.insert(b.start, rewrite_container(&payload, &mut zero)?);
        }

        // New layout and offset map.
        let mut map = Vec::with_capacity(kept.len());
        let mut new_start = 0u64;
        for b in &kept {
            map.push((b.start, b.start + b.size, new_start));
            new_start += match rewritten.get(&b.start) {
                Some(p) => (box_header(&b.kind, p.len()).len() + p.len()) as u64,
                None => b.size,
            };
        }
        let map = OffsetMap(map);

        for p in rewritten.values_mut() {
            patch_chunk_offsets(p, &map)?;
        }
        let zero = merge_ranges(zero);

        // Pass 2: stream out.
        for b in &kept {
            if let Some(p) = rewritten.get(&b.start) {
                out.write_all(&box_header(&b.kind, p.len()))?;
                out.write_all(p)?;
            } else if &b.kind == b"moof" {
                let mut payload = load_payload(input, b)?;
                patch_fragment_offsets(&mut payload, &map)?;
                copy_range(input, out, b.start, b.header)?;
                out.write_all(&payload)?;
            } else if &b.kind == b"mdat" {
                copy_range(input, out, b.start, b.header)?;
                copy_zeroing(input, out, b.start + b.header, b.size - b.header, &zero)?;
            } else {
                copy_range(input, out, b.start, b.size)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strip::testutil::{self, contains, Mp4Layout};
    use crate::strip::strip_to_vec;

    fn stripped(layout: Mp4Layout) -> Vec<u8> {
        strip_to_vec(&IsoBmff, testutil::mp4(layout)).unwrap()
    }

    fn top_payload<'a>(file: &'a [u8], kind: &Kind) -> &'a [u8] {
        find_child(file, kind).unwrap().unwrap()
    }

    fn check(out: &[u8]) {
        for secret in [&b"+12.3+045.6"[..], b"secret", b"GPSDATA"] {
            assert!(!contains(out, secret), "{}", String::from_utf8_lossy(secret));
        }

        let moov = top_payload(out, b"moov");
        let traks = children(moov).unwrap().into_iter().filter(|b| &b.kind == b"trak").count();
        assert_eq!(traks, 1, "metadata track removed");

        // The surviving track's chunk offset points at its sample.
        let stco = find_path(moov, &[b"trak", b"mdia", b"minf", b"stbl", b"stco"]).unwrap().unwrap();
        let offset = be_u32(stco, 8).unwrap() as usize;
        assert_eq!(&out[offset..offset + testutil::MP4_VIDEO.len()], testutil::MP4_VIDEO);

        // Timestamps zeroed (mvhd is version 1, tkhd/mdhd version 0).
        let mvhd = find_child(moov, b"mvhd").unwrap().unwrap();
        assert!(mvhd[4..20].iter().all(|&b| b == 0));
        let mdhd = find_path(moov, &[b"trak", b"mdia", b"mdhd"]).unwrap().unwrap();
        assert!(mdhd[4..12].iter().all(|&b| b == 0));
        assert_eq!(be_u32(mdhd, 12).unwrap(), 1000, "timescale untouched");
    }

    #[test]
    fn faststart_file() {
        let out = stripped(Mp4Layout::FastStart);
        check(&out);
        // Top level is exactly ftyp, moov, mdat.
        let kinds: Vec<Kind> = children(&out).unwrap().iter().map(|b| b.kind).collect();
        assert_eq!(kinds, [*b"ftyp", *b"moov", *b"mdat"]);
    }

    #[test]
    fn moov_at_end_with_dropped_boxes_before_mdat() {
        let out = stripped(Mp4Layout::MoovAtEnd);
        check(&out);
        let kinds: Vec<Kind> = children(&out).unwrap().iter().map(|b| b.kind).collect();
        assert_eq!(kinds, [*b"ftyp", *b"mdat", *b"moov"]);
    }

    #[test]
    fn metadata_samples_are_zeroed_in_place() {
        let input = testutil::mp4(Mp4Layout::FastStart);
        let out = strip_to_vec(&IsoBmff, input).unwrap();
        let mdat = top_payload(&out, b"mdat");
        assert_eq!(mdat.len(), testutil::MP4_VIDEO.len() + testutil::MP4_META.len());
        assert!(mdat[testutil::MP4_VIDEO.len()..].iter().all(|&b| b == 0));
    }

    #[test]
    fn missing_moov_is_malformed() {
        let mut file = testutil::bx(b"ftyp", b"isom\0\0\0\0");
        file.extend(testutil::bx(b"mdat", b"data"));
        assert!(matches!(
            strip_to_vec(&IsoBmff, file),
            Err(MediaLibError::MalformedContainer(_))
        ));
    }

    #[test]
    fn oversized_box_is_malformed() {
        let mut file = testutil::bx(b"ftyp", b"isom\0\0\0\0");
        file.extend_from_slice(&1000u32.to_be_bytes());
        file.extend_from_slice(b"moov");
        assert!(matches!(
            strip_to_vec(&IsoBmff, file),
            Err(MediaLibError::MalformedContainer(_))
        ));
    }

    #[test]
    fn merge_ranges_coalesces() {
        assert_eq!(merge_ranges(vec![(5, 9), (0, 3), (2, 4), (9, 9)]), vec![(0, 4), (5, 9)]);
    }
}
