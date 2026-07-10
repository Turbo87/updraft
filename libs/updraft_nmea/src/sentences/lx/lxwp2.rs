use crate::field::f64_field;
use updraft_units::Speed;

/// `$LXWP2`: the glide-computer settings: MacCready, ballast, bugs, the
/// active polar, and audio volume.
///
/// On LX systems this is also accepted as input, so a connected device and
/// the instrument can keep these settings in sync.
///
/// The LXNAV polar coefficient normalization (scaling, `v` in km/h/100) is
/// left to the consumer.
#[derive(Clone, Debug, PartialEq)]
pub struct Lxwp2 {
    /// MacCready setting.
    pub mac_cready: Option<Speed>,
    /// Ballast as an overload factor (total mass over reference mass),
    /// nominally 1.0-1.5.
    pub ballast: Option<f64>,
    /// Bugs as a percentage degradation, nominally 0-100.
    /// Some older firmware instead reports a 1.00-1.10 factor here.
    pub bugs: Option<f64>,
    /// Polar coefficient `a`.
    pub polar_a: Option<f64>,
    /// Polar coefficient `b`.
    pub polar_b: Option<f64>,
    /// Polar coefficient `c`.
    pub polar_c: Option<f64>,
    /// Audio volume as a percentage, nominally 0-100.
    pub volume: Option<f64>,
}

impl Lxwp2 {
    pub fn parse(fields: &[&[u8]]) -> Self {
        Self {
            mac_cready: f64_field(fields, 0).map(Speed::from_meters_per_second),
            ballast: f64_field(fields, 1),
            bugs: f64_field(fields, 2),
            polar_a: f64_field(fields, 3),
            polar_b: f64_field(fields, 4),
            polar_c: f64_field(fields, 5),
            volume: f64_field(fields, 6),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some_eq};

    #[test]
    fn parses_settings_with_a_polar_and_volume() {
        let fields: [&[u8]; 7] = [b"1.5", b"1.11", b"13", b"2.96", b"-3.03", b"1.35", b"45"];
        let lxwp2 = Lxwp2::parse(&fields);
        assert_some_eq!(lxwp2.mac_cready, Speed::from_meters_per_second(1.5));
        assert_some_eq!(lxwp2.ballast, 1.11);
        assert_some_eq!(lxwp2.bugs, 13.0);
        assert_some_eq!(lxwp2.polar_a, 2.96);
        assert_some_eq!(lxwp2.polar_b, -3.03);
        assert_some_eq!(lxwp2.polar_c, 1.35);
        assert_some_eq!(lxwp2.volume, 45.0);
    }

    #[test]
    fn keeps_present_fields_when_the_polar_and_volume_are_omitted() {
        // A short form: MacCready, ballast, and bugs only.
        let fields: [&[u8]; 3] = [b"1.7", b"1.1", b"5"];
        let lxwp2 = Lxwp2::parse(&fields);
        assert_some_eq!(lxwp2.mac_cready, Speed::from_meters_per_second(1.7));
        assert_some_eq!(lxwp2.ballast, 1.1);
        assert_some_eq!(lxwp2.bugs, 5.0);
        assert_none!(lxwp2.polar_a);
        assert_none!(lxwp2.volume);
    }
}
