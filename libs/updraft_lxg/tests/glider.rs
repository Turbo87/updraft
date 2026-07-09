//! High-level [`Glider`] extraction from a real preset.

use updraft_lxg::Glider;

const DG800B: &[u8] = include_bytes!("fixtures/dg-800b.lxg");

/// Asserts `actual` is within a small tolerance of `expected` (values that
/// pass through `f32` storage are not exactly representable as `f64`).
#[track_caller]
fn close(actual: Option<f64>, expected: f64) {
    let actual = actual.expect("field should be present");
    assert!(
        (actual - expected).abs() < 1e-3,
        "expected ~{expected}, got {actual}",
    );
}

#[test]
fn glider_snapshot() {
    let glider = Glider::from_bytes(DG800B).expect("fixture should decode");
    insta::assert_debug_snapshot!(glider);
}

#[test]
fn identity_and_polar() {
    let g = Glider::from_bytes(DG800B).unwrap();
    assert_eq!(g.name.as_deref(), Some("DG-800B"));
    assert_eq!(g.competition_class, Some(4));
    close(g.polar.a, 1.05);
    close(g.polar.b, -1.42);
    close(g.polar.c, 0.93);
    close(g.polar.wing_area_m2, 11.81);
}

#[test]
fn masses_and_arms() {
    let g = Glider::from_bytes(DG800B).unwrap();
    close(g.masses.empty, 352.0);
    close(g.masses.reference, 402.0);
    close(g.masses.max, 525.0);
    close(g.masses.max_pilot, 103.0);
    close(g.masses.max_water_main, 100.0);
    // Negative = forward of the datum.
    close(g.arms.empty, 560.0);
    close(g.arms.pilot, -550.0);
    close(g.arms.water_main, 171.0);
}

#[test]
fn speeds_are_in_meters_per_second() {
    let g = Glider::from_bytes(DG800B).unwrap();
    // Vne is 270 km/h -> 75 m/s.
    close(g.speeds.never_exceed, 75.0);
}

#[test]
fn flaps_pair_labels_with_speeds() {
    let g = Glider::from_bytes(DG800B).unwrap();
    let labels: Vec<_> = g.flaps.iter().map(|f| f.label.as_str()).collect();
    assert_eq!(labels, ["+8", "+5", "0", "-5", "-10", "-14"]);
    // First flap position tops out at 76 km/h -> ~21.1 m/s.
    close(g.flaps[0].max_speed, 76.0 / 3.6);
}

#[test]
fn missing_fields_are_none_not_errors() {
    // A minimal-but-valid file: version + empty root section.
    let bytes = [0xCC, 0xFF, 0xDE, 0x00, 0x00, 0xD5, 0x2F, 0x0A, 0x00, 0xC0];
    let g = Glider::from_bytes(&bytes).expect("empty file should decode");
    assert_eq!(g, Glider::default());
}
