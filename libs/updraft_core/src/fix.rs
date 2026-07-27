use crate::topic::LatLon;

/// A position report from the device's own GNSS receiver.
///
/// Distinct from a fix decoded out of NMEA: it arrives already structured,
/// from a source the operating system vouches for, so it never passes
/// through [`crate::Decoder`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fix {
    pub position: LatLon,
    pub altitude_ellipsoid_meters: Option<f64>,
    pub track_degrees: Option<f64>,
    pub ground_speed_meters_per_second: Option<f64>,
}
