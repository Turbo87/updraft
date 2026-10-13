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
//! - The line methods take the line as the first argument.
//! - For an empty line, [`CheapRuler::along`] and
//!   [`CheapRuler::point_on_line`] return `None`, and the slice methods
//!   return an empty `Vec`.
//! - For a line with one point, [`CheapRuler::point_on_line`] returns the
//!   distance to that point. Upstream returns infinity.
//! - The buffer methods return a [`BoundingBox`]. The box crosses the
//!   antimeridian instead of having longitudes outside ±180°. Latitudes
//!   are clamped to ±90°.
//! - `area`, `fromTile`, `insideBBox`, and `units` are not ported. Use
//!   [`BoundingBox::contains`] instead of `insideBBox`.
//!
//! [cheap-ruler]: https://github.com/mapbox/cheap-ruler

use crate::{BoundingBox, LatLon};
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

    /// The total length of `line`. An empty line has zero length.
    pub fn line_distance(&self, line: &[LatLon]) -> Length {
        line.windows(2).fold(Length::ZERO, |sum, pair| {
            sum + self.distance(pair[0], pair[1])
        })
    }

    /// The point at `distance` along `line`, or `None` for an empty line.
    ///
    /// A negative distance gives the first point. A distance beyond the
    /// end of the line gives the last point.
    pub fn along(&self, line: &[LatLon], distance: Length) -> Option<LatLon> {
        let first = *line.first()?;
        if distance <= Length::ZERO {
            return Some(first);
        }

        let mut sum = Length::ZERO;
        for pair in line.windows(2) {
            let segment = self.distance(pair[0], pair[1]);
            sum += segment;
            if sum > distance {
                let t = (distance - (sum - segment)) / segment;
                return Some(interpolate(pair[0], pair[1], t));
            }
        }

        line.last().copied()
    }

    /// The distance from `point` to the line segment from `a` to `b`.
    pub fn point_to_segment_distance(&self, point: LatLon, a: LatLon, b: LatLon) -> Length {
        let (closest, _) = self.closest_point_on_segment(point, a, b);
        self.distance(point, closest)
    }

    /// The point on `line` that is closest to `point`, or `None` for an
    /// empty line.
    pub fn point_on_line(&self, line: &[LatLon], point: LatLon) -> Option<PointOnLine> {
        let first = *line.first()?;
        let mut closest = PointOnLine {
            point: first,
            index: 0,
            t: 0.,
            distance: self.distance(point, first),
        };

        for (index, pair) in line.windows(2).enumerate() {
            let (candidate, t) = self.closest_point_on_segment(point, pair[0], pair[1]);
            let distance = self.distance(point, candidate);
            if index == 0 || distance < closest.distance {
                closest = PointOnLine {
                    point: candidate,
                    index,
                    t: t.clamp(0., 1.),
                    distance,
                };
            }
        }

        Some(closest)
    }

    /// The part of `line` between the points on the line that are closest
    /// to `start` and `stop`.
    ///
    /// The slice follows the direction of `line`, also if `stop` comes
    /// before `start`. An empty line gives an empty slice.
    pub fn line_slice(&self, line: &[LatLon], start: LatLon, stop: LatLon) -> Vec<LatLon> {
        let (Some(mut start), Some(mut stop)) = (
            self.point_on_line(line, start),
            self.point_on_line(line, stop),
        ) else {
            return Vec::new();
        };

        if start.index > stop.index || (start.index == stop.index && start.t > stop.t) {
            std::mem::swap(&mut start, &mut stop);
        }

        let mut slice = vec![start.point];

        let left = start.index + 1;
        let right = stop.index;
        if left <= right && line[left] != start.point {
            slice.push(line[left]);
        }
        if left < right {
            slice.extend_from_slice(&line[left + 1..=right]);
        }
        if line[right] != stop.point {
            slice.push(stop.point);
        }

        slice
    }

    /// The part of `line` between the distances `start` and `stop` along
    /// the line.
    ///
    /// The order of `start` and `stop` does not matter. A negative distance
    /// is the start of the line. The slice is empty if `start` is beyond the
    /// end of the line.
    pub fn line_slice_along(&self, line: &[LatLon], start: Length, stop: Length) -> Vec<LatLon> {
        let (start, stop) = if start > stop {
            (stop, start)
        } else {
            (start, stop)
        };
        let start = if start < Length::ZERO {
            Length::ZERO
        } else {
            start
        };

        let mut slice = Vec::new();
        let mut sum = Length::ZERO;
        for pair in line.windows(2) {
            let segment = self.distance(pair[0], pair[1]);
            sum += segment;

            if sum > start && slice.is_empty() {
                let t = (start - (sum - segment)) / segment;
                slice.push(interpolate(pair[0], pair[1], t));
            }

            if sum >= stop {
                let t = if segment > Length::ZERO {
                    (stop - (sum - segment)) / segment
                } else {
                    0.
                };
                slice.push(interpolate(pair[0], pair[1], t));
                return slice;
            }

            if sum > start {
                slice.push(pair[1]);
            }
        }

        slice
    }

    /// The box that contains all points within `buffer` of `point`.
    ///
    /// `buffer` must not be negative.
    pub fn buffer_point(&self, point: LatLon, buffer: Length) -> BoundingBox {
        let (latitude, longitude) = (point.latitude(), point.longitude());
        self.buffer(latitude, latitude, longitude, Angle::ZERO, buffer)
    }

    /// The box that contains all points within `buffer` of `bbox`.
    ///
    /// `buffer` must not be negative.
    pub fn buffer_bbox(&self, bbox: BoundingBox, buffer: Length) -> BoundingBox {
        let (south, north) = (bbox.south(), bbox.north());
        self.buffer(south, north, bbox.west(), bbox.longitude_span(), buffer)
    }

    /// Extends the box with the eastward longitude span `span` from `west`
    /// by `buffer` on all sides.
    fn buffer(
        &self,
        south: Angle,
        north: Angle,
        west: Angle,
        span: Angle,
        buffer: Length,
    ) -> BoundingBox {
        const QUARTER: Angle = Angle::from_degrees(90.);
        const HALF: Angle = Angle::from_degrees(180.);

        let vertical = Angle::from_radians(buffer.as_meters() / self.ky);
        let horizontal = Angle::from_radians(buffer.as_meters() / self.kx);

        let south = south - vertical;
        let south = if south < -QUARTER { -QUARTER } else { south };
        let north = north + vertical;
        let north = if north > QUARTER { QUARTER } else { north };
        let span = span + horizontal * 2.;
        let (west, east) = if span >= HALF * 2. {
            (-HALF, HALF)
        } else {
            let west = west - horizontal;
            (west.normalized_signed(), (west + span).normalized_signed())
        };

        BoundingBox::new(south, north, west, east)
    }

    /// The point on the segment from `a` to `b` that is closest to `point`,
    /// and the unclamped position of the projection on the segment.
    fn closest_point_on_segment(&self, point: LatLon, a: LatLon, b: LatLon) -> (LatLon, f64) {
        let (dx, dy) = self.delta(a, b);
        if dx == 0. && dy == 0. {
            return (a, 0.);
        }

        let (px, py) = self.delta(a, point);
        let t = (px * dx + py * dy) / (dx * dx + dy * dy);
        let closest = if t > 1. {
            b
        } else if t > 0. {
            interpolate(a, b, t)
        } else {
            a
        };
        (closest, t)
    }

    /// The east and north offsets in meters from `a` to `b`.
    fn delta(&self, a: LatLon, b: LatLon) -> (f64, f64) {
        let dx = wrap((b.longitude() - a.longitude()).as_radians()) * self.kx;
        let dy = (b.latitude() - a.latitude()).as_radians() * self.ky;
        (dx, dy)
    }
}

/// The point closest to a given point on a line. See
/// [`CheapRuler::point_on_line`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointOnLine {
    /// The closest point on the line.
    pub point: LatLon,
    /// The index of the first point of the segment that contains `point`.
    pub index: usize,
    /// The position of `point` on its segment, from 0 at the first point to
    /// 1 at the second point.
    pub t: f64,
    /// The distance from the given point to `point`.
    pub distance: Length,
}

/// The point at the fraction `t` of the way from `a` to `b`.
fn interpolate(a: LatLon, b: LatLon, t: f64) -> LatLon {
    let longitude = Angle::from_radians(wrap((b.longitude() - a.longitude()).as_radians()));
    LatLon::new(
        a.latitude() + (b.latitude() - a.latitude()) * t,
        a.longitude() + longitude * t,
    )
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
