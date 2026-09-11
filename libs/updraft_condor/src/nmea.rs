//! The Condor NMEA stream: passthrough of valid sentences and the
//! `$PGRMZ` pressure altitude derived from `$LXWP0`.

use std::sync::{Arc, Mutex};
use std::time::Instant;
use updraft_geo::LatLon;
use updraft_nmea::{GgaFixQuality, Message, Pgrmz, PgrmzFixDimension, RmcStatus, Step, parse};
use updraft_units::Length;

/// The latest own position from the Condor GPS sentences.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OwnFix {
    pub position: LatLon,
    /// The MSL altitude from the last `$GPGGA`.
    pub altitude: Option<Length>,
    pub at: Instant,
}

pub type SharedOwnFix = Arc<Mutex<Option<OwnFix>>>;

/// Pulls every complete sentence off the front of `buffer` and returns the
/// output bytes for them. Incomplete trailing bytes stay in the buffer.
/// GPS sentences received at `now` update `own`.
pub fn process(buffer: &mut Vec<u8>, own: &mut Option<OwnFix>, now: Instant) -> Vec<u8> {
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
                update_own_fix(&message, own, now);
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

fn update_own_fix(message: &Message, own: &mut Option<OwnFix>, now: Instant) {
    match message {
        Message::Gga(gga) if gga.fix_quality != GgaFixQuality::Invalid => {
            if let Some(position) = gga.position {
                *own = Some(OwnFix {
                    position,
                    altitude: gga.altitude,
                    at: now,
                });
            }
        }
        Message::Rmc(rmc) if rmc.status == RmcStatus::Active => {
            if let Some(position) = rmc.position {
                *own = Some(OwnFix {
                    position,
                    altitude: own.and_then(|fix| fix.altitude),
                    at: now,
                });
            }
        }
        _ => {}
    }
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
    use claims::{assert_lt, assert_none, assert_some, assert_some_eq};
    use std::time::Duration;

    const CONDOR: &[u8] =
        b"$GPGGA,120009.136,4621.5849,N,01410.2437,E,1,12,10,504.7,M,,,,,0000*00\r\n\
        $GPRMC,120009.136,A,4621.5849,N,01410.2437,E,0.00,134.00,,,,*20\r\n\
        $LXWP0,Y,0.1,504.7,0.00,,,,,,134,351,19.5*71\r\n";

    fn first_line_end() -> usize {
        CONDOR.iter().position(|byte| *byte == b'\n').unwrap() + 1
    }

    #[test]
    fn passes_sentences_through_and_adds_pressure_altitude() {
        let mut buffer = CONDOR.to_vec();
        let mut own = None;
        let now = Instant::now();
        let output = process(&mut buffer, &mut own, now);
        insta::assert_snapshot!(String::from_utf8(output).unwrap());
        assert!(buffer.is_empty());

        let fix = assert_some!(own);
        let latitude = fix.position.latitude().as_degrees() - (46.0 + 21.5849 / 60.0);
        let longitude = fix.position.longitude().as_degrees() - (14.0 + 10.2437 / 60.0);
        assert_lt!(latitude.abs(), 1e-9);
        assert_lt!(longitude.abs(), 1e-9);
        assert_some_eq!(fix.altitude, Length::from_meters(504.7));
        assert_eq!(fix.at, now);
    }

    #[test]
    fn rmc_alone_keeps_the_last_gga_altitude() {
        let (gga, rest) = CONDOR.split_at(first_line_end());
        let mut own = None;
        let first = Instant::now();
        process(&mut gga.to_vec(), &mut own, first);
        let second = first + Duration::from_secs(1);
        process(&mut rest.to_vec(), &mut own, second);

        let fix = assert_some!(own);
        assert_some_eq!(fix.altitude, Length::from_meters(504.7));
        assert_eq!(fix.at, second);
    }

    #[test]
    fn keeps_an_incomplete_sentence_for_the_next_read() {
        let first_line = first_line_end();
        let (head, tail) = CONDOR.split_at(first_line + 20);
        let mut buffer = head.to_vec();
        let mut own = None;
        let first = process(&mut buffer, &mut own, Instant::now());
        assert_eq!(first, &CONDOR[..first_line]);
        assert_eq!(buffer, &CONDOR[first_line..first_line + 20]);
        buffer.extend_from_slice(tail);
        let second = process(&mut buffer, &mut own, Instant::now());
        assert!(buffer.is_empty());
        assert!(second.starts_with(b"$GPRMC"));
    }

    #[test]
    fn drops_lines_with_a_bad_checksum() {
        let mut buffer =
            b"$GPRMC,120009.136,A,4621.5849,N,01410.2437,E,0.00,134.00,,,,*FF\r\nnoise\r\n"
                .to_vec();
        let mut own = None;
        assert!(process(&mut buffer, &mut own, Instant::now()).is_empty());
        assert!(buffer.is_empty());
        assert_none!(own);
    }
}
