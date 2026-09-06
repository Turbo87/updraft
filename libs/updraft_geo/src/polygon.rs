use crate::LatLon;

/// One canonical exterior ring without a repeated closing vertex.
#[derive(Clone, Debug, PartialEq)]
pub struct Polygon {
    vertices: Vec<LatLon>,
}

impl Polygon {
    /// Creates a ring with at least three distinct vertices.
    ///
    /// Repeated closing vertices are removed. Returns `None` when fewer than
    /// three distinct vertices remain. This does not check for self-intersections.
    pub fn from_vertices(mut vertices: Vec<LatLon>) -> Option<Self> {
        let first = *vertices.first()?;
        let second = *vertices.iter().find(|&&vertex| vertex != first)?;
        if !vertices
            .iter()
            .any(|&vertex| vertex != first && vertex != second)
        {
            return None;
        }
        while vertices.last() == Some(&first) {
            vertices.pop();
        }

        Some(Self { vertices })
    }

    /// Returns the vertices of the polygon.
    pub fn vertices(&self) -> &[LatLon] {
        &self.vertices
    }

    /// Convert the polygon to GeoJSON coordinates with a repeated closing vertex.
    pub fn to_geojson_coordinates(&self) -> Vec<[f64; 2]> {
        self.vertices
            .iter()
            .chain(self.vertices.first())
            .copied()
            .map(LatLon::to_geojson_coordinate)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some};

    const A: LatLon = LatLon::from_degrees(50., 10.);
    const B: LatLon = LatLon::from_degrees(50., 11.);
    const C: LatLon = LatLon::from_degrees(51., 10.);

    #[test]
    fn rejects_rings_with_fewer_than_three_distinct_vertices() {
        for vertices in [vec![], vec![A], vec![A; 4], vec![A, B], vec![A, B, A, B, A]] {
            assert_none!(Polygon::from_vertices(vertices));
        }
    }

    #[test]
    fn removes_closing_vertices_and_preserves_ring_order() {
        for vertices in [vec![A, B, C], vec![A, B, C, A], vec![A, B, C, A, A]] {
            let polygon = assert_some!(Polygon::from_vertices(vertices));
            assert_eq!(polygon.vertices(), &[A, B, C]);
            let coordinates = [A, B, C, A].map(LatLon::to_geojson_coordinate);
            assert_eq!(polygon.to_geojson_coordinates(), coordinates);
        }
    }
}
