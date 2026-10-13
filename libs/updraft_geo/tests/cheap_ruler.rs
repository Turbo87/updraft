//! Tests of the `CheapRuler` line methods with the upstream cheap-ruler
//! fixtures and test cases.

use approx::assert_relative_eq;
use claims::{assert_le, assert_none, assert_some, assert_some_eq};
use serde_json::json;
use updraft_geo::{CheapRuler, LatLon, PointOnLine};
use updraft_units::{Angle, Length};

const LINES: &str = include_str!("fixtures/cheap_ruler/lines.json");

fn lines() -> Vec<Vec<LatLon>> {
    let lines: Vec<Vec<[f64; 2]>> = serde_json::from_str(LINES).unwrap();
    lines
        .into_iter()
        .map(|line| line.into_iter().map(point).collect())
        .collect()
}

/// A point from upstream `[longitude, latitude]` degrees.
fn point([longitude, latitude]: [f64; 2]) -> LatLon {
    LatLon::from_degrees(latitude, longitude)
}

/// The ruler of the upstream tests.
fn ruler() -> CheapRuler {
    CheapRuler::new(Angle::from_degrees(32.8351))
}

fn round(value: f64, decimals: i32) -> f64 {
    let factor = 10_f64.powi(decimals);
    (value * factor).round() / factor
}

#[test]
fn line_distance_matches_geodesic() {
    let ruler = ruler();
    for line in lines() {
        let expected = line
            .windows(2)
            .fold(Length::ZERO, |sum, pair| sum + pair[0].distance(pair[1]));
        let error = ((ruler.line_distance(&line) - expected) / expected).abs();
        assert_le!(error, 0.001);
    }
}

#[test]
fn fixture_lines() {
    let ruler = ruler();
    let results: Vec<_> = lines()
        .iter()
        .map(|line| {
            let length = ruler.line_distance(line);
            let middle = assert_some!(ruler.along(line, length / 2.));
            json!({
                "length_m": round(length.as_meters(), 3),
                "middle": [
                    round(middle.longitude().as_degrees(), 9),
                    round(middle.latitude().as_degrees(), 9),
                ],
            })
        })
        .collect();
    insta::assert_json_snapshot!(results);
}

#[test]
fn slices_have_the_requested_length() {
    let ruler = ruler();
    for line in lines() {
        let length = ruler.line_distance(&line);
        let expected = length * 0.4;

        let slice = ruler.line_slice_along(&line, length * 0.3, length * 0.7);
        assert_relative_eq!(ruler.line_distance(&slice), expected, max_relative = 1e-9);

        let start = assert_some!(ruler.along(&line, length * 0.3));
        let stop = assert_some!(ruler.along(&line, length * 0.7));
        let slice = ruler.line_slice(&line, start, stop);
        assert_relative_eq!(ruler.line_distance(&slice), expected, max_relative = 1e-9);
    }
}

#[test]
fn along_clamps_to_the_line() {
    let ruler = ruler();
    let line = &lines()[0];
    assert_some_eq!(ruler.along(line, Length::from_meters(-5.)), line[0]);
    assert_some_eq!(
        ruler.along(line, Length::from_kilometers(1000.)),
        line[line.len() - 1]
    );
}

#[test]
fn along_across_antimeridian() {
    let ruler = ruler();
    let line = [point([179.9, 32.7]), point([-179.9, 32.9])];
    let distance = line[0].distance(line[1]) / 3.;
    let actual = assert_some!(ruler.along(&line, distance));
    let expected = line[0].destination(line[0].bearing(line[1]), distance);
    assert_le!(expected.distance(actual), Length::from_meters(20.));
}

