/// An error encountered while framing or parsing an NMEA sentence.
///
/// The framing errors ([`Empty`](Self::Empty),
/// [`NoDelimiter`](Self::NoDelimiter), [`NoChecksum`](Self::NoChecksum),
/// [`BadChecksum`](Self::BadChecksum)) come from [`Sentence::parse`], the
/// field errors from the individual sentence parsers.
///
/// [`Sentence::parse`]: crate::Sentence::parse
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseError {
    /// The line was empty or carried an empty address field.
    Empty,
    /// The line did not start with a `$` or `!` delimiter.
    NoDelimiter,
    /// The line had no `*` marker followed by two hexadecimal digits.
    NoChecksum,
    /// The computed checksum did not match the one in the sentence.
    BadChecksum {
        /// The checksum the sentence claimed.
        expected: u8,
        /// The checksum computed over the sentence body.
        actual: u8,
    },
    /// A required field was missing (the sentence ended too early).
    MissingField,
    /// A numeric field could not be parsed as a number.
    InvalidNumber,
    /// A field held a value outside its expected set, e.g. a bad
    /// hemisphere letter or an unknown enumerated code.
    InvalidField,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => f.write_str("empty sentence"),
            Self::NoDelimiter => f.write_str("missing '$' or '!' start delimiter"),
            Self::NoChecksum => f.write_str("missing or malformed checksum"),
            Self::BadChecksum { expected, actual } => {
                write!(
                    f,
                    "checksum mismatch: expected {expected:02X}, computed {actual:02X}"
                )
            }
            Self::MissingField => f.write_str("missing required field"),
            Self::InvalidNumber => f.write_str("invalid numeric field"),
            Self::InvalidField => f.write_str("invalid field value"),
        }
    }
}

impl std::error::Error for ParseError {}
