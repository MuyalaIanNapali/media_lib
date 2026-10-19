//! Encryption schemes and keys.
//!
//! Each on-disk format is an [`EncryptionScheme`]; a [`SchemeSelector`]
//! decides which one a media category uses, so callers depend on the
//! abstractions rather than on a specific cipher.

pub mod chunked_ctr;
pub(crate) mod hkdf_sha256;
pub mod keys;
pub mod secretstream;
pub mod wipe;

use std::io::{Read, Seek, Write};

use crate::error::MediaResult;
use crate::media::MediaCategory;
use keys::MediaKey;

/// A seekable byte sink; blanket-implemented for every `Write + Seek`.
pub trait WriteSeek: Write + Seek {}
impl<T: Write + Seek + ?Sized> WriteSeek for T {}

/// One on-disk encryption format.
pub trait EncryptionScheme: Sync {
    fn encrypt(&self, key: &MediaKey, input: &mut dyn Read, output: &mut dyn WriteSeek) -> MediaResult<()>;
}

/// Chooses the encryption scheme for a media category.
pub trait SchemeSelector: Sync {
    fn scheme_for(&self, category: MediaCategory) -> &dyn EncryptionScheme;
}

/// Images use whole-file secretstream (they are decrypted whole anyway);
/// audio and video use chunked AES-CTR so they can be decrypted by range.
pub struct StandardSchemes;

impl SchemeSelector for StandardSchemes {
    fn scheme_for(&self, category: MediaCategory) -> &dyn EncryptionScheme {
        match category {
            MediaCategory::Image => &secretstream::SecretStream,
            MediaCategory::Audio | MediaCategory::Video => &chunked_ctr::ChunkedCtrScheme,
        }
    }
}
