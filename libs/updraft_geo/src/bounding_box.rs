use crate::LatLon;
use updraft_units::Angle;

/// A latitude/longitude-aligned bounding box, possibly crossing the
/// antimeridian.
///
/// Latitudes run from `south` to `north` (`south <= north` expected,
/// both values in `[-90°, 90°]`).
/// The longitude span runs **eastward** from `west` to `east`, with both
/// values expected in `[-180°, 180°]`: `west <= east` is an ordinary
/// box, `west > east` is a box crossing the antimeridian, `west == east`
/// is a zero-width box, and `west == -180°, east == 180°` covers all
/// longitudes.
///
/// With the `approx` feature, approximate-equality comparisons operate
/// on the internal **radians** of the four edges, so epsilons are
/// radians, not degrees.
///
/// With the `serde` feature, deserialization does not validate the
/// edge expectations.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(any(test, feature = "approx"), derive(approx::RelativeEq))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BoundingBox {
    south: Angle,
    north: Angle,
    west: Angle,
    east: Angle,
}

impl BoundingBox {
    /// Creates a box from its four edges.
    ///
    /// The documented expectations (`south <= north`, both within ±90°,
    /// `west` and `east` within ±180°) are debug-asserted, not
    /// validated in release builds.
    pub const fn new(south: Angle, north: Angle, west: Angle, east: Angle) -> Self {
        debug_assert!(south.as_degrees() <= north.as_degrees());
        debug_assert!(south.as_degrees() >= -90. && south.as_degrees() <= 90.);
        debug_assert!(north.as_degrees() >= -90. && north.as_degrees() <= 90.);
        debug_assert!(west.as_degrees() >= -180. && west.as_degrees() <= 180.);
        debug_assert!(east.as_degrees() >= -180. && east.as_degrees() <= 180.);
        Self {
            south,
            north,
            west,
            east,
        }
    }

    /// The smallest box containing all `points`, or `None` for an empty
    /// iterator or when any coordinate is non-finite.
    ///
    /// The longitude interval is chosen by excluding the largest
    /// longitudinal gap between the points, so point sets straddling the
    /// antimeridian produce a crossing box instead of one spanning the
    /// whole globe.
    pub fn from_points<I: IntoIterator<Item = LatLon>>(points: I) -> Option<Self> {
        let mut points = points.into_iter();
        let first = points.next().filter(|point| is_finite(*point))?;
        let mut south = first.latitude();
        let mut north = first.latitude();
        let mut longitudes = vec![first.longitude().normalized_signed()];

        for point in points {
            if !is_finite(point) {
                return None;
            }
            if point.latitude() < south {
                south = point.latitude();
            }
            if point.latitude() > north {
                north = point.latitude();
            }
            longitudes.push(point.longitude().normalized_signed());
        }

        longitudes.sort_unstable_by(|a, b| a.as_radians().total_cmp(&b.as_radians()));

        // Find the largest circular gap between adjacent longitudes. The
        // box covers everything except that gap. If the largest gap is
        // the wrap-around between the last and first sorted value, the
        // result is an ordinary non-crossing box.
        let mut west = longitudes[0];
        let mut east = longitudes[longitudes.len() - 1];
        let mut largest_gap = west.as_degrees() + 360. - east.as_degrees();
        for pair in longitudes.windows(2) {
            let gap = (pair[1] - pair[0]).as_degrees();
            if gap > largest_gap {
                largest_gap = gap;
                west = pair[1];
                east = pair[0];
            }
        }

        Some(Self::new(south, north, west, east))
    }

    pub const fn south(self) -> Angle {
        self.south
    }

    pub const fn north(self) -> Angle {
        self.north
    }

    pub const fn west(self) -> Angle {
        self.west
    }

    pub const fn east(self) -> Angle {
        self.east
    }

    pub fn crosses_antimeridian(self) -> bool {
        self.east.as_degrees() < self.west.as_degrees()
    }

    pub fn latitude_span(self) -> Angle {
        self.north - self.south
    }

    /// The eastward extent from `west` to `east`, in `[0°, 360°]`.
    pub fn longitude_span(self) -> Angle {
        Angle::from_degrees(span_degrees(self.west.as_degrees(), self.east.as_degrees()))
    }

    /// The geographic center of the box, correct for boxes crossing the
    /// antimeridian.
    pub fn center(self) -> LatLon {
        let latitude = (self.south + self.north) / 2.;
        let longitude = self.west + self.longitude_span() / 2.;
        LatLon::new(latitude, longitude.normalized_signed())
    }

