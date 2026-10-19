//! Formats that are recognised but have no stripper, named in the error so
//! the refusal is explicit.

const SIGNATURES: [(&[u8], &str); 4] = [
    (b"OggS", "Ogg"),
    (&[0x1A, 0x45, 0xDF, 0xA3], "Matroska/WebM"),
    (b"#!AMR", "AMR"),
    (b"RIFF", "RIFF"),
];

/// The name of a known-but-unsupported format, if `head` starts with one.
pub(crate) fn identify(head: &[u8]) -> Option<&'static str> {
    SIGNATURES.iter().find(|(magic, _)| head.starts_with(magic)).map(|(_, name)| *name)
}
