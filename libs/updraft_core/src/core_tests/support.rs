use super::super::*;
use crate::connection::ConnectionSpec;
use crate::external_device::ExternalDeviceConfig;
use crate::settings::SettingsSnapshot;
use crate::topic::Instruments;
use crate::traffic::{PublishedTrafficTarget, TrafficDelta, TrafficUpdate};
use updraft_geo::LatLon;
use updraft_units::{Angle, EllipsoidAltitude, Length, Speed};

pub const RMC: &[u8] = b"$GPRMC,120000.00,A,5049.38,N,00611.16,E,45.0,270.0,010126,,,A\r\n";
pub const RMC_SECOND_DEVICE: &[u8] =
    b"$GPRMC,120000.00,A,5100.00,N,00700.00,E,40.0,180.0,010126,,,A\r\n";
pub const INVALID_MODE_RMC: &[u8] =
    b"$GPRMC,120000.00,A,5100.00,N,00700.00,E,40.0,180.0,010126,,,N\r\n";
pub const POSITION_ONLY_RMC: &[u8] = b"$GPRMC,120000.00,A,5049.38,N,00611.16,E,,,010126,,,A\r\n";
pub const OPTIONAL_ONLY_RMC: &[u8] = b"$GPRMC,120000.00,A,,,,,50.0,90.0,010126,,,A\r\n";
pub const GGA: &[u8] = b"$GPGGA,120000.00,5049.38,N,00611.16,E,1,08,0.9,200.0,M,0.0,M,,\r\n";
pub const GGA_SECOND_DEVICE: &[u8] =
    b"$GPGGA,120000.00,5100.00,N,00700.00,E,1,08,0.9,300.0,M,0.0,M,,\r\n";
pub const INVALID_GGA: &[u8] =
    b"$GPGGA,120000.00,5100.00,N,00700.00,E,0,08,0.9,300.0,M,0.0,M,,\r\n";
pub const ALTITUDE_ONLY_GGA: &[u8] = b"$GPGGA,120000.00,,,,,1,08,0.9,250.0,M,0.0,M,,\r\n";
pub const PFLAA_A: &[u8] = b"$PFLAA,0,1000,200,50,1,ABC123,90,0,25,0,1,0\r\n";
pub const PFLAA_B: &[u8] = b"$PFLAA,1,-500,300,-20,2,DEF456,225,0,30,0,6,0\r\n";
pub const PFLAA_A_REPLACEMENT: &[u8] = b"$PFLAA,2,2000,400,100,1,ABC123,180,0,30,0,1,0\r\n";
pub const PFLAA_A_MISSING_EAST: &[u8] = b"$PFLAA,3,2000,,100,1,ABC123,180,0,30,0,1,0\r\n";
pub const TRACE_TIMESTAMP_FILTER: (&str, &str) =
    (r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z", "[TIME]");

pub fn device_config(enabled: bool, spec: ConnectionSpec) -> ExternalDeviceConfig {
    ExternalDeviceConfig { enabled, spec }
}

pub fn config() -> SettingsSnapshot {
    SettingsSnapshot {
        settings: Settings::default(),
        external_devices: vec![device_config(true, ConnectionSpec::tcp("127.0.0.1", 4353))],
    }
}

pub fn device_id(core: &Core, index: usize) -> ExternalDeviceId {
    let Some(Topic::ExternalDevices(devices)) = core
        .topics()
        .into_iter()
        .find(|topic| matches!(topic, Topic::ExternalDevices(_)))
    else {
        panic!("the configured external devices topic should be published");
    };
    devices[index].device_id
}

pub fn core_with_external_device() -> (Core, ExternalDeviceId) {
    let core = Core::new(config());
    let device_id = device_id(&core, 0);
    (core, device_id)
}

pub fn core_with_two_external_devices() -> (Core, ExternalDeviceId, ExternalDeviceId) {
    let core = Core::new(SettingsSnapshot {
        settings: Settings::default(),
        external_devices: vec![
            device_config(true, ConnectionSpec::tcp("127.0.0.1", 4353)),
            device_config(true, ConnectionSpec::tcp("127.0.0.1", 4354)),
        ],
    });
    let first = device_id(&core, 0);
    let second = device_id(&core, 1);
    (core, first, second)
}

pub fn instruments(core: &Core) -> Instruments {
    let Topic::Instruments(instruments) = &core.topics()[0] else {
        panic!("the first topic should contain instruments");
    };
    *instruments
}

pub fn traffic_delta(effects: &[Effect]) -> &TrafficDelta {
    effects
        .iter()
        .find_map(|effect| {
            let Effect::Emit(Topic::Traffic(TrafficUpdate::Delta(delta))) = effect else {
                return None;
            };
            Some(delta)
        })
        .expect("a traffic delta")
}

pub fn traffic_snapshot(core: &Core) -> Vec<PublishedTrafficTarget> {
    core.topics()
        .into_iter()
        .find_map(|topic| {
            let Topic::Traffic(TrafficUpdate::Snapshot(targets)) = topic else {
                return None;
            };
            Some(targets)
        })
        .expect("a traffic snapshot")
}

pub fn at(millis: u64) -> Timestamp {
    Timestamp::from_millis(millis)
}

pub fn fix(latitude_degrees: f64, longitude_degrees: f64) -> Fix {
    Fix {
        position: LatLon::from_degrees(latitude_degrees, longitude_degrees),
        altitude_ellipsoid: Some(EllipsoidAltitude::new(Length::from_meters(247.0))),
        track: Some(Angle::from_degrees(90.0)),
        ground_speed: Some(Speed::from_meters_per_second(30.0)),
    }
}