    /// Whether `point` lies inside the box. Boundaries are inclusive.
    pub fn contains(self, point: LatLon) -> bool {
        let latitude = point.latitude().as_degrees();
        if !(latitude >= self.south.as_degrees() && latitude <= self.north.as_degrees()) {
            return false;
        }

        // Both the point and the eastward span are measured from `west`,
        // wrapped into [0°, 360°), which makes the comparison immune to
        // the antimeridian.
        let west = self.west.as_degrees();
        let offset = (point.longitude().as_degrees() - west).rem_euclid(360.);
        offset <= span_degrees(west, self.east.as_degrees())
    }
}

fn is_finite(point: LatLon) -> bool {
    point.latitude().as_radians().is_finite() && point.longitude().as_radians().is_finite()
}

/// The eastward extent from `west` to `east` in degrees, in `[0, 360]`.
///
/// Kept in degrees (rather than going through [`Angle`]'s radian
/// representation) so that [`BoundingBox::contains`] compares the point
/// offset and the span computed from bit-identical subtractions.
fn span_degrees(west: f64, east: f64) -> f64 {
    let span = east - west;
    if span < 0. { span + 360. } else { span }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    fn degrees(south: f64, north: f64, west: f64, east: f64) -> BoundingBox {
        BoundingBox::new(
            Angle::from_degrees(south),
            Angle::from_degrees(north),
            Angle::from_degrees(west),
            Angle::from_degrees(east),
        )
    }

    #[test]
    fn from_points_simple() {
        let bbox = BoundingBox::from_points([
            LatLon::from_degrees(47., 8.),
            LatLon::from_degrees(52., 14.),
            LatLon::from_degrees(50., 6.),
        ])
        .unwrap();
        assert_abs_diff_eq!(bbox, degrees(47., 52., 6., 14.));
        assert!(!bbox.crosses_antimeridian());
    }

    #[test]
    fn from_points_empty_and_single() {
        assert_eq!(BoundingBox::from_points([]), None);

        let bbox = BoundingBox::from_points([LatLon::from_degrees(47., 8.)]).unwrap();
        assert_abs_diff_eq!(bbox, degrees(47., 47., 8., 8.));
        assert_eq!(bbox.longitude_span(), Angle::ZERO);
        assert!(bbox.contains(LatLon::from_degrees(47., 8.)));
        assert!(!bbox.contains(LatLon::from_degrees(47., 8.1)));
    }

    #[test]
    fn from_points_rejects_non_finite_coordinates() {
        // A single corrupted point must not silently poison the box or
        // get dropped. The caller has to see that the input was bad.
        let good = LatLon::from_degrees(47., 8.);
        let nan_latitude = LatLon::from_degrees(f64::NAN, 8.);
        let nan_longitude = LatLon::from_degrees(47., f64::NAN);
        let infinite = LatLon::from_degrees(f64::INFINITY, 8.);

        assert_eq!(BoundingBox::from_points([nan_latitude, good]), None);
        assert_eq!(BoundingBox::from_points([good, nan_latitude]), None);
        assert_eq!(BoundingBox::from_points([good, nan_longitude]), None);
        assert_eq!(BoundingBox::from_points([infinite]), None);
    }

    #[test]
    fn from_points_across_antimeridian() {
        let bbox = BoundingBox::from_points([
            LatLon::from_degrees(-40., 170.),
            LatLon::from_degrees(-35., -175.),
            LatLon::from_degrees(-38., 179.),
        ])
        .unwrap();
        assert_abs_diff_eq!(bbox, degrees(-40., -35., 170., -175.));
        assert!(bbox.crosses_antimeridian());
        assert_abs_diff_eq!(bbox.longitude_span(), Angle::from_degrees(15.),);
        assert!(bbox.contains(LatLon::from_degrees(-38., 180.)));
        assert!(!bbox.contains(LatLon::from_degrees(-38., 0.)));
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn new_rejects_inverted_latitudes() {
        degrees(52., 47., 6., 14.);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn new_rejects_out_of_range_longitudes() {
        degrees(0., 10., 200., -170.);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn new_rejects_out_of_range_latitudes() {
        degrees(47., 95., 6., 14.);
    }

    #[test]
    fn from_points_normalizes_longitudes() {
        // Longitudes outside [-180°, 180°] (e.g. from a 0-360° source)
        // are normalized, so this box does not cross the antimeridian.
        let bbox = BoundingBox::from_points([
            LatLon::from_degrees(0., 190.),
            LatLon::from_degrees(1., -160.),
        ])
        .unwrap();
        assert_abs_diff_eq!(bbox, degrees(0., 1., -170., -160.), epsilon = 1e-12);
        assert!(!bbox.crosses_antimeridian());

        // +180° and -180° normalize to the same longitude, giving a
        // zero-width box instead of one spanning the whole globe.
        let bbox = BoundingBox::from_points([
            LatLon::from_degrees(0., 180.),
            LatLon::from_degrees(1., -180.),
        ])
        .unwrap();
        assert_abs_diff_eq!(bbox, degrees(0., 1., 180., 180.));
        assert_eq!(bbox.longitude_span(), Angle::ZERO);
    }

    #[test]
    fn contains_is_inclusive_at_the_boundary() {
        let bbox = degrees(47., 52., 6., 14.);
        assert!(bbox.contains(LatLon::from_degrees(47., 6.)));
        assert!(bbox.contains(LatLon::from_degrees(52., 14.)));
        assert!(bbox.contains(LatLon::from_degrees(50., 10.)));
        assert!(!bbox.contains(LatLon::from_degrees(46.9, 10.)));
        assert!(!bbox.contains(LatLon::from_degrees(52.1, 10.)));
        assert!(!bbox.contains(LatLon::from_degrees(50., 5.9)));
        assert!(!bbox.contains(LatLon::from_degrees(50., 14.1)));
    }

    #[test]
    fn contains_across_antimeridian() {
        let bbox = degrees(-40., -35., 170., -170.);
        assert!(bbox.contains(LatLon::from_degrees(-38., 175.)));
        assert!(bbox.contains(LatLon::from_degrees(-38., 180.)));
        assert!(bbox.contains(LatLon::from_degrees(-38., -175.)));
        assert!(bbox.contains(LatLon::from_degrees(-38., 170.)));
        assert!(bbox.contains(LatLon::from_degrees(-38., -170.)));
        assert!(!bbox.contains(LatLon::from_degrees(-38., 169.9)));
        assert!(!bbox.contains(LatLon::from_degrees(-38., -169.9)));
        assert!(!bbox.contains(LatLon::from_degrees(-38., 0.)));
    }

    #[test]
    fn contains_rejects_non_finite_coordinates() {
        // A corrupted point must never be reported as inside the box,
        // matching the "garbage stays detectable" contract of
        // `from_points` and the geodesic methods.
        let bbox = degrees(47., 52., 6., 14.);
        assert!(!bbox.contains(LatLon::from_degrees(f64::NAN, 10.)));
        assert!(!bbox.contains(LatLon::from_degrees(50., f64::NAN)));
        assert!(!bbox.contains(LatLon::from_degrees(f64::NAN, f64::NAN)));
        assert!(!bbox.contains(LatLon::from_degrees(f64::INFINITY, 10.)));
        assert!(!bbox.contains(LatLon::from_degrees(50., f64::INFINITY)));
    }

    #[test]
    fn whole_world_longitude_range() {
        let bbox = degrees(-90., 90., -180., 180.);
        assert_abs_diff_eq!(bbox.longitude_span(), Angle::from_degrees(360.));
        for longitude in [-180., -90., 0., 90., 180.] {
            assert!(bbox.contains(LatLon::from_degrees(0., longitude)));
        }
    }

    #[test]
    fn center() {
        let center = degrees(47., 53., 6., 14.).center();
        assert_abs_diff_eq!(center, LatLon::from_degrees(50., 10.));

        // A box crossing the antimeridian has its center on the far
        // side of the globe, not at the numeric midpoint. Rounding may
        // land the longitude on either side of ±180°, so compare in the
        // wrap-free compass range.
        let center = degrees(-40., -30., 170., -170.).center();
        assert_abs_diff_eq!(center.latitude(), Angle::from_degrees(-35.));
        assert_abs_diff_eq!(center.longitude().normalized(), Angle::from_degrees(180.));
    }

    #[test]
    fn spans() {
        let bbox = degrees(47., 52., 6., 14.);
        assert_abs_diff_eq!(bbox.latitude_span(), Angle::from_degrees(5.));
        assert_abs_diff_eq!(bbox.longitude_span(), Angle::from_degrees(8.));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde() {
        // Angles serialize as degrees, so a `BoundingBox` reads
        // naturally.
        let bbox = degrees(47., 52., 6.5, 14.);
        let json = serde_json::to_string(&bbox).unwrap();
        assert_eq!(
            json,
            r#"{"south":47.0,"north":52.0,"west":6.5,"east":14.0}"#
        );
        assert_eq!(serde_json::from_str::<BoundingBox>(&json).unwrap(), bbox);
    }
}
