use crate::field::FieldsIter;
use updraft_geo::LatLon;
use updraft_units::Length;

/// `$PLXVTARG`: the navigation target.
#[derive(Clone, Debug, PartialEq)]
pub struct Plxvtarg {
    /// Target waypoint name. Non-UTF-8 bytes are replaced with the
    /// Unicode replacement character.
    pub name: Option<Box<str>>,
    /// Target position.
    pub position: Option<LatLon>,
    /// Target elevation.
    pub elevation: Option<Length>,
}

impl Plxvtarg {
    pub fn parse(mut fields: FieldsIter<'_>) -> Self {
        Self {
            name: fields.text(),
            position: fields.lat_lon(),
            elevation: fields.f64().map(Length::from_meters),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use claims::{assert_none, assert_some, assert_some_eq};

    #[test]
    fn parses_a_target() {
        let plxvtarg = Plxvtarg::parse(FieldsIter::new(b"KOLN,4628.80,N,01541.167,E,268.0"));
        assert_some_eq!(plxvtarg.name, "KOLN".into());
        let position = assert_some!(plxvtarg.position);
        assert_abs_diff_eq!(
            position,
            LatLon::from_degrees(46.48, 15.686_116_666),
            epsilon = 1e-9
        );
        assert_some_eq!(plxvtarg.elevation, Length::from_meters(268.0));
    }

    #[test]
    fn a_target_without_a_position_still_decodes() {
        let plxvtarg = Plxvtarg::parse(FieldsIter::new(b"KOLN,,,,,268.0"));
        assert_some_eq!(plxvtarg.name, "KOLN".into());
        assert_none!(plxvtarg.position);
        assert_some_eq!(plxvtarg.elevation, Length::from_meters(268.0));
    }
}
