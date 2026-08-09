use claims::{assert_none, assert_some};
use updraft_waypoint::FrequencyValue;

/// Verifies that frequency values pad the source precision to three digits.
#[test]
fn formats_frequency_values_with_three_decimal_digits() {
    let value = assert_some!(FrequencyValue::from_decimal("123.45"));

    assert_eq!(value.to_string(), "123.450");
}

/// Verifies that a value without a decimal point keeps its whole number.
#[test]
fn formats_whole_frequency_values() {
    let value = assert_some!(FrequencyValue::from_decimal("335"));

    assert_eq!(value.to_string(), "335.000");
}

/// Verifies that frequency values reject excess precision.
#[test]
fn rejects_excess_frequency_precision() {
    assert_none!(FrequencyValue::from_decimal("123.4567"));
}

/// Verifies that frequency values reject a trailing decimal point.
#[test]
fn rejects_trailing_decimal_point() {
    assert_none!(FrequencyValue::from_decimal("123."));
}
