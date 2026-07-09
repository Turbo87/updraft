use thiserror::Error;

/// An error produced while decoding an `.lxg` file.
///
/// The decoder is total: every byte slice either decodes to an
/// [`LxgFile`](crate::LxgFile) or produces one of these errors — it never
/// panics.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum Error {
    /// The input ended in the middle of a value (a length or payload ran
    /// past the end of the buffer).
    #[error("unexpected end of input")]
    UnexpectedEof,
    /// The file did not start with the `0xCC 0xFF` version prefix.
    #[error("missing 0xCC 0xFF version prefix")]
    MissingVersion,
    /// A value used a tag byte this format does not use.
    #[error("unsupported tag 0x{0:02X}")]
    UnsupportedTag(u8),
    /// A section header was not followed by the expected `0xD5 0x2F`
    /// fixext-2 size marker.
    #[error("section header missing 0xD5 0x2F size marker")]
    MalformedSectionHeader,
    /// A section's entries were not terminated by the `0xC0` byte.
    #[error("section not terminated by 0xC0")]
    MissingSectionTerminator,
    /// A map key was not encoded as an unsigned 16-bit integer, as every
    /// key in this format is.
    #[error("map key was not a u16")]
    NonIntegerKey,
    /// A string field was not valid UTF-8.
    #[error("string field was not valid UTF-8")]
    InvalidUtf8,
    /// Bytes remained after the root section was decoded.
    #[error("trailing bytes after root section")]
    TrailingData,
}
