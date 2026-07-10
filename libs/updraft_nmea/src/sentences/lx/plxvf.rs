use crate::field::f64_field;
use updraft_units::{Length, Speed};

/// `$PLXVF`: the vario "fast data" sentence from LXNAV varios (V7,
/// S-series), sent at a high rate (typically 5-20 Hz).
///
/// Carries acceleration, total-energy vario, indicated airspeed, and
/// pressure altitude.
#[derive(Clone, Debug, PartialEq)]
pub struct Plxvf {
    /// Device timestamp in seconds.
    pub time: Option<f64>,
    /// Acceleration along the device X axis, in g.
    pub acceleration_x: Option<f64>,
    /// Acceleration along the device Y axis, in g.
    pub acceleration_y: Option<f64>,
    /// Acceleration along the device Z axis, in g.
    pub acceleration_z: Option<f64>,
    /// Total-energy vario.
    pub vario: Option<Speed>,
    /// Indicated airspeed.
    pub indicated_airspeed: Option<Speed>,
    /// Barometric altitude.
    pub pressure_altitude: Option<Length>,
}

impl Plxvf {
    pub fn parse(fields: &[&[u8]]) -> Self {
        Self {
            time: f64_field(fields, 0),
            acceleration_x: f64_field(fields, 1),
            acceleration_y: f64_field(fields, 2),
            acceleration_z: f64_field(fields, 3),
            vario: f64_field(fields, 4).map(Speed::from_meters_per_second),
            indicated_airspeed: f64_field(fields, 5).map(Speed::from_meters_per_second),
            pressure_altitude: f64_field(fields, 6).map(Length::from_meters),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some_eq};

    #[test]
    fn parses_fast_vario_data() {
        let fields: [&[u8]; 8] = [
            b"", b"1.00", b"0.87", b"-0.12", b"-0.25", b"90.2", b"244.3", b"",
        ];
        let plxvf = Plxvf::parse(&fields);
        assert_none!(plxvf.time);
        assert_some_eq!(plxvf.acceleration_x, 1.00);
        assert_some_eq!(plxvf.acceleration_y, 0.87);
        assert_some_eq!(plxvf.acceleration_z, -0.12);
        assert_some_eq!(plxvf.vario, Speed::from_meters_per_second(-0.25));
        assert_some_eq!(
            plxvf.indicated_airspeed,
            Speed::from_meters_per_second(90.2)
        );
        assert_some_eq!(plxvf.pressure_altitude, Length::from_meters(244.3));
    }

    #[test]
    fn omitted_trailing_fields_read_as_absent() {
        // A short form: an acceleration triple only, no vario, airspeed, or
        // altitude.
        let fields: [&[u8]; 4] = [b"", b"1.00", b"0.87", b"-0.12"];
        let plxvf = Plxvf::parse(&fields);
        assert_some_eq!(plxvf.acceleration_x, 1.00);
        assert_none!(plxvf.vario);
        assert_none!(plxvf.indicated_airspeed);
        assert_none!(plxvf.pressure_altitude);
    }
}
