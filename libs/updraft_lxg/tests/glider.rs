//! High-level [`Glider`] extraction from, and encoding to, a real preset.

use updraft_lxg::{Arms, Ballast, Flap, Glider, Masses, Polar, Speeds};

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
fn writing_a_read_glider_round_trips() {
    // Read a real preset into the high-level view, write it back out, and
    // read it again: the high-level view survives a write/read cycle.
    let original = Glider::from_bytes(DG800B).unwrap();
    let reparsed = Glider::from_bytes(&original.to_bytes()).unwrap();
    assert_eq!(original, reparsed);
}

#[test]
fn building_a_glider_from_scratch_round_trips() {
    let glider = Glider {
        name: Some("Test Ship".to_owned()),
        description: None,
        competition_class: Some(2),
        polar: Polar {
            a: Some(1.0),
            b: Some(-2.0),
            c: Some(1.5),
            wing_area_m2: Some(10.5),
            reference_load_kg_m2: Some(35.0),
        },
        masses: Masses {
            empty: Some(250.0),
            reference: Some(300.0),
            max: Some(525.0),
            max_water_main: Some(150.0),
            ..Masses::default()
        },
        arms: Arms {
            empty: Some(600.0),
            pilot: Some(-500.0),
            ..Arms::default()
        },
        speeds: Speeds {
            stall: Some(20.0),
            never_exceed: Some(75.0),
            ..Speeds::default()
        },
        flaps: vec![
            Flap {
                label: "+8".to_owned(),
                max_speed: Some(25.0),
            },
            Flap {
                label: "0".to_owned(),
                max_speed: Some(40.0),
            },
        ],
        ballast: Ballast {
            wing_litres: vec![100.0, 30.0],
            tail_dump_rate: Some(0.5),
            ..Ballast::default()
        },
    };
    let reparsed = Glider::from_bytes(&glider.to_bytes()).unwrap();
    assert_eq!(glider, reparsed);
}

#[test]
fn edited_glider_reflects_the_change() {
    let mut glider = Glider::from_bytes(DG800B).unwrap();
    glider.masses.empty = Some(360.0);
    glider.polar.a = Some(1.10);

    let reparsed = Glider::from_bytes(&glider.to_bytes()).unwrap();
    assert_eq!(reparsed.masses.empty, Some(360.0));
    assert!((reparsed.polar.a.unwrap() - 1.10).abs() < 1e-6);
    assert_eq!(reparsed.name.as_deref(), Some("DG-800B"));
}

#[test]
fn missing_fields_are_none_not_errors() {
    // A minimal-but-valid file: version + empty root section.
    let bytes = [0xCC, 0xFF, 0xDE, 0x00, 0x00, 0xD5, 0x2F, 0x0A, 0x00, 0xC0];
    let g = Glider::from_bytes(&bytes).expect("empty file should decode");
    assert_eq!(g, Glider::default());
}
