//! Metadata stripping and authenticated encryption for media files.
//!
//! Layers, from the inside out:
//! - [`media`]: media categories (image, audio, video).
//! - [`strip`]: container detection and lossless metadata removal, one
//!   module per format.
//! - [`crypto`]: keys and the two encryption schemes (secretstream for
//!   images, chunked AES-CTR for audio/video).
//! - [`pipeline`]: composes a format registry and a scheme selector into
//!   strip → encrypt.
//! - [`service`]: file-level operations for platform callers.
//! - [`jni_api`]: the thin Android adapter over [`service`].

pub mod crypto;
pub mod error;
mod fs_util;
mod io_util;
pub mod jni_api;
pub mod media;
pub mod media_processor;
pub mod pipeline;
pub mod service;
pub mod strip;
