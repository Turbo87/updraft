use crate::field::{FieldsIter, finite_f64, text};
use updraft_units::{Pressure, Speed};

/// `OpenVario` `$POV` sensor data or control traffic.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Pov {
    /// Sensor values carried by the sentence, in wire order.
    Data(Vec<PovDatum>),
    /// Requested setting names following the `?` field, kept in wire order.
    Query(Vec<Box<str>>),
    /// Command fields following the `C` field, kept in wire order.
    Command(Vec<Box<str>>),
    /// A payload that does not start with a known data or control field.
    Other(Vec<Box<str>>),
}

/// One typed sensor value from an `OpenVario` data sentence.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum PovDatum {
    /// True airspeed (`S`), transmitted in kilometres per hour.
    TrueAirspeed(Speed),
    /// Static pressure (`P`), transmitted in hectopascals.
    StaticPressure(Pressure),
    /// Dynamic pressure (`Q`), transmitted in pascals.
    DynamicPressure(Pressure),
    /// Total pressure (`R`), transmitted in hectopascals.
    TotalPressure(Pressure),
    /// Temperature (`T`) in degrees Celsius.
    Temperature(f64),
    /// Battery voltage (`V`) in volts.
    Voltage(f64),
    /// Total-energy vario (`E`), transmitted in metres per second.
    TotalEnergyVario(Speed),
    /// Relative humidity (`H`) as a percentage.
    RelativeHumidity(f64),
    /// Body-axis acceleration (`A`) in metres per second squared.
    Acceleration {
        /// Forward acceleration.
        x: f64,
        /// Rightward acceleration.
        y: f64,
        /// Downward acceleration.
        z: f64,
    },
    /// Body-axis angular rate (`G`) in degrees per second.
    AngularRate {
        /// Roll rate, positive with the left wing rising.
        x: f64,
        /// Pitch rate, positive with the nose rising.
        y: f64,
        /// Yaw rate, positive in a right turn.
        z: f64,
    },
    /// An unknown or malformed datum and every field following it.
    RawTail(Vec<Box<str>>),
}

impl Pov {
    pub fn parse(mut fields: FieldsIter<'_>) -> Self {
        let Some(first) = fields.next() else {
            return Self::Data(Vec::new());
        };
        if first == b"?" {
            return Self::Query(fields.map(text).collect());
        }
        if first == b"C" {
            return Self::Command(fields.map(text).collect());
        }
        if !matches!(
            first,
            b"S" | b"P" | b"Q" | b"R" | b"T" | b"V" | b"E" | b"H" | b"A" | b"G"
        ) {
            return Self::Other(std::iter::once(first).chain(fields).map(text).collect());
        }

        let mut data = Vec::new();
        let mut next = Some(first);

        while let Some(kind) = next {
            let datum = match kind {
                b"S" => scalar(kind, &mut fields)
                    .map(Speed::from_kilometers_per_hour)
                    .map(PovDatum::TrueAirspeed),
                b"P" => scalar(kind, &mut fields)
                    .map(Pressure::from_hectopascals)
                    .map(PovDatum::StaticPressure),
                b"Q" => scalar(kind, &mut fields)
                    .map(Pressure::from_pascals)
                    .map(PovDatum::DynamicPressure),
                b"R" => scalar(kind, &mut fields)
                    .map(Pressure::from_hectopascals)
                    .map(PovDatum::TotalPressure),
                b"T" => scalar(kind, &mut fields).map(PovDatum::Temperature),
                b"V" => scalar(kind, &mut fields).map(PovDatum::Voltage),
                b"E" => scalar(kind, &mut fields)
                    .map(Speed::from_meters_per_second)
                    .map(PovDatum::TotalEnergyVario),
                b"H" => scalar(kind, &mut fields).map(PovDatum::RelativeHumidity),
                b"A" => {
                    vector(kind, &mut fields).map(|[x, y, z]| PovDatum::Acceleration { x, y, z })
                }
                b"G" => {
                    vector(kind, &mut fields).map(|[x, y, z]| PovDatum::AngularRate { x, y, z })
                }
                _ => Err(raw_tail(kind, &[], fields.clone())),
            };

            match datum {
                Ok(datum) => {
                    data.push(datum);
                    next = fields.next();
                }
                Err(tail) => {
                    data.push(PovDatum::RawTail(tail));
                    break;
                }
            }
        }

        Self::Data(data)
    }
}

