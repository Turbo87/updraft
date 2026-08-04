use claims::{assert_none, assert_some};
use updraft_airspace::AirspaceFrequencyValue;

/// Verifies that frequency values use the OpenAIP display format.
#[test]
fn formats_openaip_frequency_values() {
    let value = assert_some!(AirspaceFrequencyValue::from_megahertz("123.45"));

    assert_eq!(value.to_string(), "123.450");
}

/// Verifies that frequency values reject excess precision.
#[test]
fn rejects_excess_frequency_precision() {
    assert_none!(AirspaceFrequencyValue::from_megahertz("123.4567"));
}
