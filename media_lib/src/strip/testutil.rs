//! Builders for synthetic media files seeded with recognisable metadata.

use super::crc32::crc32;

pub fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

pub fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    find(haystack, needle).is_some()
}

/// Little-endian TIFF with Make ("PixelSecret"), Orientation and a GPS IFD
/// holding "GPSSECRET".
pub fn exif_tiff_le(orientation: u16) -> Vec<u8> {
    let mut t = Vec::new();
    t.extend_from_slice(b"II");
    t.extend_from_slice(&42u16.to_le_bytes());
    t.extend_from_slice(&8u32.to_le_bytes());
    // IFD0 at 8: 3 entries -> ends at 8 + 2 + 36 + 4 = 50.
    t.extend_from_slice(&3u16.to_le_bytes());
    let entry = |t: &mut Vec<u8>, tag: u16, kind: u16, count: u32, value: u32| {
        t.extend_from_slice(&tag.to_le_bytes());
        t.extend_from_slice(&kind.to_le_bytes());
        t.extend_from_slice(&count.to_le_bytes());
        t.extend_from_slice(&value.to_le_bytes());
    };
    entry(&mut t, 0x010F, 2, 12, 50); // Make -> offset 50
    entry(&mut t, 0x0112, 3, 1, orientation as u32);
    entry(&mut t, 0x8825, 4, 1, 62); // GPS IFD -> offset 62
    t.extend_from_slice(&0u32.to_le_bytes());
    t.extend_from_slice(b"PixelSecret\0"); // 50..62
    t.extend_from_slice(&1u16.to_le_bytes()); // GPS IFD: 1 entry
    entry(&mut t, 0x0001, 2, 10, 80);
    t.extend_from_slice(&0u32.to_le_bytes());
    t.extend_from_slice(b"GPSSECRET\0"); // 80..90
    t
}

// --------------------------------------------------------------------- JPEG

/// Entropy data with a stuffed byte (FF00) and a restart marker (FFD3).
pub const JPEG_SCAN: &[u8] = &[0x11, 0xFF, 0x00, 0x12, 0xFF, 0xD3, 0x34, 0x56];
pub const JPEG_TRAILER: &[u8] = b"TRAILER motion photo video";

fn jpeg_segment(out: &mut Vec<u8>, marker: u8, payload: &[u8]) {
    out.extend_from_slice(&[0xFF, marker]);
    out.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
    out.extend_from_slice(payload);
}

pub fn jpeg_with_metadata(orientation: u16) -> Vec<u8> {
    let mut j = vec![0xFF, 0xD8];
    jpeg_segment(&mut j, 0xE0, b"JFIF\0\x01\x01\0\0\x01\0\x01\0\0");
    let mut exif = b"Exif\0\0".to_vec();
    exif.extend(exif_tiff_le(orientation));
    jpeg_segment(&mut j, 0xE1, &exif);
    jpeg_segment(&mut j, 0xE1, b"http://ns.adobe.com/xap/1.0/\0<x:xmpmeta>GPSSECRET</x:xmpmeta>");
    jpeg_segment(&mut j, 0xE2, b"ICC_PROFILE\0\x01\x01profiledata");
    jpeg_segment(&mut j, 0xED, b"Photoshop 3.0\0IPTC secret");
    jpeg_segment(&mut j, 0xFE, b"a comment");
    jpeg_segment(&mut j, 0xDB, &[0u8; 65]);
    jpeg_segment(&mut j, 0xC0, &[8, 0, 1, 0, 1, 1, 1, 0x11, 0]);
    jpeg_segment(&mut j, 0xC4, &[0u8; 17]);
    jpeg_segment(&mut j, 0xDA, &[1, 1, 0, 0, 0x3F, 0]);
    j.extend_from_slice(JPEG_SCAN);
    j.extend_from_slice(&[0xFF, 0xD9]);
    j.extend_from_slice(JPEG_TRAILER);
    j
}

// ---------------------------------------------------------------------- PNG

fn png_chunk(out: &mut Vec<u8>, kind: &[u8], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    out.extend_from_slice(&crc32(&[kind, data]).to_be_bytes());
}

pub fn png_with_metadata(orientation: u16) -> Vec<u8> {
    let mut p = b"\x89PNG\r\n\x1a\n".to_vec();
    png_chunk(&mut p, b"IHDR", &[0, 0, 0, 1, 0, 0, 0, 1, 8, 2, 0, 0, 0]);
    png_chunk(&mut p, b"sRGB", &[0]);
    png_chunk(&mut p, b"tEXt", b"Author\0secret GPSSECRET");
    png_chunk(&mut p, b"tIME", &[7, 0xE9, 1, 1, 0, 0, 0]);
    png_chunk(&mut p, b"eXIf", &exif_tiff_le(orientation));
    png_chunk(&mut p, b"zzZz", b"private");
    png_chunk(&mut p, b"IDAT", b"compressed-pixels");
    png_chunk(&mut p, b"IEND", b"");
    p.extend_from_slice(b"TRAILER");
    p
}

// --------------------------------------------------------------------- WebP

