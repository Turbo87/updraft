//! Conversions to and from [`geo_types`] (the `geo-types` feature).
//!
//! Geographic coordinates in the `GeoRust` ecosystem follow the x/y
//! convention: **x is longitude, y is latitude**, both in degrees.
//! These conversions are the only place where that mapping happens, so
//! the swap footgun stays contained.
//!
//! [`BoundingBox`](crate::BoundingBox) deliberately has no conversion:
//! [`geo_types::Rect`] cannot represent a box crossing the
//! antimeridian.

use geo_types::{Coord, Point};

use crate::LatLon;

impl From<LatLon> for Coord {
    fn from(point: LatLon) -> Self {
        Coord {
            x: point.longitude().as_degrees(),
            y: point.latitude().as_degrees(),
        }
    }
}

impl From<Coord> for LatLon {
    fn from(coord: Coord) -> Self {
        Self::from_degrees(coord.y, coord.x)
    }
}

impl From<LatLon> for Point {
    fn from(point: LatLon) -> Self {
        Self(point.into())
    }
}

impl From<Point> for LatLon {
    fn from(point: Point) -> Self {
        point.0.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latlon_to_geo_types() {
        let point = LatLon::from_degrees(51.5, -0.1278);

        let coord = Coord::from(point);
        assert_eq!(coord.x, -0.1278);
        assert_eq!(coord.y, 51.5);

        let geo_point = Point::from(point);
        assert_eq!(geo_point.x(), -0.1278);
        assert_eq!(geo_point.y(), 51.5);
    }

    #[test]
    fn geo_types_to_latlon() {
        let coord = Coord {
            x: -0.1278,
            y: 51.5,
        };
        assert_eq!(LatLon::from(coord), LatLon::from_degrees(51.5, -0.1278));

        let point = Point::new(-0.1278, 51.5);
        assert_eq!(LatLon::from(point), LatLon::from_degrees(51.5, -0.1278));
    }
}
