//! Framing layer: pulls one checksum-validated sentence off a byte buffer
//! at a time, resynchronizing past noise.

use crate::Message;
use crate::sentences;

/// Bytes a single sentence may span before the parser stops looking for a
/// terminator and resynchronizes, bounding buffer growth on a garbage
/// stream. Comfortably above the longest real proprietary sentence.
const MAX_SENTENCE_LEN: usize = 1024;

/// The outcome of a single [`parse()`] step.
#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    /// A complete sentence was decoded. The input has advanced past it.
    Frame(Message),
    /// The bytes at the front of the input were not a valid frame. They
    /// have been discarded up to the next resynchronisation boundary.
    Rejected(RejectReason),
    /// The input does not yet hold a complete sentence. Feed more bytes
    /// and call again. Any leading blank lines have been consumed.
    Incomplete,
}

/// Why a [`Step::Rejected`] region could not be framed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RejectReason {
    /// A sentence carried a `*` checksum field that did not match its
    /// contents, or whose two hex digits were malformed. Sentences without a
    /// checksum are accepted, so a missing checksum never lands here.
    BadChecksum,
    /// Bytes that were not part of any sentence: leading noise, trailing
    /// bytes after a sentence's checksum, or a start marker not followed by
    /// a delimiter within [`MAX_SENTENCE_LEN`].
    Junk,
}

/// Pulls the next sentence off the front of `input`, advancing it past
/// whatever was consumed.
///
/// A sentence ends at its `*HH` checksum when it has one, so a checksummed
/// sentence self-delimits even without a trailing newline. A checksum-less
/// sentence ends at the next newline. Returns [`Step::Incomplete`] when the
/// buffer does not yet hold a full sentence. On every other outcome the
/// input advances by at least one byte, so a caller can loop until
/// `Incomplete` and then read more bytes.
pub fn parse(input: &mut &[u8]) -> Step {
    // Skip line breaks at the start of the buffer.
    let start = input.iter().position(|&byte| !is_newline(byte));
    let start = start.unwrap_or(input.len());
    *input = &input[start..];

    let Some(&first) = input.first() else {
        return Step::Incomplete;
    };

    if first != b'$' && first != b'!' {
        // Noise before the next start marker, or trailing bytes after a
        // preceding sentence's checksum: discard up to the next boundary so
        // the following call can lock onto a real sentence.
        let boundary = input
            .iter()
            .position(|&byte| is_start_marker(byte) || is_newline(byte))
            .unwrap_or(input.len());

        *input = &input[boundary..];
        return Step::Rejected(RejectReason::Junk);
    }

    // A checksum (if any) delimits the sentence, otherwise the newline does.
    // Only look one sentence ahead: a start marker with no terminator within
    // `MAX_SENTENCE_LEN` never began a real sentence, and scanning the whole
    // buffer on every call would be quadratic on a delimiter-free stream. The
    // first `*`/`\r`/`\n` decides which terminator wins.
    let horizon = &input[..input.len().min(MAX_SENTENCE_LEN)];
    let terminator = horizon
        .iter()
        .position(|&byte| byte == b'*' || is_newline(byte));

    match terminator {
        Some(pos) if input[pos] == b'*' => frame_with_checksum(input, pos),

        // No checksum before the newline: a checksum-less sentence.
        Some(pos) => {
            let frame = Step::Frame(sentences::parse_body(&input[1..pos]));
            *input = &input[pos..];
            frame
        }

        // Neither delimiter yet: the marker has no terminator in view.
        None => stalled(input),
    }
}

/// Frames a sentence whose body starts at `input[1]` and whose checksum field
/// begins at the `*` found at `star`.
fn frame_with_checksum(input: &mut &[u8], star: usize) -> Step {
    let checksum = input
        .get(star + 1..star + 3)
        .and_then(|hex| Some(hex_digit(hex[0])? << 4 | hex_digit(hex[1])?));

    match checksum {
        Some(checksum) => {
            let body = &input[1..star];
            let frame = if checksum == xor(body) {
                Step::Frame(sentences::parse_body(body))
            } else {
                Step::Rejected(RejectReason::BadChecksum)
            };
            *input = &input[star + 3..];
            frame
        }

        // A `*` not followed by two hex digits is a broken checksum field, so
        // reject the whole line once its end is in view.
        None => {
            let horizon = &input[..input.len().min(MAX_SENTENCE_LEN)];
            match horizon
                .iter()
                .position(|&byte| byte == b'\r' || byte == b'\n')
            {
                Some(newline) => {
                    *input = &input[newline..];
                    Step::Rejected(RejectReason::BadChecksum)
                }
                None => stalled(input),
            }
        }
    }
}

/// A start marker at `input[0]` has no terminator in view yet: wait for more
/// bytes, unless the run is already longer than a real sentence, in which case
/// resynchronize one byte past the marker rather than growing the buffer.
fn stalled(input: &mut &[u8]) -> Step {
    if input.len() > MAX_SENTENCE_LEN {
        *input = &input[1..];
        Step::Rejected(RejectReason::Junk)
    } else {
        Step::Incomplete
    }
}

fn is_start_marker(byte: u8) -> bool {
    byte == b'$' || byte == b'!'
}

fn is_newline(byte: u8) -> bool {
    byte == b'\r' || byte == b'\n'
}

