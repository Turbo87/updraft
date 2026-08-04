//! Canonical airspace data.

mod frequency;
mod openair;
mod operating_hours;
mod transponder;
mod types;

pub use frequency::{AirspaceFrequency, AirspaceFrequencyUnit, AirspaceFrequencyValue};
pub use operating_hours::{
    AirspaceOperatingHours, AirspaceOperatingPeriod, AirspaceOperatingSchedule,
};
pub use transponder::{AirspaceTransponderCode, AirspaceTransponderSetting};
pub use types::{
    Airspace, AirspaceActivity, AirspaceAltitude, AirspaceClass, AirspaceDataset,
    AirspaceGeometryError, AirspaceId, AirspaceImportError, AirspaceParseError, AirspaceType,
};
