//! `$LXWP1` — `LXNav` device identification.

use crate::error::ParseError;
use crate::fields::Fields;

/// A parsed `$LXWP1` sentence: the instrument's identity strings.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Lxwp1 {
    /// Instrument product name (e.g. `LX5000IGC`).
    pub product: Option<String>,
    /// Instrument serial number.
    pub serial: Option<String>,
    /// Firmware / software version.
    pub software_version: Option<String>,
    /// Hardware version.
    pub hardware_version: Option<String>,
    /// License string, when present.
    pub license: Option<String>,
}

fn owned(field: &str) -> Option<String> {
    (!field.is_empty()).then(|| field.to_owned())
}

pub(crate) fn parse(fields: Fields<'_>) -> Result<Lxwp1, ParseError> {
    let fields: Vec<&str> = fields.collect();
    let get = |index: usize| fields.get(index).copied().unwrap_or("");
    Ok(Lxwp1 {
        product: owned(get(0)),
        serial: owned(get(1)),
        software_version: owned(get(2)),
        hardware_version: owned(get(3)),
        license: owned(get(4)),
    })
}

#[cfg(test)]
mod tests {
    use crate::{ParseResult, parse};

    #[test]
    fn device_identity() {
        let ParseResult::Lxwp1(lxwp1) = parse("$LXWP1,LX5000IGC,12345,9.6,1.4,*69").unwrap() else {
            panic!("expected LXWP1");
        };
        assert_eq!(lxwp1.product.as_deref(), Some("LX5000IGC"));
        assert_eq!(lxwp1.serial.as_deref(), Some("12345"));
        assert_eq!(lxwp1.software_version.as_deref(), Some("9.6"));
        assert_eq!(lxwp1.hardware_version.as_deref(), Some("1.4"));
        // Trailing license field is empty.
        assert_eq!(lxwp1.license, None);
    }
}
