use crate::{TrafficTargetId, TrafficTargetIdType};
use serde::de::Error as _;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// United FlarmNet fields. Missing text is represented by an empty string.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(default, rename_all(serialize = "camelCase"))]
pub struct FlarmnetRecord {
    pub flarm_id: String,
    pub call_sign: String,
    pub registration: String,
    pub plane_type: String,
    pub pilot_name: String,
    pub airfield: String,
    pub frequency: String,
}

/// Records share one hexadecimal address namespace, as in United FlarmNet.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FlarmnetDatabase {
    records: BTreeMap<u32, FlarmnetRecord>,
}

impl FlarmnetDatabase {
    /// Rejects malformed JSON, empty databases, invalid IDs, and duplicate IDs.
    pub fn from_json(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        let records: Vec<FlarmnetRecord> = serde_json::from_slice(bytes)?;
        if records.is_empty() {
            return Err(serde_json::Error::custom("FlarmNet database is empty"));
        }
        let mut database = Self::default();
        for mut record in records {
            let id = &record.flarm_id;
            if id.len() != 6 || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                let message = "FlarmNet ID must contain six hexadecimal digits";
                return Err(serde_json::Error::custom(message));
            }
            let id = u32::from_str_radix(id, 16).expect("Six hexadecimal digits fit in u32");
            record.flarm_id.make_ascii_uppercase();
            for text in [
                &mut record.call_sign,
                &mut record.registration,
                &mut record.plane_type,
                &mut record.pilot_name,
                &mut record.airfield,
                &mut record.frequency,
            ] {
                *text = text.trim().to_owned();
            }
            if database.records.insert(id, record).is_some() {
                let message = "FlarmNet database contains a duplicate ID";
                return Err(serde_json::Error::custom(message));
            }
        }
        Ok(database)
    }

    /// Matches FLARM and ICAO addresses. Random and unknown ID types are excluded.
    pub fn lookup(&self, id: TrafficTargetId) -> Option<&FlarmnetRecord> {
        match id.id_type {
            TrafficTargetIdType::Flarm | TrafficTargetIdType::Icao => self.records.get(&id.value),
            TrafficTargetIdType::Random | TrafficTargetIdType::Other(_) => None,
        }
    }
}
