use updraft_geo::LatLon;
use updraft_units::Length;

/// Fractions along a geodesic segment where it enters and exits a 500 m cylinder.
pub fn crossings(from: LatLon, to: LatLon, center: LatLon) -> (Option<f64>, Option<f64>) {
    let (length, bearing) = from.distance_bearing(to);
    if length.as_meters() < 0.001 {
        return (None, None);
    }
    let distance = |fraction: f64| {
        from.destination(bearing, Length::from_meters(length.as_meters() * fraction))
            .distance(center)
            .as_meters()
    };
    // GeographicLib can return a sub-micrometer error at an exact boundary.
    let from_inside = from.distance(center).as_meters() <= 500. + 1e-6;
    let to_inside = to.distance(center).as_meters() <= 500. + 1e-6;
    let (mut low, mut high) = (0., 1.);
    for _ in 0..40 {
        let a = low + (high - low) / 3.;
        let b = high - (high - low) / 3.;
        if distance(a) < distance(b) {
            high = b;
        } else {
            low = a;
        }
    }
    let closest = if from_inside {
        0.
    } else if to_inside {
        1.
    } else {
        (low + high) / 2.
    };
    if !from_inside && !to_inside && distance(closest) >= 500. {
        return (None, None);
    }
    let boundary = |mut outside: f64, mut inside: f64| {
        for _ in 0..40 {
            let middle = (outside + inside) / 2.;
            if distance(middle) > 500. {
                outside = middle;
            } else {
                inside = middle;
            }
        }
        (outside + inside) / 2.
    };
    let entry = (!from_inside).then(|| boundary(0., closest));
    let exit = (!to_inside).then(|| boundary(1., closest));
    (entry, exit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use updraft_units::Angle;

    #[test]
    fn pass_through_crosses_twice_on_the_ellipsoid_and_across_the_antimeridian() {
        for center in [
            LatLon::from_degrees(0., 0.),
            LatLon::from_degrees(50., 179.999),
            LatLon::from_degrees(89.999, 10.),
        ] {
            let from = center.destination(Angle::from_degrees(270.), Length::from_meters(600.));
            let to = center.destination(Angle::from_degrees(90.), Length::from_meters(600.));
            let (entry, exit) = crossings(from, to, center);
            assert_abs_diff_eq!(claims::assert_some!(entry), 1. / 12., epsilon = 1e-7);
            assert_abs_diff_eq!(claims::assert_some!(exit), 11. / 12., epsilon = 1e-7);
        }
    }

    #[test]
    fn stationary_and_distant_segments_do_not_cross() {
        let center = LatLon::from_degrees(0., 0.);
        assert_eq!(crossings(center, center, center), (None, None));
        assert_eq!(
            crossings(
                LatLon::from_degrees(1., -1.),
                LatLon::from_degrees(1., 1.),
                center
            ),
            (None, None)
        );
    }

    #[test]
    fn a_report_on_the_boundary_can_enter_or_leave() {
        let center = LatLon::from_degrees(0., 0.);
        let boundary = center.destination(Angle::from_degrees(90.), Length::from_meters(500.));
        let outside = center.destination(Angle::from_degrees(90.), Length::from_meters(600.));
        claims::assert_some!(crossings(boundary, outside, center).1);
        claims::assert_some!(crossings(outside, boundary, center).0);
    }
}
