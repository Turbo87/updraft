use updraft_geo::LatLon;
use updraft_units::{Angle, EllipsoidAltitude, Speed};

/// A position report from the device's own GNSS receiver.
///
/// Distinct from a fix decoded out of NMEA: it arrives already structured,
/// from a source the operating system vouches for, so it never passes
/// through [`crate::Decoder`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fix {
    pub position: LatLon,
    pub altitude_ellipsoid: Option<EllipsoidAltitude>,
    pub track: Option<Angle>,
    pub ground_speed: Option<Speed>,
}
