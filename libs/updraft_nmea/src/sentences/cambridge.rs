use crate::field::FieldsIter;
use updraft_units::{Angle, Length, Pressure, Speed};

/// Cambridge `!w` flight data.
#[derive(Clone, Debug, PartialEq)]
pub struct CaiW {
    /// Direction of the wind vector.
    pub wind_vector_direction: Option<Angle>,
    /// Wind speed.
    pub wind_speed: Option<Speed>,
    /// Age of the wind solution in seconds.
    pub wind_age_seconds: Option<u32>,
    /// Along-track wind component, positive for a tailwind.
    pub tailwind_component: Option<Speed>,
    /// Altitude corrected for the transmitted QNH.
    pub true_altitude: Option<Length>,
    /// Altimeter pressure setting.
    pub qnh: Option<Pressure>,
    /// True airspeed.
    pub true_airspeed: Option<Speed>,
    /// Instantaneous total-energy vario.
    pub vario: Option<Speed>,
    /// Averaged total-energy vario.
    pub average_vario: Option<Speed>,
    /// Relative vario.
    pub relative_vario: Option<Speed>,
    /// MacCready setting.
    pub mac_cready: Option<Speed>,
    /// Water-ballast fraction.
    pub ballast: Option<f64>,
    /// Glider performance fraction after bug degradation.
    pub bugs: Option<f64>,
}

impl CaiW {
    pub fn parse(mut fields: FieldsIter<'_>) -> Self {
        Self {
            wind_vector_direction: fields.f64().map(Angle::from_degrees),
            wind_speed: fields
                .f64()
                .map(|value| Speed::from_meters_per_second(value / 10.0)),
            wind_age_seconds: fields
                .bytes()
                .and_then(|field| btoi::btou::<u32>(field).ok()),
            tailwind_component: fields
                .f64()
                .map(|value| Speed::from_meters_per_second((500.0 - value) / 10.0)),
            true_altitude: fields
                .f64()
                .map(|value| Length::from_meters(value - 1000.0)),
            qnh: fields.f64().map(Pressure::from_hectopascals),
            true_airspeed: fields
                .f64()
                .map(|value| Speed::from_meters_per_second(value / 100.0)),
            vario: vario(&mut fields),
            average_vario: vario(&mut fields),
            relative_vario: vario(&mut fields),
            mac_cready: fields.f64().map(|value| Speed::from_knots(value / 10.0)),
            ballast: fields.f64().map(|value| value / 100.0),
            bugs: fields.f64().map(|value| value / 100.0),
        }
    }
}

fn vario(fields: &mut FieldsIter<'_>) -> Option<Speed> {
    fields
        .f64()
        .map(|value| Speed::from_knots((value - 200.0) / 10.0))
}
