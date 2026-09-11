//! The Condor UDP telemetry: the MacCready value as `$LXWP2`.

use std::time::{Duration, Instant};
use updraft_nmea::Lxwp2;
use updraft_units::Speed;

/// A changed value goes out at most this often.
const CHANGE_INTERVAL: Duration = Duration::from_secs(1);

/// An unchanged value is repeated this often for clients that connect
/// during a flight.
const REPEAT_INTERVAL: Duration = Duration::from_secs(5);

/// The `MC` value of one datagram, in m/s. The datagram is ASCII text with
/// one `key=value` pair per line.
pub fn mac_cready(datagram: &[u8]) -> Option<Speed> {
    let text = std::str::from_utf8(datagram).ok()?;
    text.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        if key.trim() != "MC" {
            return None;
        }
        let value: f64 = value.trim().parse().ok()?;
        value
            .is_finite()
            .then(|| Speed::from_meters_per_second(value))
    })
}

/// Decides when the MacCready value goes out as `$LXWP2`.
#[derive(Default)]
pub struct MacCreadyEmitter {
    last_sent: Option<(Speed, Instant)>,
}

impl MacCreadyEmitter {
    /// The sentence to send for a value received at `now`, if any.
    pub fn offer(&mut self, value: Speed, now: Instant) -> Option<Vec<u8>> {
        let due = match self.last_sent {
            None => true,
            Some((sent_value, sent_at)) => {
                let elapsed = now.saturating_duration_since(sent_at);
                elapsed >= REPEAT_INTERVAL || (sent_value != value && elapsed >= CHANGE_INTERVAL)
            }
        };
        if !due {
            return None;
        }
        let lxwp2 = Lxwp2 {
            mac_cready: Some(value),
            ballast: None,
            bugs: None,
            polar_a: None,
            polar_b: None,
            polar_c: None,
            volume: None,
        };
        let sentence = Vec::<u8>::try_from(&lxwp2).ok()?;
        self.last_sent = Some((value, now));
        Some(sentence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some, assert_some_eq};

    const DATAGRAM: &[u8] = include_bytes!("../../../testdata/condor/udp.txt");

    #[test]
    fn reads_the_mac_cready_value() {
        assert_some_eq!(mac_cready(DATAGRAM), Speed::from_meters_per_second(0.0));
        assert_some_eq!(
            mac_cready(b"time=1\r\nMC=1.5\r\nwater=0\r\n"),
            Speed::from_meters_per_second(1.5)
        );
        assert_none!(mac_cready(b"time=1\nwater=0\n"));
        assert_none!(mac_cready(b"MC=abc\n"));
    }

    #[test]
    fn sends_changes_at_most_once_per_second_and_repeats_every_five() {
        let mut emitter = MacCreadyEmitter::default();
        let start = Instant::now();
        let one = Speed::from_meters_per_second(1.0);
        let two = Speed::from_meters_per_second(2.0);

        let first = assert_some!(emitter.offer(one, start));
        assert_eq!(first, b"$LXWP2,1,,,,,,*3C\r\n");

        // A change within the first second waits.
        assert_none!(emitter.offer(two, start + Duration::from_millis(500)));
        assert_none!(emitter.offer(one, start + Duration::from_millis(900)));

        let second = assert_some!(emitter.offer(two, start + Duration::from_millis(1100)));
        assert_eq!(second, b"$LXWP2,2,,,,,,*3F\r\n");

        // The same value is repeated after five seconds.
        assert_none!(emitter.offer(two, start + Duration::from_millis(6000)));
        assert_some!(emitter.offer(two, start + Duration::from_millis(6100)));
    }
}
