//! Geoid undulation lookup over the 1° grid embedded in the binary.

use updraft_geo::LatLon;
use updraft_units::{EllipsoidAltitude, Length, MslAltitude};

/// Embedded 1° geoid-undulation grid: signed whole-metre samples of the
/// EGM96 geoid height above the WGS84 ellipsoid, row-major, [`ROWS`] rows
/// (north→south) × [`COLS`] columns (west→east). Bytes are `i8`.
const GRID: &[u8] = include_bytes!("../data/egm96_1deg.bin");

/// Grid rows: latitudes 90°N (row 0) … 90°S (row 180) in 1° steps.
const ROWS: usize = 181;
/// Grid columns: longitudes 0°E (col 0) … 359°E (col 359) in 1° steps.
const COLS: usize = 360;

const _: () = assert!(GRID.len() == ROWS * COLS);

/// Undulation at grid cell `(row, col)`, in metres. Indices must be in
/// range (`row < ROWS`, `col < COLS`).
#[inline]
fn cell(row: usize, col: usize) -> f64 {
    assert!(row < ROWS && col < COLS);
    f64::from(GRID[row * COLS + col] as i8)
}

/// The EGM96 geoid undulation *N* at `position` (the height of the geoid
/// above the WGS84 ellipsoid) bilinearly interpolated from the embedded
/// 1° grid.
#[inline]
pub fn undulation(position: LatLon) -> Length {
    let lat = position.latitude().as_degrees();
    let lon = position.longitude().as_degrees();

    // Casting NaN to an integer index would silently saturate to 0 rather
    // than propagate, so reject it up front to keep garbage detectable.
    if lat.is_nan() || lon.is_nan() {
        return Length::from_meters(f64::NAN);
    }

    // Latitude clamps to the poles; longitude wraps into [0, 360). The
    // wrap is spelled `lon − 360·⌊lon/360⌋` rather than
    // `lon.rem_euclid(360.0)`: it is equivalent for the finite inputs that
    // reach here (NaN was rejected above) but a few nanoseconds cheaper,
    // and the `% COLS` below already absorbs the boundary case where the
    // wrapped value lands on exactly 360.
    let lat = lat.clamp(-90.0, 90.0);
    let lon = lon - 360.0 * (lon / 360.0).floor();

    // Row grows southward from 90°N; column grows eastward from 0°E, one
    // degree per step. Each floor feeds both a grid index and its
    // fractional weight, so compute it once.
    let row = 90.0 - lat; // [0, 180]
    let row_floor = row.floor();
    let r0 = row_floor as usize; // 0..=180
    let r1 = (r0 + 1).min(ROWS - 1); // clamp at the south pole
    let fy = row - row_floor;

    let lon_floor = lon.floor();
    let c0 = lon_floor as usize % COLS;
    let c1 = (c0 + 1) % COLS; // wraps 359 → 0 across the antimeridian
    let fx = lon - lon_floor;

    let top = cell(r0, c0) * (1.0 - fx) + cell(r0, c1) * fx;
    let bottom = cell(r1, c0) * (1.0 - fx) + cell(r1, c1) * fx;
    Length::from_meters(top * (1.0 - fy) + bottom * fy)
}

/// Convert a WGS84-ellipsoidal height to MSL (orthometric) height at
/// `position`: `msl = ellipsoidal − N`.
pub fn ellipsoidal_to_msl(position: LatLon, ellipsoidal: EllipsoidAltitude) -> MslAltitude {
    MslAltitude::new(ellipsoidal.into_inner() - undulation(position))
}

/// Convert an MSL (orthometric) height to WGS84-ellipsoidal height at
/// `position`: `ellipsoidal = msl + N`.
pub fn msl_to_ellipsoidal(position: LatLon, msl: MslAltitude) -> EllipsoidAltitude {
    EllipsoidAltitude::new(msl.into_inner() + undulation(position))
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn n(lat: f64, lon: f64) -> f64 {
        undulation(LatLon::from_degrees(lat, lon)).as_meters()
    }

    #[test]
    fn known_nodes_match_the_grid() {
        // Exact integer-degree points hit a grid node directly (no
        // interpolation); values are the rounded EGM96 undulation in
        // metres from the official 15′ source.
        assert_relative_eq!(n(0.0, 0.0), 17.0);
        assert_relative_eq!(n(52.0, 7.0), 44.0); // Germany
        assert_relative_eq!(n(37.0, -122.0), -33.0); // California
        assert_relative_eq!(n(7.0, 80.0), -97.0); // Sri Lanka geoid low
    }

    #[test]
    fn bilinear_interpolates_between_nodes() {
        // Along latitude 52° the nodes at 12°E and 13°E are 43 m and 42 m;
        // the midpoint must be their average.
        assert_relative_eq!(n(52.0, 12.0), 43.0);
        assert_relative_eq!(n(52.0, 13.0), 42.0);
        assert_relative_eq!(n(52.0, 12.5), 42.5);
    }

    #[test]
    fn longitude_wraps() {
        assert_relative_eq!(n(52.0, 7.0), n(52.0, 367.0));
        assert_relative_eq!(n(52.0, 7.0), n(52.0, -353.0));
        // A query just past the last column interpolates back to 0°E.
        assert_relative_eq!(n(0.0, 359.5), (n(0.0, 359.0) + n(0.0, 0.0)) / 2.0);
    }

    #[test]
    fn latitude_clamps_to_the_poles() {
        assert_relative_eq!(n(90.0, 0.0), 14.0);
        assert_relative_eq!(n(-90.0, 0.0), -30.0);
        // Beyond the poles clamps rather than indexing out of range.
        assert_relative_eq!(n(95.0, 7.0), n(90.0, 7.0));
        assert_relative_eq!(n(-95.0, 7.0), n(-90.0, 7.0));
    }

    #[test]
    fn nan_in_nan_out() {
        assert!(n(f64::NAN, 7.0).is_nan());
        assert!(n(52.0, f64::NAN).is_nan());
    }

    #[test]
    fn conversions_are_inverse() {
        let pos = LatLon::from_degrees(52.0, 7.0);
        let ellipsoidal = EllipsoidAltitude::new(Length::from_meters(1000.0));
        let msl = ellipsoidal_to_msl(pos, ellipsoidal);

        // Germany's undulation is positive, so MSL sits below ellipsoidal.
        assert_relative_eq!(msl.into_inner(), Length::from_meters(956.0));
        assert_relative_eq!(msl_to_ellipsoidal(pos, msl), ellipsoidal);
    }
}
