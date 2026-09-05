use crate::{ArrivalReserve, MacCready, WaypointSnapshot, topic::Instruments};
use updraft_polar::GlidePolar;

/// Waypoints and glide inputs captured by one core query.
/// Later sensor, catalog, and settings changes do not alter this snapshot.
#[derive(Clone, Debug)]
pub struct GlideSnapshot {
    pub waypoints: WaypointSnapshot,
    pub instruments: Instruments,
    pub polar: GlidePolar,
    pub mac_cready: MacCready,
    pub arrival_reserve: ArrivalReserve,
}
