use super::super::*;
use super::support::at;
use crate::AirspaceStatus;
use crate::connection::ConnectionSpec;
use crate::external_device::ExternalDeviceConfig;
use crate::settings::{
    AltitudeUnit, DistanceUnit, Locale, SettingsSnapshot, SpeedUnit, UnitSettings,
    VerticalSpeedUnit,
};
use crate::topic::Instruments;

#[test]
fn topics_include_settings_and_external_devices() {
    let settings = Settings {
        locale: Some(Locale::De),
        ..Settings::default()
    };
    let tcp = ConnectionSpec::tcp("127.0.0.1", 4353);
    let bluetooth = ConnectionSpec::bluetooth_spp("00:11:22:33:44:55");
    let core = Core::new(SettingsSnapshot {
        settings,
        external_devices: vec![
            ExternalDeviceConfig {
                enabled: true,
                spec: tcp.clone(),
            },
            ExternalDeviceConfig {
                enabled: false,
                spec: bluetooth.clone(),
            },
        ],
    });

    let topics = core.topics();
    assert_eq!(topics.len(), 5);
    assert_eq!(topics[0], Topic::Instruments(Instruments::default()));
    assert_eq!(topics[1], Topic::Settings(settings));
    let Topic::ExternalDevices(devices) = &topics[2] else {
        panic!("the configured external devices topic should be published");
    };
    assert_eq!(devices.len(), 2);
    assert_eq!(
        (devices[0].config.enabled, &devices[0].config.spec),
        (true, &tcp)
    );
    assert_eq!(
        (devices[1].config.enabled, &devices[1].config.spec),
        (false, &bluetooth),
    );
    assert_ne!(devices[0].device_id, devices[1].device_id);
}

#[test]
fn setting_locale_updates_the_topic_and_requests_persistence() {
    let mut core = Core::new(SettingsSnapshot::default());
    let settings = Settings {
        locale: Some(Locale::De),
        ..Settings::default()
    };
    let snapshot = SettingsSnapshot {
        settings,
        external_devices: Vec::new(),
    };

    let input = SetLocale::new(Locale::De);
    assert_eq!(
        core.apply(input, at(0)).effects,
        vec![
            Effect::emit(Topic::Settings(settings)),
            Effect::persist_settings(snapshot),
        ]
    );
    assert_eq!(
        core.topics(),
        vec![
            Topic::Instruments(Instruments::default()),
            Topic::Settings(settings),
            Topic::ExternalDevices(Vec::new()),
            Topic::Airspace(AirspaceStatus::None),
            Topic::Traffic(TrafficUpdate::Snapshot(Vec::new())),
        ]
    );
}

#[test]
fn setting_the_active_explicit_locale_is_a_no_op() {
    let mut core = Core::new(SettingsSnapshot {
        settings: Settings {
            locale: Some(Locale::De),
            ..Settings::default()
        },
        external_devices: Vec::new(),
    });

    let input = SetLocale::new(Locale::De);
    assert_eq!(core.apply(input, at(0)).effects, vec![]);
}

#[test]
fn setting_units_updates_the_topic_and_requests_persistence() {
    let mut core = Core::new(SettingsSnapshot {
        settings: Settings {
            locale: Some(Locale::De),
            ..Settings::default()
        },
        external_devices: Vec::new(),
    });
    let units = UnitSettings {
        altitude: AltitudeUnit::Feet,
        distance: DistanceUnit::NauticalMiles,
        speed: SpeedUnit::Knots,
        vertical_speed: VerticalSpeedUnit::FeetPerMinute,
    };
    let settings = Settings {
        locale: Some(Locale::De),
        units,
    };
    let snapshot = SettingsSnapshot {
        settings,
        external_devices: Vec::new(),
    };

    let input = SetUnits::new(units);
    assert_eq!(
        core.apply(input, at(0)).effects,
        vec![
            Effect::emit(Topic::Settings(settings)),
            Effect::persist_settings(snapshot),
        ]
    );
    assert_eq!(
        core.topics(),
        vec![
            Topic::Instruments(Instruments::default()),
            Topic::Settings(settings),
            Topic::ExternalDevices(Vec::new()),
            Topic::Airspace(AirspaceStatus::None),
            Topic::Traffic(TrafficUpdate::Snapshot(Vec::new())),
        ]
    );
}

#[test]
fn setting_the_active_unit_selections_is_a_no_op() {
    let units = UnitSettings {
        altitude: AltitudeUnit::Feet,
        distance: DistanceUnit::NauticalMiles,
        speed: SpeedUnit::Knots,
        vertical_speed: VerticalSpeedUnit::FeetPerMinute,
    };
    let mut core = Core::new(SettingsSnapshot {
        settings: Settings {
            units,
            ..Settings::default()
        },
        external_devices: Vec::new(),
    });

    let input = SetUnits::new(units);
    assert_eq!(core.apply(input, at(0)).effects, vec![]);
}
