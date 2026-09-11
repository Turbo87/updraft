//! The Condor NMEA stream: passthrough of valid sentences and the
//! `$PGRMZ` pressure altitude derived from `$LXWP0`.

use updraft_nmea::{Message, Pgrmz, PgrmzFixDimension, Step, parse};
use updraft_units::Length;

/// Pulls every complete sentence off the front of `buffer` and returns the
/// output bytes for them. Incomplete trailing bytes stay in the buffer.
pub fn process(buffer: &mut Vec<u8>) -> Vec<u8> {
    let mut output = Vec::new();
    let mut input = buffer.as_slice();
    loop {
        let before = input;
        match parse(&mut input) {
            Step::Frame(message) => {
                let consumed = before.len() - input.len();
                let raw = trim_newlines(&before[..consumed]);
                output.extend_from_slice(raw);
                output.extend_from_slice(b"\r\n");
                if let Some(pgrmz) = derived_pgrmz(&message) {
                    output.extend_from_slice(&pgrmz);
                }
            }
            Step::Rejected(reason) => {
                tracing::debug!(?reason, "dropped an invalid Condor NMEA line");
            }
            Step::Incomplete => break,
        }
    }
    let remaining = input.len();
    buffer.drain(..buffer.len() - remaining);
    output
}

/// Condor sends the altimeter reading in the `$LXWP0` baro altitude field.
/// Updraft reads pressure altitude only from `$PGRMZ`, in whole feet.
fn derived_pgrmz(message: &Message) -> Option<Vec<u8>> {
    let Message::Lxwp0(lxwp0) = message else {
        return None;
    };
    let altitude = lxwp0.pressure_altitude?;
    let pgrmz = Pgrmz {
        altitude: Some(Length::from_feet(altitude.as_feet().round())),
        fix_dimension: PgrmzFixDimension::ThreeDimensional,
    };
    Vec::<u8>::try_from(&pgrmz).ok()
}

fn trim_newlines(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|byte| !matches!(byte, b'\r' | b'\n'))
        .unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|byte| !matches!(byte, b'\r' | b'\n'))
        .map_or(start, |end| end + 1);
    &bytes[start..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONDOR: &[u8] =
        b"$GPGGA,120009.136,4621.5849,N,01410.2437,E,1,12,10,504.7,M,,,,,0000*00\r\n\
        $GPRMC,120009.136,A,4621.5849,N,01410.2437,E,0.00,134.00,,,,*20\r\n\
        $LXWP0,Y,0.1,504.7,0.00,,,,,,134,351,19.5*71\r\n";

    #[test]
    fn passes_sentences_through_and_adds_pressure_altitude() {
        let mut buffer = CONDOR.to_vec();
        let output = process(&mut buffer);
        insta::assert_snapshot!(String::from_utf8(output).unwrap());
        assert!(buffer.is_empty());
    }

    #[test]
    fn keeps_an_incomplete_sentence_for_the_next_read() {
        let first_line = CONDOR.iter().position(|byte| *byte == b'\n').unwrap() + 1;
        let (head, tail) = CONDOR.split_at(first_line + 20);
        let mut buffer = head.to_vec();
        let first = process(&mut buffer);
        assert_eq!(first, &CONDOR[..first_line]);
        assert_eq!(buffer, &CONDOR[first_line..first_line + 20]);
        buffer.extend_from_slice(tail);
        let second = process(&mut buffer);
        assert!(buffer.is_empty());
        assert!(second.starts_with(b"$GPRMC"));
    }

    #[test]
    fn drops_lines_with_a_bad_checksum() {
        let mut buffer =
            b"$GPRMC,120009.136,A,4621.5849,N,01410.2437,E,0.00,134.00,,,,*FF\r\nnoise\r\n"
                .to_vec();
        assert!(process(&mut buffer).is_empty());
        assert!(buffer.is_empty());
    }
}
