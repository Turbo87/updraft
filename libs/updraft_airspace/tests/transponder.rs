use claims::{assert_none, assert_some};
use updraft_airspace::AirspaceTransponderCode;

/// Verifies that transponder codes preserve all four octal digits.
#[test]
fn formats_four_digit_octal_codes() {
    let code = assert_some!(AirspaceTransponderCode::from_octal_digits(123));

    assert_eq!(code.to_string(), "0123");
}

/// Verifies that invalid digits and values longer than four digits are rejected.
#[test]
fn rejects_invalid_transponder_codes() {
    for code in [1289, 10_000] {
        assert_none!(AirspaceTransponderCode::from_octal_digits(code), "{code}");
    }
}