fn xor(body: &[u8]) -> u8 {
    body.iter().fold(0, |checksum, &byte| checksum ^ byte)
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_some_eq;

    /// Parses a single complete sentence and returns the step.
    fn parse_one(mut sentence: &[u8]) -> Step {
        parse(&mut sentence)
    }

    #[test]
    fn parses_gga() {
        let s = b"$GPGGA,134749.60,4857.88170,N,00705.83929,E,2,25,1.00,1452.0,M,47.2,M,,*63\r\n";
        insta::assert_debug_snapshot!(parse_one(s));
    }

    #[test]
    fn parses_rmc() {
        let s = b"$GPRMC,134749.60,A,4857.88170,N,00705.83929,E,35.9,270.6,281224,,,D*61\r\n";
        insta::assert_debug_snapshot!(parse_one(s));
    }

    #[test]
    fn parses_gsa() {
        let s = b"$GPGSA,A,3,,,,,,,,,,,,,1.0,1.0,1.0*33\r\n";
        insta::assert_debug_snapshot!(parse_one(s));
    }

    #[test]
    fn parses_pgrmz() {
        let s = b"$PGRMZ,4395,f,3*20\r\n";
        insta::assert_debug_snapshot!(parse_one(s));
    }

    #[test]
    fn keeps_unrecognised_sentence_as_unknown() {
        let s = b"$PXABC,11,1,2,1,0,,0,,,*7A\r\n";
        insta::assert_debug_snapshot!(parse_one(s));
    }

    #[test]
    fn decodes_a_sentence_with_a_non_utf8_field() {
        // A stray Latin-1 byte (0xB0, `°`) in a field does not fail the
        // sentence: the GGA still decodes.
        let s = b"$GPGGA,134749.60,4857.88170,N,00705.83929,E,2,25,1.00,1452.0,M,47.2,M,,\xB0\r\n";
        insta::assert_debug_snapshot!(parse_one(s));
    }

    #[test]
    fn rejects_only_an_invalid_checksum() {
        // A wrong checksum is rejected...
        let s = b"$GPGSA,A,3,,,,,,,,,,,,,1.0,1.0,1.0*00\r\n";
        assert_eq!(parse_one(s), Step::Rejected(RejectReason::BadChecksum));

        // ...but a missing checksum is accepted.
        let s = b"$GPGSA,A,3,,,,,,,,,,,,,1.0,1.0,1.0\r\n";
        assert!(matches!(parse_one(s), Step::Frame(Message::Gsa(_))));
    }

    #[test]
    fn salvages_the_sentence_and_rejects_trailing_bytes() {
        // `*20` is a valid checksum, so the sentence frames at the
        // checksum. The trailing `XYZ` before the newline is separate junk.

        let mut input = b"$PGRMZ,4395,f,3*20XYZ\r\n".as_slice();
        assert!(matches!(parse(&mut input), Step::Frame(Message::Pgrmz(_))));
        assert_eq!(parse(&mut input), Step::Rejected(RejectReason::Junk));
    }

    #[test]
    fn partial_sentence_needs_more_bytes() {
        let mut input = b"$GPGGA,134749.60".as_slice();
        assert_eq!(parse(&mut input), Step::Incomplete);

        // The partial sentence is left in place for the next read.
        assert_eq!(input, b"$GPGGA,134749.60");
    }

    #[test]
    fn resynchronises_past_leading_junk() {
        let mut input = b"garbage$GPGGA,1*00\r\n".as_slice();
        assert_eq!(parse(&mut input), Step::Rejected(RejectReason::Junk));

        // The junk is gone and the marker is next.
        assert_some_eq!(input.first(), &b'$');
    }

    #[test]
    fn rejects_a_malformed_checksum_field() {
        // `*ZZ` is not two hex digits: the line is rejected once its end is
        // in view.

        let s = b"$GPGGA,1*ZZ\r\n";
        assert_eq!(parse_one(s), Step::Rejected(RejectReason::BadChecksum));
    }

    #[test]
    fn waits_for_a_split_checksum() {
        // Only one checksum digit has arrived, and there is no newline yet:
        // the parser must wait rather than reject the half-seen checksum.
        let mut input = b"$GPGGA,1*6".as_slice();
        assert_eq!(parse(&mut input), Step::Incomplete);
    }

    #[test]
    fn abandons_an_overlong_run_without_a_terminator() {
        // A marker followed by a garbage run longer than a sentence resyncs
        // one byte at a time rather than growing the buffer forever.
        let buffer = vec![b'$'; MAX_SENTENCE_LEN + 1];
        let mut input = buffer.as_slice();
        assert_eq!(parse(&mut input), Step::Rejected(RejectReason::Junk));
        assert_eq!(input.len(), MAX_SENTENCE_LEN);
    }

    #[test]
    fn decodes_a_no_fix_multi_constellation_gga() {
        // A cold-start GNSS sentence: combined talker, quality 0, no position.
        let s = b"$GNGGA,,,,,,0,,,,,,,,\r\n";
        insta::assert_debug_snapshot!(parse_one(s));
    }

    #[test]
    fn decodes_a_void_rmc() {
        // A navigation-receiver-warning RMC before a fix is acquired.
        let s = b"$GPRMC,,V,,,,,,,,,,N\r\n";
        insta::assert_debug_snapshot!(parse_one(s));
    }

    #[test]
    fn frames_a_bang_marker_sentence() {
        // A `!`-led sentence (AIS) frames like any other. An undecoded type
        // becomes `Unknown` rather than being discarded.
        let s = b"!AIVDM,1,1,,A,177KQ\r\n";
        assert!(matches!(parse_one(s), Step::Frame(Message::Unknown(_))));
    }
}
