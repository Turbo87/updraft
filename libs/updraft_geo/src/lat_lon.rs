use geographiclib_rs::{DirectGeodesic, Geodesic, InverseGeodesic};
use updraft_units::{Angle, Length};

/// Mean earth radius (IUGG arithmetic mean radius R1) used by the
/// haversine fast path.
const MEAN_EARTH_RADIUS_METERS: f64 = 6_371_008.771_4;

/// A geographic position on the WGS84 ellipsoid.
///
/// Latitude is positive north, longitude positive east. Values are not
/// validated or normalized on construction. The geodesic methods expect
/// latitudes within ±90° (`geographiclib` returns NaN distances and
/// bearings outside that range) while longitudes may be any angle.
///
/// The geodesic methods ([`distance`](Self::distance),
/// [`bearing`](Self::bearing), [`destination`](Self::destination), …)
/// solve on the WGS84 ellipsoid with sub-millimeter accuracy, matching
/// the earth model used for FAI badge/record and OLC/WeGlide scoring
/// distances. [`haversine_distance`](Self::haversine_distance) is a
/// fast spherical approximation (within ~0.5% of the ellipsoid) for
/// latency- or throughput-sensitive uses.
///
/// With the `approx` feature, approximate-equality comparisons operate
/// on the internal **radians** of both angles, so epsilons are radians,
/// not degrees.
///
/// With the `ts` feature, the matching TypeScript type mirrors the serde
/// representation: `latitude`/`longitude` as degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(any(test, feature = "approx"), derive(approx::RelativeEq))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LatLon {
    latitude: Angle,
    longitude: Angle,
}

impl LatLon {
    pub const fn new(latitude: Angle, longitude: Angle) -> Self {
        Self {
            latitude,
            longitude,
        }
    }

    pub const fn from_degrees(latitude: f64, longitude: f64) -> Self {
        Self::new(
            Angle::from_degrees(latitude),
            Angle::from_degrees(longitude),
        )
    }

    pub const fn latitude(self) -> Angle {
        self.latitude
    }

    pub const fn longitude(self) -> Angle {
        self.longitude
    }

    /// The geodesic distance to `other` on the WGS84 ellipsoid.
    pub fn distance(self, other: Self) -> Length {
        let s12: f64 = Geodesic::wgs84().inverse(
            self.latitude.as_degrees(),
            self.longitude.as_degrees(),
            other.latitude.as_degrees(),
            other.longitude.as_degrees(),
        );
        Length::from_meters(s12)
    }

    /// The initial bearing (forward azimuth) of the geodesic towards
    /// `other`, normalized to the compass range `[0°, 360°)`.
    pub fn bearing(self, other: Self) -> Angle {
        // This tuple arity only solves for the azimuths, skipping the
        // distance computation.
        let (azi1, _azi2, _a12): (f64, f64, f64) = Geodesic::wgs84().inverse(
            self.latitude.as_degrees(),
            self.longitude.as_degrees(),
            other.latitude.as_degrees(),
            other.longitude.as_degrees(),
        );
        Angle::from_degrees(azi1).normalized()
    }

    /// Distance and initial bearing to `other` from a single geodesic
    /// solution, cheaper than calling [`distance`](Self::distance) and
    /// [`bearing`](Self::bearing) separately.
    pub fn distance_bearing(self, other: Self) -> (Length, Angle) {
        let (s12, azi1, _azi2, _a12): (f64, f64, f64, f64) = Geodesic::wgs84().inverse(
            self.latitude.as_degrees(),
            self.longitude.as_degrees(),
            other.latitude.as_degrees(),
            other.longitude.as_degrees(),
        );
        (
            Length::from_meters(s12),
            Angle::from_degrees(azi1).normalized(),
        )
    }

    /// The point reached by following the geodesic that leaves this
    /// point with the given initial `bearing` for `distance`.
    pub fn destination(self, bearing: Angle, distance: Length) -> Self {
        let (lat2, lon2): (f64, f64) = Geodesic::wgs84().direct(
            self.latitude.as_degrees(),
            self.longitude.as_degrees(),
            bearing.as_degrees(),
            distance.as_meters(),
        );
        Self::from_degrees(lat2, lon2)
    }

