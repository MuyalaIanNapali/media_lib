//! What kind of content a file holds, independent of its container.

use crate::error::{MediaLibError, MediaResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaCategory {
    Image,
    Audio,
    Video,
}

impl MediaCategory {
    /// The category of a MIME type such as `image/jpeg`.
    pub fn from_mime(mime: &str) -> MediaResult<Self> {
        match mime.split_once('/').map(|(top, _)| top) {
            Some("image") => Ok(Self::Image),
            Some("audio") => Ok(Self::Audio),
            Some("video") => Ok(Self::Video),
            _ => Err(MediaLibError::UnsupportedMimeType(mime.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_top_level_type() {
        assert_eq!(MediaCategory::from_mime("image/jpeg").unwrap(), MediaCategory::Image);
        assert_eq!(MediaCategory::from_mime("audio/mpeg").unwrap(), MediaCategory::Audio);
        assert_eq!(MediaCategory::from_mime("video/mp4").unwrap(), MediaCategory::Video);
        for bad in ["text/plain", "image", ""] {
            assert!(matches!(MediaCategory::from_mime(bad), Err(MediaLibError::UnsupportedMimeType(_))));
        }
    }
}
