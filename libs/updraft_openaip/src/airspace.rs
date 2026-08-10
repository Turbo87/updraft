//! The `asp` dataset.

use crate::code::codes;
use crate::common::{Countries, FrequencyUnit, HoursOfOperation, Polygon, VerticalLimit};
use serde::Deserialize;

/// One airspace record.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Airspace {
    #[serde(rename = "_id")]
    pub id: Box<str>,
    pub name: Box<str>,
    pub r#type: AirspaceType,
    pub icao_class: IcaoClass,
    pub activity: Activity,
    pub country: Countries,
    pub geometry: Polygon,
    pub lower_limit: VerticalLimit,
    pub upper_limit: VerticalLimit,
    /// The lowest permitted lower limit of a floating airspace.
    pub lower_limit_min: Option<VerticalLimit>,
    /// The highest permitted upper limit of a floating airspace.
    pub upper_limit_max: Option<VerticalLimit>,
    /// Activation depends on a request to the controlling authority.
    pub on_request: bool,
    /// Activation depends on demand.
    pub on_demand: bool,
    /// Activation depends on a NOTAM.
    pub by_notam: bool,
    /// Entry depends on a special agreement.
    pub special_agreement: bool,
    /// Entry depends on compliance with a request of the controlling authority.
    pub request_compliance: bool,
    pub hours_of_operation: HoursOfOperation,
    /// Start of a temporary activation period as an RFC 3339 timestamp.
    pub active_from: Option<Box<str>>,
    /// End of a temporary activation period as an RFC 3339 timestamp.
    pub active_until: Option<Box<str>>,
    #[serde(default)]
    pub frequencies: Box<[Frequency]>,
    #[serde(default)]
    pub transponder_settings: Box<[TransponderSetting]>,
    pub remarks: Option<Box<str>>,
    /// The record comes from an automated data ingestion.
    pub data_ingestion: bool,
    pub created_at: Box<str>,
    pub created_by: Box<str>,
    pub updated_at: Box<str>,
    pub updated_by: Box<str>,
}

/// A radio frequency of an airspace.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Frequency {
    #[serde(rename = "_id")]
    pub id: Box<str>,
    /// The frequency in the given unit, for example `123.625`.
    ///
    /// Two United Kingdom records carry a corrupt entry that holds no value.
    pub value: Option<Box<str>>,
    pub unit: FrequencyUnit,
    pub name: Option<Box<str>>,
    pub primary: bool,
    pub remarks: Option<Box<str>>,
}

/// A transponder code of an airspace.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransponderSetting {
    pub code: Box<str>,
    pub primary: bool,
    pub remarks: Option<Box<str>>,
}

codes! {
    /// The airspace type.
    pub enum AirspaceType {
        0 => Other,
        1 => Restricted,
        2 => Danger,
        3 => Prohibited,
        /// Controlled tower region.
        4 => Ctr,
        /// Transponder mandatory zone.
        5 => Tmz,
        /// Radio mandatory zone.
        6 => Rmz,
        /// Terminal manoeuvring area.
        7 => Tma,
        /// Temporary reserved area.
        8 => Tra,
        /// Temporary segregated area.
        9 => Tsa,
        /// Flight information region.
        10 => Fir,
        /// Upper flight information region.
        11 => Uir,
        /// Air defence identification zone.
        12 => Adiz,
        /// Airport traffic zone.
        13 => Atz,
        /// Military airport traffic zone.
        14 => Matz,
        15 => Airway,
        /// Military training route.
        16 => Mtr,
        17 => AlertArea,
        18 => WarningArea,
        19 => ProtectedArea,
        /// Helicopter traffic zone.
        20 => Htz,
        21 => GlidingSector,
        /// Transponder setting.
        22 => Trp,
        /// Traffic information zone.
        23 => Tiz,
        /// Traffic information area.
        24 => Tia,
        /// Military training area.
        25 => Mta,
        /// Control area.
        26 => Cta,
        /// Area control centre sector.
        27 => AccSector,
        28 => AerialSportingOrRecreationalActivity,
        29 => LowAltitudeOverflightRestriction,
        /// Military route.
        30 => Mrt,
        /// Temporary segregated or reserved area feeding route.
        31 => Tfr,
        /// Visual flight rules sector.
        32 => VfrSector,
        /// Flight information service sector.
        33 => FisSector,
        /// Lower traffic area.
        34 => Lta,
        /// Upper traffic area.
        35 => Uta,
        /// Military controlled tower region.
        36 => Mctr,
    }
}

codes! {
    /// The ICAO airspace class.
    pub enum IcaoClass {
        0 => A,
        1 => B,
        2 => C,
        3 => D,
        4 => E,
        5 => F,
        6 => G,
        /// Unclassified or special use airspace.
        8 => Sua,
    }
}

codes! {
    /// The intended activity of an airspace as `ENR 5.5` defines it.
    ///
    /// The API schema also permits code 6 and does not describe it. No record
    /// in the datasets uses it, so it stays `Unknown`.
    pub enum Activity {
        /// No specific activity. This is the default for airspaces that
        /// `ENR 5.5` does not define.
        0 => None,
        1 => Parachuting,
        2 => Aerobatics,
        3 => AeroclubAndAerialWorkArea,
        /// Ultralight machine activity.
        4 => Ulm,
        5 => HangGlidingAndParagliding,
    }
}
