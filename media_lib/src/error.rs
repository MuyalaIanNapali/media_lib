//! Typed errors for every fallible operation in `media_lib`.
//!
//! Each variant has a stable numeric [`MediaLibError::code`] that crosses the
//! JNI boundary inside `org.anon.mediaplayer.MediaLibException`. Kotlin mirrors
//! these codes in `NativeErrorCode`, so codes must never be renumbered or
//! reused — only appended.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MediaLibError {
    /// Opening, reading, writing, seeking or renaming a file failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The supplied key is not exactly 32 bytes.
    #[error("key must be 32 bytes, got {0}")]
    InvalidKeyLength(usize),

    /// The MIME type is neither image, audio nor video.
    #[error("unsupported MIME type: {0}")]
    UnsupportedMimeType(String),

    /// The secretstream header could not be parsed.
    #[error("invalid stream header")]
    InvalidHeader,

    /// The CHATMED2 header has an unknown magic, version or chunk size.
    #[error("unsupported encrypted format (magic, version or chunk size)")]
    UnsupportedFormat,

    /// The header MAC did not verify: wrong key or tampered header.
    #[error("header authentication failed")]
    HeaderAuthenticationFailed,

    /// A chunk failed authentication: wrong key or tampered ciphertext.
    #[error("chunk {0} authentication failed")]
    ChunkAuthenticationFailed(u64),

    /// The stream ended before its final chunk.
    #[error("ciphertext truncated: stream ended before final chunk")]
    Truncated,

    /// Bytes follow the final chunk.
    #[error("trailing data after final chunk")]
    TrailingData,

    /// A chunk length prefix is zero or larger than the format allows.
    #[error("invalid chunk length: {0}")]
    InvalidChunkLength(usize),

    /// The encrypted file is not the size its header implies.
    #[error("encrypted file has an unexpected size or is truncated")]
    FileSizeMismatch,

    /// A requested plaintext range is negative, out of bounds or overflows.
    #[error("invalid range: {0}")]
    InvalidRange(&'static str),

    /// HKDF expansion failed.
    #[error("key derivation failed")]
    KeyDerivation,

    /// The encryption primitive itself reported a failure.
    #[error("encryption failed: {0}")]
    EncryptionFailed(String),

    /// A JNI call (string/array conversion, object creation) failed.
    #[error("JNI error: {0}")]
    Jni(#[from] jni::errors::Error),

    /// The file's container format has no metadata stripper (or doesn't match
    /// its MIME type), so it is refused rather than stored with metadata.
    #[error("unsupported container: {0}")]
    UnsupportedContainer(String),

    /// The source file's container structure is invalid.
    #[error("malformed container: {0}")]
    MalformedContainer(&'static str),

    /// An image or MP4 `moov` box exceeds the in-memory processing limit.
    #[error("container structure too large to process")]
    ContainerTooLarge,
}

impl MediaLibError {
    /// Stable code passed to Kotlin. Append new codes; never renumber.
    pub fn code(&self) -> i32 {
        match self {
            Self::Io(_) => 1,
            Self::InvalidKeyLength(_) => 2,
            Self::UnsupportedMimeType(_) => 3,
            Self::InvalidHeader => 4,
            Self::UnsupportedFormat => 5,
            Self::HeaderAuthenticationFailed => 6,
            Self::ChunkAuthenticationFailed(_) => 7,
            Self::Truncated => 8,
            Self::TrailingData => 9,
            Self::InvalidChunkLength(_) => 10,
            Self::FileSizeMismatch => 11,
            Self::InvalidRange(_) => 12,
            Self::KeyDerivation => 13,
            Self::EncryptionFailed(_) => 14,
            Self::Jni(_) => 15,
            Self::UnsupportedContainer(_) => 16,
            Self::MalformedContainer(_) => 17,
            Self::ContainerTooLarge => 18,
        }
    }
}

pub type MediaResult<T> = Result<T, MediaLibError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn all_variants() -> Vec<MediaLibError> {
        vec![
            MediaLibError::Io(std::io::Error::other("x")),
            MediaLibError::InvalidKeyLength(0),
            MediaLibError::UnsupportedMimeType(String::new()),
            MediaLibError::InvalidHeader,
            MediaLibError::UnsupportedFormat,
            MediaLibError::HeaderAuthenticationFailed,
            MediaLibError::ChunkAuthenticationFailed(0),
            MediaLibError::Truncated,
            MediaLibError::TrailingData,
            MediaLibError::InvalidChunkLength(0),
            MediaLibError::FileSizeMismatch,
            MediaLibError::InvalidRange(""),
            MediaLibError::KeyDerivation,
            MediaLibError::EncryptionFailed(String::new()),
            MediaLibError::Jni(jni::errors::Error::NullPtr("x")),
            MediaLibError::UnsupportedContainer(String::new()),
            MediaLibError::MalformedContainer(""),
            MediaLibError::ContainerTooLarge,
        ]
    }

    #[test]
    fn codes_are_unique() {
        let codes: Vec<i32> = all_variants().iter().map(MediaLibError::code).collect();
        let unique: HashSet<i32> = codes.iter().copied().collect();
        assert_eq!(codes.len(), unique.len());
    }

    #[test]
    fn codes_are_stable() {
        // Kotlin's NativeErrorCode depends on these exact values.
        let codes: Vec<i32> = all_variants().iter().map(MediaLibError::code).collect();
        assert_eq!(codes, (1..=18).collect::<Vec<i32>>());
    }
}
