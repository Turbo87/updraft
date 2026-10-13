//! Fast flat-earth approximations of geodesic measurements around one
//! latitude.
//!
//! This is a port of the JavaScript [cheap-ruler] library, version
//! 4.1.1. The ruler uses the WGS84 ellipsoid model of upstream.
//!
//! The port differs from upstream in these ways:
//!
//! - Inputs and outputs use [`LatLon`], [`Angle`], and [`Length`]. There is
//!   no units parameter.
//! - [`CheapRuler::bearing`] returns the compass range `[0°, 360°)`, the
//!   same as [`LatLon::bearing`].
//!
//! [cheap-ruler]: https://github.com/mapbox/cheap-ruler

use crate::LatLon;
use std::f64::consts::TAU;
use updraft_units::{Angle, Length};

// Values that define the WGS84 ellipsoid.
const EQUATORIAL_RADIUS_METERS: f64 = 6_378_137.;
const FLATTENING: f64 = 1. / 298.257_223_563;
const ECCENTRICITY_SQUARED: f64 = FLATTENING * (2. - FLATTENING);

/// Fast approximations of distances, bearings, and destination points
/// near a reference latitude.
///
/// The ruler treats the area around its latitude as flat. Create the
/// ruler at the mean latitude of the points that it measures. The error
/// increases with the distance between the points and very quickly with
/// the distance from the reference latitude. With a ruler at the mean
/// latitude, the distance error stays below 0.1% up to 500 km at
/// latitudes up to 60°, and up to 300 km at 70°. A ruler at the latitude
/// of one of the points has an error of approximately 1% at 300 km and
/// 50°.
///
/// Use [`LatLon::distance`] and the other geodesic methods for scored
/// distances.
///
/// Inputs are not validated. Results for NaN coordinates and for
/// latitudes outside ±90° are unspecified. Near the poles the east-west
/// scale goes to zero and the results become unusable.
#[derive(Clone, Copy, Debug)]
pub struct CheapRuler {
    /// Meters per radian of longitude, from the normal radius of
    /// curvature.
    kx: f64,
    /// Meters per radian of latitude, from the meridional radius of
    /// curvature.
    ky: f64,
}

impl CheapRuler {
    /// Creates a ruler for measurements near `latitude`.
    pub fn new(latitude: Angle) -> Self {
        // Curvature formulas from
        // https://en.wikipedia.org/wiki/Earth_radius#Meridional
        let cos_latitude = latitude.cos();
        let w2 = 1. / (1. - ECCENTRICITY_SQUARED * (1. - cos_latitude * cos_latitude));
        let w = w2.sqrt();

        Self {
            kx: EQUATORIAL_RADIUS_METERS * w * cos_latitude,
            ky: EQUATORIAL_RADIUS_METERS * w * w2 * (1. - ECCENTRICITY_SQUARED),
        }
    }

    /// The distance from `a` to `b`.
    pub fn distance(&self, a: LatLon, b: LatLon) -> Length {
        let (dx, dy) = self.delta(a, b);
        Length::from_meters((dx * dx + dy * dy).sqrt())
    }

    /// The bearing from `a` to `b`, in the compass range `[0°, 360°)`.
    ///
    /// The result approximates the geodesic bearing at the middle of the
    /// path, not the initial bearing at `a`. The two differ by half of the
    /// meridian convergence, for example approximately 0.4° for 100 km
    /// east at 50°.
    pub fn bearing(&self, a: LatLon, b: LatLon) -> Angle {
        let (dx, dy) = self.delta(a, b);
        Angle::from_radians(dx.atan2(dy)).normalized()
    }

    /// The point at `distance` from `point` in the direction `bearing`.
    pub fn destination(&self, point: LatLon, bearing: Angle, distance: Length) -> LatLon {
        let (sin, cos) = bearing.sin_cos();
        self.offset(point, distance * sin, distance * cos)
    }

    /// The point at the `east` and `north` offsets from `point`.
    ///
    /// The longitude of the result is not normalized.
    pub fn offset(&self, point: LatLon, east: Length, north: Length) -> LatLon {
        LatLon::new(
            point.latitude() + Angle::from_radians(north.as_meters() / self.ky),
            point.longitude() + Angle::from_radians(east.as_meters() / self.kx),
        )
    }