#[test]
fn point_on_line() {
    let ruler = ruler();
    let line = [
        point([-77.031669, 38.878605]),
        point([-77.029609, 38.881946]),
    ];
    let p = point([-77.034076, 38.882017]);

    let result = assert_some!(ruler.point_on_line(&line, p));
    assert_relative_eq!(
        result.point,
        point([-77.03052689033436, 38.880457324462576]),
        max_relative = 1e-12
    );
    assert_eq!(result.index, 0);
    assert_relative_eq!(result.t, 0.5544221677861756, max_relative = 1e-12);
    assert_relative_eq!(
        result.distance,
        Length::from_kilometers(0.37461484020420416),
        max_relative = 1e-12
    );
    assert_eq!(result.distance, ruler.distance(p, result.point));

    let before = assert_some!(ruler.point_on_line(&line, point([-80., 38.])));
    assert_eq!(before.t, 0.);
    let after = assert_some!(ruler.point_on_line(&line, point([-75., 38.])));
    assert_eq!(after.t, 1.);
}

#[test]
fn point_on_line_across_antimeridian() {
    let ruler = ruler();
    let line = [point([179.9, 32.7]), point([-179.9, 32.9])];
    let result = assert_some!(ruler.point_on_line(&line, point([180., 32.7])));
    assert_relative_eq!(
        result.point,
        point([179.9416136283502, 32.7416136283502]),
        max_relative = 1e-12
    );
}

#[test]
fn point_on_line_with_one_point() {
    let ruler = ruler();
    let line = [point([30.5, 32.5])];
    let p = point([30.6, 32.6]);
    assert_some_eq!(
        ruler.point_on_line(&line, p),
        PointOnLine {
            point: line[0],
            index: 0,
            t: 0.,
            distance: ruler.distance(p, line[0]),
        }
    );
}

#[test]
fn point_to_segment_distance() {
    let ruler = ruler();
    let distance = ruler.point_to_segment_distance(
        point([-77.034076, 38.882017]),
        point([-77.031669, 38.878605]),
        point([-77.029609, 38.881946]),
    );
    assert_relative_eq!(
        distance,
        Length::from_kilometers(0.37461484020420416),
        max_relative = 1e-12
    );
}

#[test]
fn line_slice_in_reverse_order() {
    let ruler = ruler();
    let line = &lines()[0];
    let length = ruler.line_distance(line);
    let start = assert_some!(ruler.along(line, length * 0.7));
    let stop = assert_some!(ruler.along(line, length * 0.3));
    let slice = ruler.line_slice(line, start, stop);
    assert_relative_eq!(
        ruler.line_distance(&slice),
        Length::from_kilometers(0.018676476689649835),
        max_relative = 1e-9
    );
}

#[test]
fn line_slice_with_one_point() {
    let ruler = ruler();
    let line = [point([30.5, 50.5])];
    let slice = ruler.line_slice(&line, point([30., 50.]), point([31., 51.]));
    assert_eq!(slice, line);
}

#[test]
fn line_slice_along_with_negative_start() {
    let ruler = ruler();
    let line = &lines()[0];
    let length = ruler.line_distance(line);
    let slice = ruler.line_slice_along(line, Length::from_meters(-1000.), length * 0.5);
    assert_eq!(slice[0], line[0]);
}

#[test]
fn line_slice_along_with_zero_stop_and_duplicate_first_point() {
    let ruler = ruler();
    let line = [point([30., 50.]), point([30., 50.]), point([30.1, 50.1])];
    let slice = ruler.line_slice_along(&line, Length::ZERO, Length::ZERO);
    assert_eq!(slice, [line[0]]);
}

#[test]
fn line_slice_along_with_start_after_stop() {
    let ruler = ruler();
    let line = &lines()[0];
    let length = ruler.line_distance(line);
    assert_eq!(
        ruler.line_slice_along(line, length * 0.7, length * 0.3),
        ruler.line_slice_along(line, length * 0.3, length * 0.7)
    );
}

#[test]
fn empty_line() {
    let ruler = ruler();
    let p = point([30., 50.]);
    assert_eq!(ruler.line_distance(&[]), Length::ZERO);
    assert_none!(ruler.along(&[], Length::ZERO));
    assert_none!(ruler.point_on_line(&[], p));
    assert_eq!(ruler.line_slice(&[], p, p), []);
    assert_eq!(
        ruler.line_slice_along(&[], Length::ZERO, Length::from_meters(1.)),
        []
    );
}
