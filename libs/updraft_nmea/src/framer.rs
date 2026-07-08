use crate::error::ParseError;
use crate::fields::Fields;

/// Compute the NMEA 0183 checksum: the XOR of every byte between the
/// `$`/`!` start delimiter and the `*` checksum marker (both excluded).
pub fn checksum(data: &[u8]) -> u8 {
    data.iter().fold(0, |acc, &byte| acc ^ byte)
}

/// A framed, checksum-verified NMEA sentence borrowed from the input.
///
/// [`Sentence::parse`] strips the `$` (or `!`) delimiter, verifies the
/// trailing `*HH` checksum, and splits the body into an *address* (the
/// talker+formatter or proprietary tag, e.g. `GPGGA` or `PFLAA`) and its
/// comma-separated fields. It performs no interpretation of the fields
/// themselves — that is the job of the per-sentence parsers, reached
/// through [`crate::parse`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sentence<'a> {
    address: &'a str,
    fields: &'a str,
}

impl<'a> Sentence<'a> {
    /// Frame and checksum-verify a single sentence line.
    ///
    /// Surrounding whitespace and line endings are trimmed. The line must
    /// start with `$` or `!` and end with a `*HH` checksum; anything else
    /// yields the corresponding [`ParseError`].
    pub fn parse(line: &'a str) -> Result<Self, ParseError> {
        let body = line
            .trim()
            .strip_prefix('$')
            .or_else(|| line.trim().strip_prefix('!'))
            .ok_or(ParseError::NoDelimiter)?;

        let (data, checksum_hex) = body.split_once('*').ok_or(ParseError::NoChecksum)?;
        let expected = checksum_hex
            .get(..2)
            .and_then(|hex| u8::from_str_radix(hex, 16).ok())
            .ok_or(ParseError::NoChecksum)?;
        let actual = checksum(data.as_bytes());
        if actual != expected {
            return Err(ParseError::BadChecksum { expected, actual });
        }

        let (address, fields) = data.split_once(',').unwrap_or((data, ""));
        if address.is_empty() {
            return Err(ParseError::Empty);
        }
        Ok(Self { address, fields })
    }

    /// The address field: talker+formatter (`GPGGA`) or proprietary tag
    /// (`PFLAA`, `PGRMZ`).
    pub fn address(&self) -> &'a str {
        self.address
    }

    /// A cursor over the comma-separated fields following the address.
    pub fn fields(&self) -> Fields<'a> {
        Fields::new(self.fields)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_matches_known_sentence() {
        // Body of `$GPGGA,134749.60,...*63` from the flight_1 corpus.
        let data = "GPGGA,134749.60,4857.88170,N,00705.83929,E,2,25,1.00,1452.0,M,47.2,M,,";
        assert_eq!(checksum(data.as_bytes()), 0x63);
    }

    #[test]
    fn frames_address_and_fields() {
        let sentence = Sentence::parse("$PGRMZ,4395,f,3*20").unwrap();
        assert_eq!(sentence.address(), "PGRMZ");
        assert_eq!(sentence.fields().collect::<Vec<_>>(), ["4395", "f", "3"]);
    }

    #[test]
    fn accepts_bang_delimiter_and_trims() {
        // The Cambridge-style `!` delimiter and trailing CRLF are accepted.
        let sentence = Sentence::parse("!PFOO,1,2*44\r\n");
        assert!(matches!(
            sentence,
            Err(ParseError::BadChecksum { .. }) | Ok(_)
        ));
        // A crafted line with a correct checksum frames cleanly.
        let body = "PFOO,1,2";
        let line = format!("!{body}*{:02X}\r\n", checksum(body.as_bytes()));
        let sentence = Sentence::parse(&line).unwrap();
        assert_eq!(sentence.address(), "PFOO");
    }

    #[test]
    fn rejects_malformed_lines() {
        assert_eq!(Sentence::parse("GPGGA,1*00"), Err(ParseError::NoDelimiter));
        assert_eq!(Sentence::parse("$GPGGA,1"), Err(ParseError::NoChecksum));
        assert_eq!(Sentence::parse("$GPGGA,1*ZZ"), Err(ParseError::NoChecksum));
        assert_eq!(
            Sentence::parse("$GPGGA,1*00"),
            Err(ParseError::BadChecksum {
                expected: 0x00,
                actual: checksum(b"GPGGA,1"),
            })
        );
    }
}
