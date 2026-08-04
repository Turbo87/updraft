use claims::assert_none;
use updraft_airspace::AirspaceOperatingHours;

/// Verifies that operating hours require at least one period.
#[test]
fn rejects_empty_operating_hours() {
    assert_none!(AirspaceOperatingHours::new(Vec::new(), None));
}
