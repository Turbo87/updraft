use super::{FlarmId, FlarmIdType};
use crate::field::{FieldsIter, hex_digit};

const MAX_IDENTITY_BYTES: usize = 17;

/// Periodic identity data received through FLARM Messaging (`$PFLAM,U`).
#[derive(Clone, Debug, PartialEq)]
pub struct Pflam {
    pub id_type: FlarmIdType,
    pub id: FlarmId,
    pub identity: PflamIdentity,
}

/// The identity field carried by a periodic FLARM Messaging sentence.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum PflamIdentity {
    Registration(Box<str>),
    PilotName(Box<str>),
    AircraftType(Box<str>),
    Callsign(Box<str>),
}

impl Pflam {
    pub fn parse(mut fields: FieldsIter<'_>) -> Option<Self> {
        (fields.bytes()? == b"U").then_some(())?;
        let id_type = FlarmIdType::from_field(fields.bytes()?)?;
        let id_field = fields.bytes()?;
        (id_field.len() == 6).then_some(())?;
        let id = FlarmId::parse(id_field)?;
        let message_type = fields.bytes()?;
        let value = decode_identity(fields.bytes()?)?;
        fields.bytes().is_none().then_some(())?;

        let identity = match message_type {
            b"AREG" => PflamIdentity::Registration(value),
            b"PNAME" => PflamIdentity::PilotName(value),
            b"ATYPE" => PflamIdentity::AircraftType(value),
            b"ACALL" => PflamIdentity::Callsign(value),
            _ => return None,
        };

        Some(Self {
            id_type,
            id,
            identity,
        })
    }
}

fn decode_identity(field: &[u8]) -> Option<Box<str>> {
    let (pairs, remainder) = field.as_chunks::<2>();
    (!pairs.is_empty() && remainder.is_empty() && pairs.len() <= MAX_IDENTITY_BYTES)
        .then_some(())?;

    let bytes = pairs
        .iter()
        .map(|pair| Some(hex_digit(pair[0])? << 4 | hex_digit(pair[1])?))
        .collect::<Option<Vec<_>>>()?;
    String::from_utf8(bytes).ok().map(String::into_boxed_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some};

    #[test]
    fn parses_all_periodic_identity_types() {
        let cases = [
            (
                b"U,2,DD8F12,AREG,442D4B58595A".as_slice(),
                FlarmIdType::Flarm,
                0xDD8F12,
                PflamIdentity::Registration("D-KXYZ".into()),
            ),
            (
                b"U,1,ABC123,PNAME,416461204C6F76656C616365".as_slice(),
                FlarmIdType::Icao,
                0xABC123,
                PflamIdentity::PilotName("Ada Lovelace".into()),
            ),
            (
                b"U,2,DD8F12,ATYPE,415357203237".as_slice(),
                FlarmIdType::Flarm,
                0xDD8F12,
                PflamIdentity::AircraftType("ASW 27".into()),
            ),
            (
                b"U,2,DD8F12,ACALL,58595A".as_slice(),
                FlarmIdType::Flarm,
                0xDD8F12,
                PflamIdentity::Callsign("XYZ".into()),
            ),
        ];

        for (fields, id_type, address, identity) in cases {
            let pflam = assert_some!(Pflam::parse(FieldsIter::new(fields)));
            assert_eq!(pflam.id_type, id_type);
            assert_eq!(pflam.id.address, address);
            assert_eq!(pflam.identity, identity);
        }
    }

    #[test]
    fn rejects_non_periodic_and_unsupported_messages() {
        assert_none!(Pflam::parse(FieldsIter::new(
            b"A,2,DD8F12,AREG,442D4B58595A"
        )));
        assert_none!(Pflam::parse(FieldsIter::new(b"U,2,DD8F12,VER,312E3030")));
    }

    #[test]
    fn rejects_invalid_target_ids() {
        assert_none!(Pflam::parse(FieldsIter::new(
            b"U,X,DD8F12,AREG,442D4B58595A"
        )));
        assert_none!(Pflam::parse(FieldsIter::new(
            b"U,2,D8F12,AREG,442D4B58595A"
        )));
        assert_none!(Pflam::parse(FieldsIter::new(
            b"U,2,INVALID,AREG,442D4B58595A"
        )));
    }

    #[test]
    fn rejects_invalid_identity_payloads() {
        assert_none!(Pflam::parse(FieldsIter::new(b"U,2,DD8F12,AREG,")));
        assert_none!(Pflam::parse(FieldsIter::new(b"U,2,DD8F12,AREG,4")));
        assert_none!(Pflam::parse(FieldsIter::new(b"U,2,DD8F12,AREG,GG")));
        assert_none!(Pflam::parse(FieldsIter::new(b"U,2,DD8F12,AREG,FF")));
        assert_none!(Pflam::parse(FieldsIter::new(
            b"U,2,DD8F12,AREG,313233343536373839303132333435363738"
        )));
        assert_none!(Pflam::parse(FieldsIter::new(
            b"U,2,DD8F12,AREG,58595A,EXTRA"
        )));
    }
}