    /// The east and north offsets in meters from `a` to `b`.
    fn delta(&self, a: LatLon, b: LatLon) -> (f64, f64) {
        let dx = wrap((b.longitude() - a.longitude()).as_radians()) * self.kx;
        let dy = (b.latitude() - a.latitude()).as_radians() * self.ky;
        (dx, dy)
    }
}

/// Normalizes a longitude difference in radians into `[-π, π]`.
fn wrap(radians: f64) -> f64 {
    // `floor(x + 0.5)` matches the rounding of JavaScript `Math.round`.
    radians - (radians / TAU + 0.5).floor() * TAU
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::{assert_abs_diff_eq, assert_relative_eq};
    use claims::assert_le;

    const BEARINGS: [f64; 8] = [0., 45., 90., 135., 180., 225., 270., 315.];

    /// A ruler at the mean latitude of `a` and `b`.
    fn midpoint_ruler(a: LatLon, b: LatLon) -> CheapRuler {
        CheapRuler::new((a.latitude() + b.latitude()) / 2.)
    }

    #[test]
    fn distance_matches_geodesic() {
        let cases = [
            (0., 500.),
            (30., 500.),
            (45., 500.),
            (60., 500.),
            (70., 300.),
        ];
        for (latitude, max_kilometers) in cases {
            for sign in [-1., 1.] {
                let origin = LatLon::from_degrees(sign * latitude, 10.);
                for kilometers in [1., 10., 100., max_kilometers] {
                    for bearing in BEARINGS {
                        let target = origin.destination(
                            Angle::from_degrees(bearing),
                            Length::from_kilometers(kilometers),
                        );
                        let ruler = midpoint_ruler(origin, target);
                        let expected = origin.distance(target);
                        let error = ((ruler.distance(origin, target) - expected) / expected).abs();
                        assert_le!(error, 0.001, "{latitude}° {kilometers} km {bearing}°");
                    }
                }
            }
        }
    }

    #[test]
    fn bearing_matches_geodesic_at_middle_of_path() {
        let origin = LatLon::from_degrees(50., 10.);
        let half = Length::from_kilometers(50.);
        for bearing in BEARINGS {
            let bearing = Angle::from_degrees(bearing);
            let middle = origin.destination(bearing, half);
            let target = origin.destination(bearing, half * 2.);
            assert_abs_diff_eq!(
                midpoint_ruler(origin, target).bearing(origin, target),
                middle.bearing(target),
                epsilon = 0.01_f64.to_radians()
            );
        }
    }

    #[test]
    fn bearing_uses_compass_range() {
        let ruler = CheapRuler::new(Angle::ZERO);
        let origin = LatLon::from_degrees(0., 0.);
        let west = LatLon::from_degrees(0., -0.1);
        assert_relative_eq!(ruler.bearing(origin, west), Angle::from_degrees(270.));
    }

    #[test]
    fn destination_matches_geodesic() {
        // The latitude of the destination is not known in advance, so the
        // ruler uses the latitude of the start point.
        let origin = LatLon::from_degrees(50., 10.);
        let ruler = CheapRuler::new(origin.latitude());
        let distance = Length::from_kilometers(1.);
        for bearing in BEARINGS {
            let bearing = Angle::from_degrees(bearing);
            let expected = origin.destination(bearing, distance);
            let actual = ruler.destination(origin, bearing, distance);
            assert_le!(expected.distance(actual), Length::from_meters(0.2));
        }
    }

    #[test]
    fn offset_round_trips_through_distance() {
        let ruler = CheapRuler::new(Angle::from_degrees(50.));
        let origin = LatLon::from_degrees(50., 10.);
        let target = ruler.offset(origin, Length::from_meters(300.), Length::from_meters(400.));
        assert_relative_eq!(
            ruler.distance(origin, target),
            Length::from_meters(500.),
            max_relative = 1e-12
        );
    }

    #[test]
    fn distance_across_antimeridian() {
        // Upstream test case.
        let ruler = CheapRuler::new(Angle::from_degrees(32.8351));
        let a = LatLon::from_degrees(32.7, 179.9);
        let b = LatLon::from_degrees(32.9, -179.9);
        let expected = a.distance(b);
        let error = ((ruler.distance(a, b) - expected) / expected).abs();
        assert_le!(error, 0.001);
        assert_abs_diff_eq!(
            ruler.bearing(a, b),
            a.bearing(b),
            epsilon = 0.1_f64.to_radians()
        );
    }
}
