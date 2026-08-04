use crate::AirspaceGeometryError;
use ::openair::{Coord, Direction, Geometry, PolygonSegment};
use std::f64::consts::TAU;
use updraft_geo::{LatLon, Polygon};
use updraft_units::{Angle, Length};

/// The maximum distance between a normalized curve and its chord.
pub const MAX_AIRSPACE_CURVE_ERROR: Length = Length::from_meters(1.);

/// Converts one parsed geometry to a canonical airspace polygon.
///
/// # Errors
///
/// Returns an error if a coordinate, radius, or polygon ring is invalid.
pub fn normalize_geometry(geometry: Geometry) -> Result<Polygon, AirspaceGeometryError> {
    let vertices = match geometry {
        Geometry::Circle {
            centerpoint,
            radius,
        } => normalize_circle(centerpoint, radius)?,
        Geometry::Polygon { segments } => normalize_polygon_segments(segments)?,
    };

    let mut distinct = Vec::with_capacity(3);
    for vertex in &vertices {
        if !distinct.contains(vertex) {
            distinct.push(*vertex);
            if distinct.len() == 3 {
                break;
            }
        }
    }
    if distinct.len() < 3 {
        return Err(AirspaceGeometryError::InvalidRing);
    }

    Ok(Polygon::from_vertices(vertices))
}

/// Converts one parsed circle to polygon vertices within the curve-error limit.
fn normalize_circle(
    centerpoint: Coord,
    radius_nautical_miles: f32,
) -> Result<Vec<LatLon>, AirspaceGeometryError> {
    let center = validate_coord(centerpoint)?;
    let radius = normalize_radius(radius_nautical_miles)?;
    let sweep = Angle::from_radians(TAU);
    let segment_count = curve_segment_count(radius, sweep, MAX_AIRSPACE_CURVE_ERROR)?.max(3);
    Ok((0..segment_count)
        .map(|index| {
            let bearing = sweep * index as f64 / segment_count as f64;
            center.destination(bearing, radius)
        })
        .collect())
}

/// Converts parsed point and arc segments to polygon vertices and removes adjacent duplicates.
fn normalize_polygon_segments(
    segments: Vec<PolygonSegment>,
) -> Result<Vec<LatLon>, AirspaceGeometryError> {
    let mut vertices = Vec::new();
    for segment in segments {
        let normalized = match segment {
            PolygonSegment::Point(point) => {
                let vertex = validate_coord(point)?;
                if vertices.last() != Some(&vertex) {
                    vertices.push(vertex);
                }
                continue;
            }
            PolygonSegment::Arc(arc) => normalize_db_arc(arc)?,
            PolygonSegment::ArcSegment(arc) => normalize_da_arc(arc)?,
        };
        for vertex in normalized {
            if vertices.last() != Some(&vertex) {
                vertices.push(vertex);
            }
        }
    }
    Ok(vertices)
}

/// Converts a `DB` arc to vertices within the curve-error limit.
/// Interpolates the radius between the exact endpoints.
fn normalize_db_arc(arc: ::openair::Arc) -> Result<Vec<LatLon>, AirspaceGeometryError> {
    let center = validate_coord(arc.centerpoint)?;
    let start = validate_coord(arc.start)?;
    let end = validate_coord(arc.end)?;
    let start_radius = center.distance(start);
    let end_radius = center.distance(end);
    if start_radius == Length::ZERO || end_radius == Length::ZERO {
        return Err(AirspaceGeometryError::InvalidRadius);
    }
    let maximum_radius = if start_radius > end_radius {
        start_radius
    } else {
        end_radius
    };
    let start_bearing = center.bearing(start);
    let end_bearing = center.bearing(end);
    let sweep = directed_sweep(start_bearing, end_bearing, arc.direction);
    let segment_count = curve_segment_count(maximum_radius, sweep, MAX_AIRSPACE_CURVE_ERROR)?;
    let capacity = segment_count
        .checked_add(1)
        .ok_or(AirspaceGeometryError::InvalidRadius)?;
    let mut vertices = Vec::with_capacity(capacity);
    vertices.push(start);
    for index in 1..segment_count {
        let fraction = index as f64 / segment_count as f64;
        let radius = start_radius + (end_radius - start_radius) * fraction;
        vertices.push(center.destination(
            arc_bearing(start_bearing, sweep, arc.direction, index, segment_count),
            radius,
        ));
    }
    vertices.push(end);
    Ok(vertices)
}

