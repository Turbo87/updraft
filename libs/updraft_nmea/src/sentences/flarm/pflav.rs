use crate::field::FieldsIter;

/// A FLARM version request or answer (`$PFLAV`).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Pflav {
    /// Requests the current version information (`R`).
    Request,
    /// Reports the current version information (`A`).
    Answer {
        /// Hardware version in the form reported by the device.
        hardware_version: Box<str>,
        /// Firmware version in the form reported by the device.
        firmware_version: Box<str>,
        /// Obstacle database version, or `None` when no database is present.
        obstacle_database_version: Option<Box<str>>,
    },
}

impl Pflav {
    pub fn parse(mut fields: FieldsIter<'_>) -> Option<Self> {
        match fields.bytes()? {
            b"R" if fields.next().is_none() => Some(Self::Request),
            b"A" => {
                let hardware_version = fields.text()?;
                let firmware_version = fields.text()?;
                let obstacle_database_version = fields.text();
                Some(Self::Answer {
                    hardware_version,
                    firmware_version,
                    obstacle_database_version,
                })
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some_eq};

    #[test]
    fn parses_a_complete_answer() {
        let pflav = Pflav::parse(FieldsIter::new(b"A,2.00,5.00,alps20110221_"));
        assert_some_eq!(
            pflav,
            Pflav::Answer {
                hardware_version: "2.00".into(),
                firmware_version: "5.00".into(),
                obstacle_database_version: Some("alps20110221_".into()),
            }
        );
    }

    #[test]
    fn parses_a_request() {
        let pflav = Pflav::parse(FieldsIter::new(b"R"));
        assert_some_eq!(pflav, Pflav::Request);
    }

    #[test]
    fn parses_an_answer_without_an_obstacle_database() {
        let pflav = Pflav::parse(FieldsIter::new(b"A,1.0,7.40,"));
        assert_some_eq!(
            pflav,
            Pflav::Answer {
                hardware_version: "1.0".into(),
                firmware_version: "7.40".into(),
                obstacle_database_version: None,
            }
        );
    }

    #[test]
    fn rejects_an_unknown_query_type() {
        assert_none!(Pflav::parse(FieldsIter::new(b"X")));
    }

    #[test]
    fn rejects_an_answer_without_required_versions() {
        assert_none!(Pflav::parse(FieldsIter::new(b"A,,7.40,")));
        assert_none!(Pflav::parse(FieldsIter::new(b"A,1.0,,")));
    }

    #[test]
    fn rejects_fields_after_a_request() {
        assert_none!(Pflav::parse(FieldsIter::new(b"R,1.0")));
    }

    #[test]
    fn ignores_fields_after_an_answer() {
        let pflav = Pflav::parse(FieldsIter::new(b"A,1.0,7.40,,extra"));
        assert_some_eq!(
            pflav,
            Pflav::Answer {
                hardware_version: "1.0".into(),
                firmware_version: "7.40".into(),
                obstacle_database_version: None,
            }
        );
    }
}
