use crate::LatLon;

/// One canonical exterior ring without a repeated closing vertex.
#[derive(Clone, Debug, PartialEq)]
pub struct Polygon {
    vertices: Vec<LatLon>,
}

impl Polygon {
    pub fn from_vertices(mut vertices: Vec<LatLon>) -> Self {
        while vertices.first() == vertices.last() {
            vertices.pop();
        }

        Self { vertices }
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