fn riff_chunk(out: &mut Vec<u8>, kind: &[u8], data: &[u8]) {
    out.extend_from_slice(kind);
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(data);
    if data.len() % 2 == 1 {
        out.push(0);
    }
}

fn riff(form: &[u8], body: Vec<u8>) -> Vec<u8> {
    let mut r = b"RIFF".to_vec();
    r.extend_from_slice(&((body.len() + 4) as u32).to_le_bytes());
    r.extend_from_slice(form);
    r.extend(body);
    r
}

pub fn webp_with_metadata(orientation: u16) -> Vec<u8> {
    let mut body = Vec::new();
    riff_chunk(&mut body, b"VP8X", &[0x0C, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    riff_chunk(&mut body, b"VP8L", &[0x2F, 0, 0, 0, 0x10]);
    let mut exif = b"Exif\0\0".to_vec();
    exif.extend(exif_tiff_le(orientation));
    riff_chunk(&mut body, b"EXIF", &exif);
    riff_chunk(&mut body, b"XMP ", b"<x:xmpmeta>GPSSECRET</x:xmpmeta>");
    riff(b"WEBP", body)
}

// ---------------------------------------------------------------------- GIF

pub fn gif_with_metadata() -> Vec<u8> {
    let mut g = b"GIF89a".to_vec();
    g.extend_from_slice(&[1, 0, 1, 0, 0x80, 0, 0]); // 1x1, global table of 2
    g.extend_from_slice(&[0, 0, 0, 255, 255, 255]);
    g.extend_from_slice(b"\x21\xFF\x0BNETSCAPE2.0\x03\x01\x00\x00\x00");
    g.extend_from_slice(b"\x21\xFF\x0BXMP DataXMP\x05hello\x00");
    g.extend_from_slice(b"\x21\xFE\x09a comment\x00");
    g.extend_from_slice(&[0x21, 0xF9, 0x04, 0, 0, 0, 0, 0]);
    g.extend_from_slice(&[0x2C, 0, 0, 0, 0, 1, 0, 1, 0, 0]);
    g.extend_from_slice(&[0x02, 0x02, 0x4C, 0x01, 0x00]);
    g.push(0x3B);
    g.extend_from_slice(b"TRAILER");
    g
}

// -------------------------------------------------------------------- Audio

pub fn id3v2(frames: &[u8]) -> Vec<u8> {
    let size = frames.len() as u32;
    let mut t = b"ID3\x03\x00\x00".to_vec();
    t.extend_from_slice(&[
        ((size >> 21) & 0x7F) as u8,
        ((size >> 14) & 0x7F) as u8,
        ((size >> 7) & 0x7F) as u8,
        (size & 0x7F) as u8,
    ]);
    t.extend_from_slice(frames);
    t
}

pub fn id3v1() -> Vec<u8> {
    let mut t = b"TAG".to_vec();
    t.extend_from_slice(b"secret title");
    t.resize(128, 0);
    t
}

pub fn mp3_frames() -> Vec<u8> {
    let mut frames = Vec::new();
    for i in 0..3u8 {
        frames.extend_from_slice(&[0xFF, 0xFB, 0x90, 0x00]);
        frames.extend((0..400).map(|n| (n as u8).wrapping_mul(7).wrapping_add(i)));
    }
    frames
}

pub fn mp3_with_tags(frames: &[u8]) -> Vec<u8> {
    let mut f = id3v2(b"TIT2\0\0\0\x07\0\0\0secret");
    f.extend_from_slice(frames);

    // Lyrics3v2
    let lyrics = b"LYRICSBEGINLYR00006secret";
    f.extend_from_slice(lyrics);
    f.extend_from_slice(format!("{:06}", lyrics.len()).as_bytes());
    f.extend_from_slice(b"LYRICS200");

    // APEv2 with header and footer
    let items = b"\x06\0\0\0\0\0\0\0Title\0secret";
    let size = (items.len() + 32) as u32;
    let ape = |flags: u32| {
        let mut h = b"APETAGEX".to_vec();
        h.extend_from_slice(&2000u32.to_le_bytes());
        h.extend_from_slice(&size.to_le_bytes());
        h.extend_from_slice(&1u32.to_le_bytes());
        h.extend_from_slice(&flags.to_le_bytes());
        h.extend_from_slice(&[0u8; 8]);
        h
    };
    f.extend(ape(0xA000_0000));
    f.extend_from_slice(items);
    f.extend(ape(0x8000_0000));

    f.extend(id3v1());
    f
}

pub const FLAC_FRAMES: &[u8] = b"\xFF\xF8\x69\x08FRAMEDATA-0123456789";

pub fn flac_with_metadata() -> Vec<u8> {
    let block = |out: &mut Vec<u8>, kind: u8, last: bool, data: &[u8]| {
        let len = (data.len() as u32).to_be_bytes();
        out.push(kind | if last { 0x80 } else { 0 });
        out.extend_from_slice(&len[1..]);
        out.extend_from_slice(data);
    };
    let mut f = id3v2(b"TIT2secret");
    f.extend_from_slice(b"fLaC");
    block(&mut f, 0, false, &[0u8; 34]);
    block(&mut f, 4, false, b"\x07\0\0\0vendor!\x01\0\0\0\x0D\0\0\0ARTIST=secret");
    block(&mut f, 6, false, b"PICTUREDATA");
    block(&mut f, 3, false, &[0x11u8; 18]);
    block(&mut f, 1, true, &[0u8; 16]);
    f.extend_from_slice(FLAC_FRAMES);
    f.extend(id3v1());
    f
}

pub const WAV_SAMPLES: &[u8] = b"\x01\x02\x03\x04\x05\x06\x07";

pub fn wav_with_metadata() -> Vec<u8> {
    let mut body = Vec::new();
    riff_chunk(&mut body, b"fmt ", &[1, 0, 1, 0, 0x44, 0xAC, 0, 0, 0x88, 0x58, 1, 0, 2, 0, 16, 0]);
    riff_chunk(&mut body, b"LIST", b"INFOINAM\x07\0\0\0secret\0");
    riff_chunk(&mut body, b"bext", b"originator: secret");
    riff_chunk(&mut body, b"data", WAV_SAMPLES);
    riff(b"WAVE", body)
}

// ---------------------------------------------------------------------- MP4

pub const MP4_VIDEO: &[u8] = b"VIDEOSAMPLE-0123456789";
pub const MP4_META: &[u8] = b"GPSDATA!+12.3+045.6/";

pub fn bx(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut b = ((payload.len() + 8) as u32).to_be_bytes().to_vec();
    b.extend_from_slice(kind);
    b.extend_from_slice(payload);
    b
}

fn full(kind: &[u8; 4], version: u8, body: &[u8]) -> Vec<u8> {
    let mut p = vec![version, 0, 0, 0];
    p.extend_from_slice(body);
    bx(kind, &p)
}

fn u32s(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_be_bytes()).collect()
}

fn trak(handler: &[u8; 4], offset: u32, len: u32) -> Vec<u8> {
    let mut tkhd = u32s(&[0x1111_1111, 0x2222_2222, 1, 0, 1000]);
    tkhd.resize(80, 0);
    let mdhd = [u32s(&[0x1111_1111, 0x2222_2222, 1000, 1000]), vec![0x55, 0xC4, 0, 0]].concat();
    let mut hdlr = vec![0u8; 4];
    hdlr.extend_from_slice(handler);
    hdlr.extend_from_slice(&[0u8; 12]);
    hdlr.extend_from_slice(b"Handler\0");

    let stbl = [
        full(b"stsc", 0, &u32s(&[1, 1, 1, 1])),
        full(b"stsz", 0, &u32s(&[0, 1, len])),
        full(b"stco", 0, &u32s(&[1, offset])),
    ]
    .concat();
    let minf = bx(b"minf", &bx(b"stbl", &stbl));
    let mdia = bx(b"mdia", &[full(b"mdhd", 0, &mdhd), full(b"hdlr", 0, &hdlr), minf].concat());
    bx(b"trak", &[full(b"tkhd", 0, &tkhd), mdia].concat())
}

fn moov(video_offset: u32, meta_offset: u32) -> Vec<u8> {
    let mut mvhd = Vec::new();
    mvhd.extend_from_slice(&0x1111_1111_1111_1111u64.to_be_bytes());
    mvhd.extend_from_slice(&0x2222_2222_2222_2222u64.to_be_bytes());
    mvhd.extend_from_slice(&1000u32.to_be_bytes());
    mvhd.extend_from_slice(&1000u64.to_be_bytes());
    mvhd.resize(108, 0);

    let udta = bx(b"udta", &bx(&[0xA9, b'x', b'y', b'z'], b"+12.3+045.6/"));
    let meta = full(b"meta", 0, &bx(b"keys", b"com.apple.quicktime.location.ISO6709 secret"));
    bx(
        b"moov",
        &[
            full(b"mvhd", 1, &mvhd),
            udta,
            meta,
            trak(b"vide", video_offset, MP4_VIDEO.len() as u32),
            trak(b"meta", meta_offset, MP4_META.len() as u32),
        ]
        .concat(),
    )
}

pub enum Mp4Layout {
    /// ftyp, moov, mdat
    FastStart,
    /// ftyp, free, udta, mdat, moov
    MoovAtEnd,
}

pub fn mp4(layout: Mp4Layout) -> Vec<u8> {
    let ftyp = bx(b"ftyp", b"isom\0\0\x02\0isomiso2mp41");
    let mdat = bx(b"mdat", &[MP4_VIDEO, MP4_META].concat());
    match layout {
        Mp4Layout::FastStart => {
            let data = (ftyp.len() + moov(0, 0).len() + 8) as u32;
            [ftyp, moov(data, data + MP4_VIDEO.len() as u32), mdat].concat()
        }
        Mp4Layout::MoovAtEnd => {
            let free = bx(b"free", b"old secret bytes");
            let udta = bx(b"udta", b"secret top-level udta");
            let data = (ftyp.len() + free.len() + udta.len() + 8) as u32;
            [ftyp, free, udta, mdat, moov(data, data + MP4_VIDEO.len() as u32)].concat()
        }
    }
}