/// Converts a `DA` arc at a constant radius to vertices within the curve-error limit.
fn normalize_da_arc(arc: ::openair::ArcSegment) -> Result<Vec<LatLon>, AirspaceGeometryError> {
    let center = validate_coord(arc.centerpoint)?;
    let radius = normalize_radius(arc.radius)?;
    let start_bearing = Angle::from_degrees(f64::from(arc.angle_start));
    let end_bearing = Angle::from_degrees(f64::from(arc.angle_end));
    if !start_bearing.as_radians().is_finite() || !end_bearing.as_radians().is_finite() {
        return Err(AirspaceGeometryError::InvalidCoordinate);
    }
    let sweep = directed_sweep(start_bearing, end_bearing, arc.direction);
    let segment_count = curve_segment_count(radius, sweep, MAX_AIRSPACE_CURVE_ERROR)?;
    let capacity = segment_count
        .checked_add(1)
        .ok_or(AirspaceGeometryError::InvalidRadius)?;
    let mut vertices = Vec::with_capacity(capacity);
    vertices.push(center.destination(start_bearing, radius));
    for index in 1..segment_count {
        vertices.push(center.destination(
            arc_bearing(start_bearing, sweep, arc.direction, index, segment_count),
            radius,
        ));
    }
    vertices.push(center.destination(end_bearing, radius));
    Ok(vertices)
}

/// Returns the positive sweep angle for the specified direction.
/// Returns one full turn when the start and end bearings are equal.
fn directed_sweep(start: Angle, end: Angle, direction: Direction) -> Angle {
    let sweep = match direction {
        Direction::Cw => (end - start).normalized(),
        Direction::Ccw => (start - end).normalized(),
    };
    if sweep == Angle::ZERO {
        Angle::from_radians(TAU)
    } else {
        sweep
    }
}

/// Returns the bearing at one boundary between arc segments.
fn arc_bearing(
    start: Angle,
    sweep: Angle,
    direction: Direction,
    index: usize,
    segment_count: usize,
) -> Angle {
    let offset = sweep * index as f64 / segment_count as f64;
    match direction {
        Direction::Cw => start + offset,
        Direction::Ccw => start - offset,
    }
}

/// Calculates the minimum segment count that keeps the chord error within the specified limit.
fn curve_segment_count(
    radius: Length,
    sweep: Angle,
    error_budget: Length,
) -> Result<usize, AirspaceGeometryError> {
    let acos_input = (1. - error_budget / radius).clamp(-1., 1.);
    let maximum_step = Angle::from_radians(2. * acos_input.acos());
    let segment_count = (sweep / maximum_step).ceil();
    if !maximum_step.as_radians().is_finite()
        || maximum_step <= Angle::ZERO
        || !segment_count.is_finite()
        || segment_count > usize::MAX as f64
    {
        return Err(AirspaceGeometryError::InvalidRadius);
    }
    Ok((segment_count as usize).max(1))
}

/// Checks a parsed coordinate and converts it to a latitude and longitude.
fn validate_coord(coord: Coord) -> Result<LatLon, AirspaceGeometryError> {
    if coord.lat.is_finite()
        && coord.lng.is_finite()
        && (-90. ..=90.).contains(&coord.lat)
        && (-180. ..=180.).contains(&coord.lng)
    {
        Ok(LatLon::from_degrees(coord.lat, coord.lng))
    } else {
        Err(AirspaceGeometryError::InvalidCoordinate)
    }
}

/// Converts a radius from nautical miles to a typed length and checks its value.
fn normalize_radius(radius_nautical_miles: f32) -> Result<Length, AirspaceGeometryError> {
    let radius = Length::from_nautical_miles(f64::from(radius_nautical_miles));
    validate_radius(radius)?;
    Ok(radius)
}

/// Checks that a radius is finite and greater than zero.
fn validate_radius(radius: Length) -> Result<(), AirspaceGeometryError> {
    if radius.as_meters().is_finite() && radius.as_meters() > 0. {
        Ok(())
    } else {
        Err(AirspaceGeometryError::InvalidRadius)
    }
}
