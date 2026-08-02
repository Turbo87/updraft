//! NMEA sentence encoding shared by typed sentence implementations.

use crate::Talker;
use thiserror::Error;
use updraft_geo::LatLon;

/// An NMEA field cannot be represented in a sentence.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum EncodeError {
    /// The named typed field has no valid NMEA representation.
    #[error("NMEA field cannot be encoded: {0}")]
    InvalidField(&'static str),
}

pub struct SentenceEncoder {
    body: String,
}

impl SentenceEncoder {
    pub fn new(identifier: &str) -> Self {
        Self {
            body: identifier.to_owned(),
        }
    }

    pub fn field(&mut self, value: &str) {
        self.body.push(',');
        self.body.push_str(value);
    }

    pub fn text_field(
        &mut self,
        value: Option<&str>,
        field: &'static str,
    ) -> Result<(), EncodeError> {
        let value = value.unwrap_or_default();
        if !value.is_ascii()
            || value
                .bytes()
                .any(|byte| matches!(byte, b',' | b'*' | b'\r' | b'\n'))
        {
            return Err(EncodeError::InvalidField(field));
        }
        self.field(value);
        Ok(())
    }

    pub fn finish(self) -> Vec<u8> {
        let checksum = self.body.bytes().fold(0, |checksum, byte| checksum ^ byte);
        format!("${}*{checksum:02X}\r\n", self.body).into_bytes()
    }
}

pub fn talker_code(talker: &Talker) -> Result<&str, EncodeError> {
    let code = match talker {
        Talker::Gps => "GP",
        Talker::Glonass => "GL",
        Talker::Galileo => "GA",
        Talker::BeiDou => "GB",
        Talker::Qzss => "GQ",
        Talker::Combined => "GN",
        Talker::Other(code)
            if matches!(code.as_bytes(), [first, second]
                if first.is_ascii_uppercase() && second.is_ascii_uppercase()) =>
        {
            code
        }
        Talker::Other(_) => return Err(EncodeError::InvalidField("talker")),
    };
    Ok(code)
}

pub fn optional_field<T: ToString>(value: Option<T>) -> String {
    value.map_or_else(String::new, |value| value.to_string())
}

pub fn position_fields(position: Option<LatLon>) -> [String; 4] {
    let Some(position) = position else {
        return std::array::from_fn(|_| String::new());
    };

    let latitude = position.latitude().as_degrees();
    let longitude = position.longitude().as_degrees();
    let latitude_value = coordinate_field(latitude, 2);
    let longitude_value = coordinate_field(longitude, 3);
    let latitude_hemisphere = if latitude.is_sign_negative() {
        "S"
    } else {
        "N"
    };
    let longitude_hemisphere = if longitude.is_sign_negative() {
        "W"
    } else {
        "E"
    };

    [
        latitude_value,
        latitude_hemisphere.to_owned(),
        longitude_value,
        longitude_hemisphere.to_owned(),
    ]
}

fn coordinate_field(degrees: f64, degree_width: usize) -> String {
    let total_minutes = (degrees.abs() * 60.0 * 100_000.0).round() / 100_000.0;
    let whole_degrees = (total_minutes / 60.0).trunc();
    let minutes = total_minutes - whole_degrees * 60.0;
    format!("{:0degree_width$}{minutes:08.5}", whole_degrees as u16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carries_rounded_coordinate_minutes_into_degrees() {
        assert_eq!(coordinate_field(89.999_999_999, 2), "9000.00000");
    }
}
