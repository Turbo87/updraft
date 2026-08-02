use updraft_core::{
    ConnectionSpec, Core, Effect, ExternalDeviceConfig, ExternalDeviceId, SettingsSnapshot, Topic,
};

pub const EXTERNAL_DEVICE_ID_FILTER: (&str, &str) =
    (r"ExternalDeviceId\(\d+\)", "ExternalDeviceId([ID])");

/// Rounds to a quantity-appropriate precision so snapshots record real
/// behaviour changes and not last-bit float differences.
pub fn describe(effect: &Effect) -> String {
    fn number(value: Option<f64>, decimals: usize) -> String {
        value.map_or_else(|| "none".to_owned(), |v| format!("{v:.decimals$}"))
    }

    match effect {
        Effect::Emit(Topic::Instruments(instruments)) => {
            let position = instruments.position.map_or_else(
                || "none".to_owned(),
                |p| format!("{:.5},{:.5}", p.latitude_degrees, p.longitude_degrees),
            );

            format!(
                "instruments pos={position} track={} gs={} alt={}",
                number(instruments.track_degrees, 2),
                number(instruments.ground_speed_meters_per_second, 2),
                number(instruments.altitude_msl_meters, 1),
            )
        }
        Effect::Emit(Topic::Settings(settings)) => format!("settings {settings:?}"),
        Effect::Emit(Topic::ExternalDevices(devices)) => {
            format!("external devices {devices:?}")
        }
        Effect::Emit(Topic::Traffic(update)) => format!("traffic {update:?}"),
        Effect::OpenConnection { device_id, spec } => format!("open {device_id:?} {spec:?}"),
        Effect::CloseConnection { device_id } => format!("close {device_id:?}"),
        Effect::PersistSettings(settings) => format!("persist settings {settings:?}"),
    }
}

pub fn core_with_external_device() -> (Core, ExternalDeviceId) {
    let core = Core::new(SettingsSnapshot {
        settings: Default::default(),
        external_devices: vec![ExternalDeviceConfig {
            enabled: true,
            spec: ConnectionSpec::tcp("127.0.0.1", 4353),
        }],
    });
    let Some(Topic::ExternalDevices(devices)) = core
        .topics()
        .into_iter()
        .find(|topic| matches!(topic, Topic::ExternalDevices(_)))
    else {
        panic!("the configured external devices topic should be published");
    };
    (core, devices[0].device_id)
}

pub fn core_with_two_external_devices() -> Core {
    Core::new(SettingsSnapshot {
        settings: Default::default(),
        external_devices: vec![
            ExternalDeviceConfig {
                enabled: true,
                spec: ConnectionSpec::tcp("127.0.0.1", 4353),
            },
            ExternalDeviceConfig {
                enabled: false,
                spec: ConnectionSpec::bluetooth_spp("00:11:22:33:44:55"),
            },
        ],
    })
}

pub fn external_device_ids(core: &Core) -> Vec<ExternalDeviceId> {
    let Some(Topic::ExternalDevices(devices)) = core
        .topics()
        .into_iter()
        .find(|topic| matches!(topic, Topic::ExternalDevices(_)))
    else {
        panic!("the configured external devices topic should be published");
    };
    devices.iter().map(|device| device.device_id).collect()
}

pub fn mutation_effects(effects: &[Effect]) -> String {
    effects.iter().map(describe).collect::<Vec<_>>().join("\n")
}
