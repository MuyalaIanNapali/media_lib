//! Read helpers shared by the crypto and strip modules.

use std::io::{self, Read};

use crate::error::{MediaLibError, MediaResult};

/// Reads until `buf` is full or EOF, returning the bytes read. A single
/// `read` may return short mid-stream, so this loops.
pub(crate) fn fill<R: Read + ?Sized>(reader: &mut R, buf: &mut [u8]) -> io::Result<usize> {
    let mut n = 0;
    while n < buf.len() {
        match reader.read(&mut buf[n..]) {
            Ok(0) => break,
            Ok(k) => n += k,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(n)
}

/// `read_exact` on ciphertext: a short read means the file was truncated.
pub(crate) fn read_exact_or_truncated<R: Read + ?Sized>(reader: &mut R, buf: &mut [u8]) -> MediaResult<()> {
    reader.read_exact(buf).map_err(|e| match e.kind() {
        io::ErrorKind::UnexpectedEof => MediaLibError::Truncated,
        _ => MediaLibError::Io(e),
    })
}
