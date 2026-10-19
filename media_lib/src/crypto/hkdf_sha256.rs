use dryoc::hkdf::{HkdfSha256, HkdfSha256Prk};
use dryoc::types::Bytes;

use crate::error::{MediaLibError, MediaResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HkdfSha256Crypto;

impl HkdfSha256Crypto {
    pub fn extract(
        salt: &[u8],
        ikm: &[u8],
    ) -> Vec<u8> {
        HkdfSha256::extract(Some(salt), ikm)
            .into_prk()
            .as_slice()
            .to_vec()
    }

    pub(crate) fn expand(
        prk: &[u8],
        info: &[u8],
        length: usize,
    ) -> MediaResult<Vec<u8>> {
        let prk = HkdfSha256Prk::try_from(prk)
            .map_err(|_| MediaLibError::KeyDerivation)?;

        HkdfSha256::from_prk(prk)
            .expand_to_vec(info, length)
            .map_err(|_| MediaLibError::KeyDerivation)
    }
}