    /// The great-circle distance to `other` on a mean-radius sphere.
    ///
    /// Substantially faster than [`distance`](Self::distance) but off by
    /// up to ~0.5% (latitude-dependent) from the ellipsoidal value, so
    /// it must not be used for scored distances.
    ///
    /// NaN coordinates yield a NaN distance, but unlike the geodesic
    /// methods this returns a plausible finite value for latitudes
    /// outside ±90° rather than NaN.
    pub fn haversine_distance(self, other: Self) -> Length {
        let lat1 = self.latitude.as_radians();
        let lat2 = other.latitude.as_radians();
        let half_dlat = (lat2 - lat1) / 2.;
        let half_dlon = (other.longitude - self.longitude).as_radians() / 2.;

        let a = half_dlat.sin().powi(2) + lat1.cos() * lat2.cos() * half_dlon.sin().powi(2);
        // Rounding can push `a` a hair above 1 for antipodal points,
        // so clamp to keep `asin` defined. `clamp` (unlike `min`)
        // propagates NaN, keeping garbage input detectable.
        let central_angle = 2. * a.sqrt().clamp(0., 1.).asin();
        Length::from_meters(MEAN_EARTH_RADIUS_METERS * central_angle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::{assert_abs_diff_eq, assert_relative_eq};

    #[test]
    fn accessors() {
        let point = LatLon::from_degrees(51.5, -0.1278);
        assert_eq!(point.latitude().as_degrees(), 51.5);
        assert_eq!(point.longitude().as_degrees(), -0.1278);
        assert_eq!(
            point,
            LatLon::new(Angle::from_degrees(51.5), Angle::from_degrees(-0.1278))
        );
    }

    #[test]
    fn distance_matches_geographiclib_reference() {
        // Reference value from the `geographiclib-rs` documentation
        // (Los Angeles to Tallinn).
        let los_angeles = LatLon::from_degrees(34.095925, -118.2884237);
        let tallinn = LatLon::from_degrees(59.4323439, 24.7341649);
        let distance = los_angeles.distance(tallinn);
        assert_relative_eq!(distance, Length::from_meters(9_094_718.72751138));
    }

    #[test]
    fn distance_matches_vincenty_reference() {
        // The classic Vincenty test pair on WGS84 (Lizard Point to
        // Dunnet Head), 969954.166 m per the movable-type reference
        // implementation.
        let lizard_point = LatLon::from_degrees(50.06632, -5.71475);
        let dunnet_head = LatLon::from_degrees(58.64402, -3.07009);
        let distance = lizard_point.distance(dunnet_head);
        assert_relative_eq!(
            distance,
            Length::from_meters(969_954.166314209),
            max_relative = 1e-13
        );
    }

    #[test]
    fn distance_is_symmetric() {
        let a = LatLon::from_degrees(47.0, 11.0);
        let b = LatLon::from_degrees(52.0, -3.0);
        // Sub-nanometer tolerance: symmetry is a property of the
        // geodesic, but bit-exact equality would pin an implementation
        // detail of `geographiclib-rs`.
        assert_relative_eq!(a.distance(b), b.distance(a));
        assert_eq!(a.distance(a), Length::ZERO);
    }

    #[test]
    fn distance_is_invariant_to_longitude_wrapping() {
        // Longitudes may be any angle, so a point given as 190° is the
        // same point as -170°.
        let wrapped = LatLon::from_degrees(10., 190.);
        let signed = LatLon::from_degrees(10., -170.);
        let target = LatLon::from_degrees(20., -100.);
        assert_relative_eq!(wrapped.distance(target), signed.distance(target));
    }

    #[test]
    fn bearing_cardinal_directions() {
        let origin = LatLon::from_degrees(0., 0.);
        let cases = [
            (LatLon::from_degrees(10., 0.), 0.),
            (LatLon::from_degrees(0., 10.), 90.),
            (LatLon::from_degrees(-10., 0.), 180.),
            // The compass normalization turns -90° into 270°.
            (LatLon::from_degrees(0., -10.), 270.),
        ];
        for (target, expected) in cases {
            assert_abs_diff_eq!(origin.bearing(target), Angle::from_degrees(expected),);
        }
    }

    #[test]
    fn distance_bearing_matches_individual_calls() {
        let a = LatLon::from_degrees(51.5, -0.1278);
        let b = LatLon::from_degrees(48.8566, 2.3522);
        let (distance, bearing) = a.distance_bearing(b);
        assert_eq!(distance, a.distance(b));
        assert_eq!(bearing, a.bearing(b));
    }

    #[test]
    fn destination_matches_geographiclib_reference() {
        // Reference value from the `geographiclib-rs` documentation:
        // 10000 km northeast of JFK.
        let jfk = LatLon::from_degrees(40.64, -73.78);
        let destination =
            jfk.destination(Angle::from_degrees(45.), Length::from_kilometers(10_000.));
        assert_abs_diff_eq!(
            destination,
            LatLon::from_degrees(32.621100463725796, 49.052487092959836),
        );
    }

    #[test]
    fn destination_round_trips_through_inverse() {
        let distance = Length::from_kilometers(100.);
        for lat in [-60., -30., 0., 30., 60.] {
            for lon in [-170., -45., 0., 90.] {
                for bearing_degrees in [0., 77., 180., 291.] {
                    let origin = LatLon::from_degrees(lat, lon);
                    let bearing = Angle::from_degrees(bearing_degrees);
                    let there = origin.destination(bearing, distance);

                    let (back_distance, back_bearing) = origin.distance_bearing(there);
                    assert_relative_eq!(back_distance, distance, epsilon = 1e-8);
                    assert_abs_diff_eq!(back_bearing, bearing, epsilon = 1e-8);
                }
            }
        }
    }

    #[test]
    fn haversine_approximates_geodesic() {
        let a = LatLon::from_degrees(51.5, -0.1278);
        let b = LatLon::from_degrees(48.8566, 2.3522);
        let geodesic = a.distance(b);
        let haversine = a.haversine_distance(b);
        assert_relative_eq!(haversine, geodesic, max_relative = 0.005);

        assert_eq!(a.haversine_distance(a), Length::ZERO);
        assert_eq!(a.haversine_distance(b), b.haversine_distance(a));
    }

    #[test]
    fn haversine_handles_antipodal_points() {
        let a = LatLon::from_degrees(0., 0.);
        let b = LatLon::from_degrees(0., 180.);
        let half_circumference =
            Length::from_meters(MEAN_EARTH_RADIUS_METERS * std::f64::consts::PI);
        assert_relative_eq!(a.haversine_distance(b), half_circumference);
    }

    #[test]
    fn garbage_input_stays_detectable() {
        // Garbage in, NaN out: corrupted coordinates must not turn into
        // plausible distances or bearings.
        let out_of_range = LatLon::from_degrees(95., 0.);
        let nan = LatLon::from_degrees(f64::NAN, 0.);
        let good = LatLon::from_degrees(47., 8.);

        assert!(out_of_range.distance(good).as_meters().is_nan());
        assert!(out_of_range.bearing(good).as_radians().is_nan());
        let (distance, bearing) = out_of_range.distance_bearing(good);
        assert!(distance.as_meters().is_nan());
        assert!(bearing.as_radians().is_nan());

        assert!(nan.haversine_distance(good).as_meters().is_nan());
        assert!(good.haversine_distance(nan).as_meters().is_nan());
    }

    #[test]
    fn debug() {
        let point = LatLon::from_degrees(51.5, -0.1278);
        assert_eq!(
            format!("{point:?}"),
            "LatLon { latitude: 51.5°, longitude: -0.1278° }"
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde() {
        // Angles serialize as degrees, so a `LatLon` reads naturally.
        let point = LatLon::from_degrees(51.5, -0.1278);
        let json = serde_json::to_string(&point).unwrap();
        assert_eq!(json, r#"{"latitude":51.5,"longitude":-0.1278}"#);
        assert_eq!(serde_json::from_str::<LatLon>(&json).unwrap(), point);
    }
}
