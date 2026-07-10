use crate::field::{f64_field, field};
use updraft_units::{Angle, Length, Speed};

/// `$LXWP0`: the main flight-data sentence, sent about once per second.
#[derive(Clone, Debug, PartialEq)]
pub struct Lxwp0 {
    /// Whether a flight is currently being recorded (`Y`/`N`).
    pub logger_running: Option<bool>,
    /// True airspeed.
    pub true_airspeed: Option<Speed>,
    /// Barometric altitude above the 1013.25 hPa standard atmosphere.
    pub pressure_altitude: Option<Length>,
    /// Up to six total-energy vario samples, in the order sent.
    pub vario_samples: Vec<Speed>,
    /// Heading of the aircraft.
    pub heading: Option<Angle>,
    /// Direction the wind is coming from. Condor sends this rotated by 180°.
    pub wind_direction: Option<Angle>,
    /// Wind speed.
    pub wind_speed: Option<Speed>,
}

impl Lxwp0 {
    pub fn parse(fields: &[&[u8]]) -> Self {
        Self {
            logger_running: yes_no(field(fields, 0)),
            true_airspeed: f64_field(fields, 1).map(Speed::from_kilometers_per_hour),
            pressure_altitude: f64_field(fields, 2).map(Length::from_meters),
            // Fields 3-8 are the six vario samples. Present ones are kept
            // in order, empty samples are dropped. Devices fill them
            // left-to-right (all six, the first only, or none), so no
            // interior gap arises in practice.
            vario_samples: (3..9)
                .filter_map(|index| f64_field(fields, index))
                .map(Speed::from_meters_per_second)
                .collect(),
            heading: f64_field(fields, 9).map(Angle::from_degrees),
            wind_direction: f64_field(fields, 10).map(Angle::from_degrees),
            wind_speed: f64_field(fields, 11).map(Speed::from_kilometers_per_hour),
        }
    }
}

/// An LXNAV `Y`/`N` status field. Any other value, including an absent
/// field, reads as absent.
fn yes_no(value: Option<&[u8]>) -> Option<bool> {
    match value {
        Some(b"Y") => Some(true),
        Some(b"N") => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some_eq};

    #[test]
    fn parses_a_full_flight_data_sentence() {
        let fields: [&[u8]; 12] = [
            b"Y", b"222.3", b"1665.5", b"1.71", b"1.71", b"1.71", b"1.71", b"1.71", b"1.71",
            b"239", b"174", b"10.1",
        ];
        let lxwp0 = Lxwp0::parse(&fields);
        assert_some_eq!(lxwp0.logger_running, true);
        assert_some_eq!(lxwp0.true_airspeed, Speed::from_kilometers_per_hour(222.3));
        assert_some_eq!(lxwp0.pressure_altitude, Length::from_meters(1665.5));
        assert_eq!(
            lxwp0.vario_samples,
            vec![Speed::from_meters_per_second(1.71); 6]
        );
        assert_some_eq!(lxwp0.heading, Angle::from_degrees(239.0));
        assert_some_eq!(lxwp0.wind_direction, Angle::from_degrees(174.0));
        assert_some_eq!(lxwp0.wind_speed, Speed::from_kilometers_per_hour(10.1));
    }

    #[test]
    fn keeps_only_the_vario_samples_that_are_present() {
        // A common form: airspeed and altitude, a single vario sample, wind,
        // and no heading.
        let fields: [&[u8]; 12] = [
            b"N", b"", b"1266.5", b"", b"", b"", b"", b"", b"", b"", b"248", b"23.1",
        ];
        let lxwp0 = Lxwp0::parse(&fields);
        assert_some_eq!(lxwp0.logger_running, false);
        assert_none!(lxwp0.true_airspeed);
        assert_some_eq!(lxwp0.pressure_altitude, Length::from_meters(1266.5));
        assert_eq!(lxwp0.vario_samples, Vec::<Speed>::new());
        assert_none!(lxwp0.heading);
        assert_some_eq!(lxwp0.wind_direction, Angle::from_degrees(248.0));
        assert_some_eq!(lxwp0.wind_speed, Speed::from_kilometers_per_hour(23.1));
    }

    #[test]
    fn reads_the_single_leading_vario_sample() {
        let fields: [&[u8]; 12] = [
            b"Y", b"222.3", b"1665.5", b"1.71", b"", b"", b"", b"", b"", b"239", b"174", b"10.1",
        ];
        let lxwp0 = Lxwp0::parse(&fields);
        assert_eq!(
            lxwp0.vario_samples,
            vec![Speed::from_meters_per_second(1.71)]
        );
    }

    #[test]
    fn a_non_yes_no_logger_field_is_absent() {
        assert_none!(yes_no(Some(b"1")));
        assert_none!(yes_no(Some(b"")));
        assert_none!(yes_no(None));
        assert_some_eq!(yes_no(Some(b"Y")), true);
        assert_some_eq!(yes_no(Some(b"N")), false);
    }
}
