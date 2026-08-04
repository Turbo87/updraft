//! Canonical airspace data.

mod openair;
mod operating_hours;
mod types;

pub use operating_hours::{
    AirspaceOperatingHours, AirspaceOperatingPeriod, AirspaceOperatingSchedule,
};
pub use types::{
    Airspace, AirspaceActivity, AirspaceAltitude, AirspaceClass, AirspaceDataset,
    AirspaceGeometryError, AirspaceId, AirspaceImportError, AirspaceParseError, AirspaceType,
};