fn scalar<'a>(kind: &'a [u8], fields: &mut FieldsIter<'a>) -> Result<f64, Vec<Box<str>>> {
    let raw = fields.next();
    raw.and_then(finite_f64)
        .ok_or_else(|| raw_tail(kind, &[raw], fields.clone()))
}

fn vector<'a>(kind: &'a [u8], fields: &mut FieldsIter<'a>) -> Result<[f64; 3], Vec<Box<str>>> {
    let raw = [fields.next(), fields.next(), fields.next()];
    match raw.map(|field| field.and_then(finite_f64)) {
        [Some(x), Some(y), Some(z)] => Ok([x, y, z]),
        _ => Err(raw_tail(kind, &raw, fields.clone())),
    }
}

fn raw_tail<'a>(
    kind: &'a [u8],
    consumed: &[Option<&'a [u8]>],
    fields: FieldsIter<'a>,
) -> Vec<Box<str>> {
    std::iter::once(kind)
        .chain(consumed.iter().filter_map(|field| *field))
        .chain(fields)
        .map(text)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_scalar_data_in_wire_order() {
        let pov = Pov::parse(FieldsIter::new(
            b"S,123.45,P,1018.35,Q,23.3,R,1025.17,T,23.52,V,11.99,E,2.15,H,58.42",
        ));

        insta::assert_debug_snapshot!(pov);
    }

    #[test]
    fn parses_body_axis_vectors() {
        let pov = Pov::parse(FieldsIter::new(
            b"A,-1.5099,-0.0292,13.7134,G,4.165,-8.709,-10.479",
        ));

        insta::assert_debug_snapshot!(pov);
    }

    #[test]
    fn preserves_an_unknown_data_tail() {
        let pov = Pov::parse(FieldsIter::new(b"P,1013.25,X,1,2,E,3"));

        assert_eq!(
            pov,
            Pov::Data(vec![
                PovDatum::StaticPressure(Pressure::from_hectopascals(1013.25)),
                PovDatum::RawTail(vec![
                    "X".into(),
                    "1".into(),
                    "2".into(),
                    "E".into(),
                    "3".into(),
                ]),
            ])
        );
    }

    #[test]
    fn preserves_a_malformed_known_data_tail() {
        let pov = Pov::parse(FieldsIter::new(b"A,1,nope,3,E,2"));

        assert_eq!(
            pov,
            Pov::Data(vec![PovDatum::RawTail(vec![
                "A".into(),
                "1".into(),
                "nope".into(),
                "3".into(),
                "E".into(),
                "2".into(),
            ])])
        );
    }

    #[test]
    fn preserves_invalid_or_incomplete_data_tails() {
        for input in [
            "T,nan,E,2",
            "T,inf,E,2",
            "T,-inf,E,2",
            "T,nope,E,2",
            "T,,E,2",
            "T",
            "A,nan,2,3,E,2",
            "A,1,inf,3,E,2",
            "A,1,2,-inf,E,2",
            "A,1,,3,E,2",
            "A,1,2",
        ] {
            let pov = Pov::parse(FieldsIter::new(input.as_bytes()));
            let tail = input.split(',').map(Box::from).collect();
            assert_eq!(pov, Pov::Data(vec![PovDatum::RawTail(tail)]));
        }
    }

    #[test]
    fn preserves_repeated_data_in_wire_order() {
        let pov = Pov::parse(FieldsIter::new(b"E,1,E,2"));

        assert_eq!(
            pov,
            Pov::Data(vec![
                PovDatum::TotalEnergyVario(Speed::from_meters_per_second(1.)),
                PovDatum::TotalEnergyVario(Speed::from_meters_per_second(2.)),
            ])
        );
    }

    #[test]
    fn preserves_query_fields() {
        let pov = Pov::parse(FieldsIter::new(b"?,RPO,,MC"));

        assert_eq!(pov, Pov::Query(vec!["RPO".into(), "".into(), "MC".into()]));
    }

    #[test]
    fn preserves_command_fields() {
        let pov = Pov::parse(FieldsIter::new(b"C,MC,,+0.5"));

        assert_eq!(
            pov,
            Pov::Command(vec!["MC".into(), "".into(), "+0.5".into()])
        );
    }

    #[test]
    fn preserves_other_openvario_traffic() {
        let pov = Pov::parse(FieldsIter::new(b"Z,foo,,bar"));

        assert_eq!(
            pov,
            Pov::Other(vec!["Z".into(), "foo".into(), "".into(), "bar".into()])
        );
    }
}
