//! Overwrite plaintext before its memory is released.

use std::ops::{Deref, DerefMut};
use std::sync::atomic::{compiler_fence, Ordering};

/// Zeroes `buf` with volatile writes the optimiser can't elide.
pub fn wipe(buf: &mut [u8]) {
    for byte in buf.iter_mut() {
        // SAFETY: `byte` is a valid, aligned, exclusive reference.
        unsafe { std::ptr::write_volatile(byte, 0) };
    }
    compiler_fence(Ordering::SeqCst);
}

/// A buffer that is wiped when dropped, including on early `?` returns.
pub struct WipeOnDrop(Vec<u8>);

impl WipeOnDrop {
    pub fn new(buf: Vec<u8>) -> Self {
        Self(buf)
    }

    /// Hands the buffer to the caller, who becomes responsible for it.
    pub fn into_inner(mut self) -> Vec<u8> {
        std::mem::take(&mut self.0)
    }
}

impl Deref for WipeOnDrop {
    type Target = Vec<u8>;
    fn deref(&self) -> &Vec<u8> {
        &self.0
    }
}

impl DerefMut for WipeOnDrop {
    fn deref_mut(&mut self) -> &mut Vec<u8> {
        &mut self.0
    }
}

impl Drop for WipeOnDrop {
    fn drop(&mut self) {
        // The whole allocation, not just `len`: earlier contents may linger.
        let capacity = self.0.capacity();
        self.0.resize(capacity, 0);
        wipe(&mut self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wipes_slices() {
        let mut buf = vec![7u8; 32];
        wipe(&mut buf);
        assert!(buf.iter().all(|&b| b == 0));
    }

    #[test]
    fn into_inner_keeps_contents() {
        let guard = WipeOnDrop::new(vec![1, 2, 3]);
        assert_eq!(guard.into_inner(), vec![1, 2, 3]);
    }
}
