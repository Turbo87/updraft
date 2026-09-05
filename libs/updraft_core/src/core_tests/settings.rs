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
use crate::{
    ArrivalReserve, Ballast, Bugs, GlidePerformance, MacCready, SetArrivalReserve, SetBallast,
    SetBugs, SetMacCready,
};
use claims::assert_some_eq;

#[test]
fn maccready_is_published_but_not_persisted_and_resets_on_restart() {
    let mut core = Core::new(SettingsSnapshot::default());
    let initial = Topic::GlidePerformance(GlidePerformance::default());
    assert_some_eq!(core.topics().last(), &initial);
    let mac_cready = claims::assert_ok!(MacCready::try_from(1.5));
    let performance = GlidePerformance {
        mac_cready,
        ..GlidePerformance::default()
    };
    let topic = Topic::GlidePerformance(performance);
    let effects = core.apply(SetMacCready { mac_cready }, at(0)).effects;
    assert_eq!(effects, vec![Effect::emit(topic.clone())]);
    assert_some_eq!(core.topics().last(), &topic);
    let repeated = core.apply(SetMacCready { mac_cready }, at(1));
    assert_eq!(repeated.effects, vec![]);

    let bugs = claims::assert_ok!(Bugs::try_from(10.0));
    let performance = GlidePerformance {
        mac_cready,
        bugs,
        ..GlidePerformance::default()
    };
    let topic = Topic::GlidePerformance(performance);
    let effects = core.apply(SetBugs { bugs }, at(1)).effects;
    assert_eq!(effects, vec![Effect::emit(topic)]);
    let repeated = core.apply(SetBugs { bugs }, at(1));
    assert_eq!(repeated.effects, vec![]);

    let ballast = claims::assert_ok!(Ballast::try_from(100.5));
    let performance = GlidePerformance {
        ballast,
        ..performance
    };
    let topic = Topic::GlidePerformance(performance);
    let effects = core.apply(SetBallast { ballast }, at(1)).effects;
    assert_eq!(effects, vec![Effect::emit(topic)]);
    assert_eq!(core.apply(SetBallast { ballast }, at(1)).effects, vec![]);

    let effects = core.apply(SetLocale::new(Locale::De), at(2)).effects;
    let Effect::PersistSettings(snapshot) = &effects[1] else {
        panic!("settings must be saved")
    };
    insta::assert_json_snapshot!(snapshot);
    let restarted = Core::new(snapshot.clone());
    assert_some_eq!(restarted.topics().last(), &initial);
}

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
    assert_eq!(topics.len(), 7);
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
            Topic::Waypoints(crate::WaypointStatus::default()),
            Topic::Traffic(TrafficUpdate::Snapshot(Vec::new())),
            Topic::GlidePerformance(GlidePerformance::default()),
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
        ..Settings::default()
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
            Topic::Waypoints(crate::WaypointStatus::default()),
            Topic::Traffic(TrafficUpdate::Snapshot(Vec::new())),
            Topic::GlidePerformance(GlidePerformance::default()),
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

#[test]
fn setting_polar_updates_settings_and_requests_persistence() {
    let mut core = Core::new(SettingsSnapshot::default());
    let polar = claims::assert_ok!(crate::PolarId::try_from("LS 8-18".to_owned()));
    let settings = Settings {
        polar,
        ..Settings::default()
    };
    let snapshot = SettingsSnapshot {
        settings,
        external_devices: Vec::new(),
    };

    assert_eq!(
        core.apply(SetPolar { polar }, at(0)).effects,
        vec![
            Effect::emit(Topic::Settings(settings)),
            Effect::persist_settings(snapshot),
        ]
    );
    assert_eq!(core.apply(SetPolar { polar }, at(1)).effects, vec![]);
}

#[test]
fn arrival_reserve_publishes_and_persists_only_changes() {
    let mut core = Core::new(SettingsSnapshot::default());
    let reserve = claims::assert_ok!(ArrivalReserve::try_from(304.8));
    let settings = Settings {
        arrival_reserve: reserve,
        ..Settings::default()
    };
    let snapshot = SettingsSnapshot {
        settings,
        ..SettingsSnapshot::default()
    };
    let effects = core.apply(SetArrivalReserve { reserve }, at(0)).effects;
    assert_eq!(
        effects,
        vec![
            Effect::emit(settings.as_topic()),
            Effect::persist_settings(snapshot),
        ]
    );
    let repeated = core.apply(SetArrivalReserve { reserve }, at(1));
    assert_eq!(repeated.effects, vec![]);
}

#[test]
fn loaded_and_changed_polars_drive_netto() {
    use claims::{assert_ok, assert_some, assert_some_eq};
    let polar = assert_ok!(crate::PolarId::try_from("LS 8-18".to_owned()));
    let snapshot = SettingsSnapshot {
        settings: Settings {
            polar,
            ..Settings::default()
        },
        ..SettingsSnapshot::default()
    };
    let mut loaded = Core::new(snapshot);
    let mut changed = Core::new(SettingsSnapshot::default());
    let input = b"$LXWP0,Y,108,1000,1,1,1,1,1,1,239,174,10\r\n$PGRMZ,1000,m,2\r\n";
    for core in [&mut loaded, &mut changed] {
        let connection = ConnectionSpec::tcp("127.0.0.1", 4353);
        let add_device = AddExternalDevice::new(connection);
        let device = core.apply(add_device, at(0)).response;
        core.apply(Bytes::new(device, input), at(0));
        core.apply(Bytes::new(device, input), at(1_000));
    }
    let expected = assert_some!(loaded.instruments().derived);
    let before = assert_some!(changed.instruments().derived);
    assert_ne!(assert_some!(before.netto), assert_some!(expected.netto));
    assert_ne!(
        assert_some!(before.relative_vario),
        assert_some!(expected.relative_vario),
    );

    let effects = changed.apply(SetPolar { polar }, at(1_000)).effects;
    assert_some_eq!(changed.instruments().derived, expected);
    let instruments = Effect::emit(changed.instruments().as_topic());
    assert_some_eq!(effects.last(), &instruments);

    let bugs = assert_ok!(Bugs::try_from(10.0));
    let clean = changed.instruments();
    let effects = changed.apply(SetBugs { bugs }, at(1_000)).effects;
    assert_ne!(changed.instruments(), clean);
    let instruments = Effect::emit(changed.instruments().as_topic());
    assert_some_eq!(effects.last(), &instruments);

    let polar = crate::PolarId::default();
    let ballast = assert_ok!(Ballast::try_from(100.5));
    let dry = changed.instruments();
    let effects = changed.apply(SetBallast { ballast }, at(1_000)).effects;
    assert_ne!(changed.instruments(), dry);
    let instruments = Effect::emit(changed.instruments().as_topic());
    assert_some_eq!(effects.last(), &instruments);
    changed.apply(SetPolar { polar }, at(1_000));
    loaded.apply(SetPolar { polar }, at(1_000));
    loaded.apply(SetBugs { bugs }, at(1_000));
    loaded.apply(SetBallast { ballast }, at(1_000));
    assert_eq!(changed.instruments(), loaded.instruments());
}
