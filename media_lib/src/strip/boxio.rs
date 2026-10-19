//! Small, bounds-checked byte helpers shared by the strippers.

use std::io::{self, Read, Seek, SeekFrom, Write};

use crate::error::{MediaLibError, MediaResult};

/// A seekable byte source; blanket-implemented for every `Read + Seek`.
pub trait ReadSeek: Read + Seek {}
impl<T: Read + Seek + ?Sized> ReadSeek for T {}

fn slice(data: &[u8], offset: usize, len: usize) -> MediaResult<&[u8]> {
    offset
        .checked_add(len)
        .and_then(|end| data.get(offset..end))
        .ok_or(MediaLibError::MalformedContainer("structure extends past its container"))
}

pub fn be_u16(data: &[u8], offset: usize) -> MediaResult<u16> {
    Ok(u16::from_be_bytes(slice(data, offset, 2)?.try_into().unwrap()))
}

pub fn be_u32(data: &[u8], offset: usize) -> MediaResult<u32> {
    Ok(u32::from_be_bytes(slice(data, offset, 4)?.try_into().unwrap()))
}

pub fn be_u64(data: &[u8], offset: usize) -> MediaResult<u64> {
    Ok(u64::from_be_bytes(slice(data, offset, 8)?.try_into().unwrap()))
}

pub fn le_u32(data: &[u8], offset: usize) -> MediaResult<u32> {
    Ok(u32::from_le_bytes(slice(data, offset, 4)?.try_into().unwrap()))
}

/// Bounds-checked sub-slice.
pub fn bytes(data: &[u8], offset: usize, len: usize) -> MediaResult<&[u8]> {
    slice(data, offset, len)
}

pub fn stream_len(input: &mut dyn ReadSeek) -> MediaResult<u64> {
    Ok(input.seek(SeekFrom::End(0))?)
}

/// Reads as many bytes as are available, up to `buf.len()`.
pub fn read_up_to(input: &mut dyn Read, buf: &mut [u8]) -> MediaResult<usize> {
    Ok(crate::io_util::fill(input, buf)?)
}

/// Reads exactly `buf.len()` bytes at `offset`.
pub fn read_at(input: &mut dyn ReadSeek, offset: u64, buf: &mut [u8]) -> MediaResult<()> {
    input.seek(SeekFrom::Start(offset))?;
    input.read_exact(buf).map_err(|e| match e.kind() {
        io::ErrorKind::UnexpectedEof => MediaLibError::MalformedContainer("unexpected end of file"),
        _ => MediaLibError::Io(e),
    })
}

/// Streams exactly `len` bytes starting at `offset` from `input` to `out`.
pub fn copy_range(input: &mut dyn ReadSeek, out: &mut dyn Write, offset: u64, len: u64) -> MediaResult<()> {
    input.seek(SeekFrom::Start(offset))?;
    let copied = io::copy(&mut input.take(len), out)?;
    if copied != len {
        return Err(MediaLibError::MalformedContainer("unexpected end of file"));
    }
    Ok(())
}
