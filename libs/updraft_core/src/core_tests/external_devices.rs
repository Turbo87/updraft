use super::super::*;
use super::support::*;
use crate::connection::{ConnectionSpec, ConnectionState};
use crate::settings::SettingsSnapshot;
use crate::topic::Instruments;
use std::assert_matches;
use tracing_test::traced_test;

fn emitted_instruments(effects: &[Effect]) -> Vec<Instruments> {
    effects
        .iter()
        .filter_map(|effect| {
            let Effect::Emit(Topic::Instruments(instruments)) = effect else {
                return None;
            };
            Some(*instruments)
        })
        .collect()
}

#[test]
#[traced_test]
fn disabled_external_device_ignores_lifecycle_events() {
    let mut core = Core::new(SettingsSnapshot {
        settings: Settings::default(),
        external_devices: vec![device_config(false, ConnectionSpec::tcp("127.0.0.1", 4353))],
    });
    let device_id = device_id(&core, 0);

    for (millis, state) in [
        (0, ConnectionState::Connecting),
        (1, ConnectionState::Connected),
        (2, ConnectionState::Disconnected),
    ] {
        let input = ConnectionChanged::new(device_id, state);
        assert!(core.apply(input, at(millis)).effects.is_empty());
    }

    logs_assert(|lines| {
        if lines.is_empty() {
            Ok(())
        } else {
            Err(format!("disabled device produced diagnostics: {lines:?}"))
        }
    });
}

#[test]
fn disabled_external_device_ignores_bytes() {
    let mut core = Core::new(SettingsSnapshot {
        settings: Settings::default(),
        external_devices: vec![device_config(false, ConnectionSpec::tcp("127.0.0.1", 4353))],
    });
    let device_id = device_id(&core, 0);

    let input = Bytes::new(device_id, RMC);
    assert!(core.apply(input, at(0)).effects.is_empty());
    assert_eq!(core.topics()[0], Topic::Instruments(Instruments::default()));
}

#[test]
fn disabling_selected_device_makes_gps_unavailable() {
    let (mut core, device_id) = core_with_external_device();
    core.apply(Bytes::new(device_id, RMC), at(0));

    let update = core.apply(SetExternalDeviceEnabled::disabled(device_id), at(1));

    assert_matches!(core.gps, DomainState::Unavailable);
    assert_eq!(
        emitted_instruments(&update.effects),
        vec![Instruments::default()]
    );
}

#[test]
fn disabling_selected_device_selects_fresh_fallback() {
    let (mut core, first, second) = core_with_two_external_devices();
    core.apply(Bytes::new(first, RMC), at(0));
    core.apply(Bytes::new(second, RMC_SECOND_DEVICE), at(1));

    let update = core.apply(SetExternalDeviceEnabled::disabled(first), at(2));

    assert_eq!(gps_instruments(&core).position.latitude_degrees, 51.0);
    assert_eq!(
        emitted_instruments(&update.effects),
        vec![instruments(&core)]
    );
}

#[test]
fn enabling_device_does_not_restore_discarded_gps() {
    let (mut core, device_id) = core_with_external_device();
    core.apply(Bytes::new(device_id, RMC), at(0));
    core.apply(SetExternalDeviceEnabled::disabled(device_id), at(1));

    let update = core.apply(SetExternalDeviceEnabled::enabled(device_id), at(2));

    assert_matches!(core.gps, DomainState::Unavailable);
    assert_eq!(emitted_instruments(&update.effects), Vec::new());
}

#[test]
fn deleting_selected_device_makes_gps_unavailable() {
    let (mut core, device_id) = core_with_external_device();
    core.apply(Bytes::new(device_id, RMC), at(0));

    let update = core.apply(DeleteExternalDevice::new(device_id), at(1));

    assert_matches!(core.gps, DomainState::Unavailable);
    assert_eq!(
        emitted_instruments(&update.effects),
        vec![Instruments::default()]
    );
}

#[test]
fn reorder_external_devices_preserves_partial_decoder_state() {
    let mut core = Core::new(SettingsSnapshot {
        settings: Settings::default(),
        external_devices: vec![
            device_config(true, ConnectionSpec::tcp("127.0.0.1", 4353)),
            device_config(true, ConnectionSpec::bluetooth_spp("00:11:22:33:44:55")),
        ],
    });
    let first = device_id(&core, 0);
    let second = device_id(&core, 1);

    let input = Bytes::new(first, &RMC[..24]);
    assert!(core.apply(input, at(0)).effects.is_empty());

    let input = ReorderExternalDevices::new(vec![second, first]);
    core.apply(input, at(1));

    let input = Bytes::new(first, &RMC[24..]);
    let effects = core.apply(input, at(2)).effects;

    assert_matches!(effects.as_slice(), [Effect::Emit(Topic::Instruments(_))]);
}

#[test]
#[traced_test]
fn reorder_external_devices_preserves_diagnostics_state() {
    let mut core = Core::new(SettingsSnapshot {
        settings: Settings::default(),
        external_devices: vec![
            device_config(true, ConnectionSpec::tcp("127.0.0.1", 4353)),
            device_config(true, ConnectionSpec::bluetooth_spp("00:11:22:33:44:55")),
        ],
    });
    let first = device_id(&core, 0);
    let second = device_id(&core, 1);

    let input = ConnectionChanged::new(first, ConnectionState::Connected);
    core.apply(input, at(0));

    core.apply(Bytes::new(first, b"abc"), at(1));

    let input = ReorderExternalDevices::new(vec![second, first]);
    core.apply(input, at(2));

    let input = ConnectionChanged::new(first, ConnectionState::Disconnected);
    core.apply(input, at(3));

    logs_assert(|lines| {
        insta::with_settings!({ filters => vec![TRACE_TIMESTAMP_FILTER] }, {
            insta::assert_snapshot!(lines.join("\n"));
        });
        Ok(())
    });
}

#[test]
fn reorder_external_devices_reselects_fresh_gps() {
    let (mut core, first, second) = core_with_two_external_devices();
    core.apply(Bytes::new(first, RMC), at(0));
    core.apply(Bytes::new(second, RMC_SECOND_DEVICE), at(1));

    let update = core.apply(ReorderExternalDevices::new(vec![second, first]), at(2));

    assert_eq!(gps_instruments(&core).position.latitude_degrees, 51.0);
    assert_eq!(
        emitted_instruments(&update.effects),
        vec![instruments(&core)]
    );

    core.apply(ReorderExternalDevices::new(vec![first, second]), at(2));
    assert_eq!(gps_instruments(&core).position.latitude_degrees, 50.823);
}

#[test]
fn reorder_external_devices_keeps_selected_last_known_gps() {
    let (mut core, first, second) = core_with_two_external_devices();
    core.apply(Bytes::new(first, RMC), at(0));
    core.apply(Bytes::new(second, RMC_SECOND_DEVICE), at(1_000));
    core.apply(Tick, at(3_000));
    core.apply(Tick, at(4_000));

    let update = core.apply(ReorderExternalDevices::new(vec![second, first]), at(4_001));

    let DomainState::LastKnown(selected) = core.gps else {
        panic!("GPS should remain last known");
    };
    assert_eq!(selected.source, SourceId::External(second));
    assert_eq!(emitted_instruments(&update.effects), Vec::new());
}

#[test]
fn edit_external_device_resets_partial_decoder_state() {
    let (mut core, device_id) = core_with_external_device();

    let input = Bytes::new(device_id, &RMC[..24]);
    assert!(core.apply(input, at(0)).effects.is_empty());

    let input = EditExternalDevice::new(device_id, ConnectionSpec::tcp("192.0.2.1", 10110));
    core.apply(input, at(1));

    let input = Bytes::new(device_id, &RMC[24..]);
    assert!(core.apply(input, at(2)).effects.is_empty());
    assert_eq!(core.topics()[0], Topic::Instruments(Instruments::default()));
}

#[test]
fn edit_external_device_resets_gps_candidate() {
    let (mut core, device_id) = core_with_external_device();

    core.apply(Bytes::new(device_id, RMC), at(0));
    core.apply(Bytes::new(device_id, GGA), at(1));

    let input = EditExternalDevice::new(device_id, ConnectionSpec::tcp("192.0.2.1", 10110));
    core.apply(input, at(2));

    let device = core
        .external_devices
        .iter()
        .find(|device| device.device_id == device_id)
        .expect("the edited external device");
    assert_eq!(device.gps, GpsCandidate::default());
}

#[test]
fn editing_selected_device_makes_gps_unavailable() {
    let (mut core, device_id) = core_with_external_device();
    core.apply(Bytes::new(device_id, RMC), at(0));

    let spec = ConnectionSpec::tcp("192.0.2.1", 10110);
    let update = core.apply(EditExternalDevice::new(device_id, spec.clone()), at(1));

    assert_matches!(core.gps, DomainState::Unavailable);
    assert_eq!(
        emitted_instruments(&update.effects),
        vec![Instruments::default()]
    );
}

#[test]
#[traced_test]
fn edit_external_device_resets_diagnostics_state() {
    let (mut core, device_id) = core_with_external_device();

    let input = ConnectionChanged::new(device_id, ConnectionState::Connected);
    core.apply(input, at(0));

    core.apply(Bytes::new(device_id, b"abc"), at(1));

    let input = EditExternalDevice::new(device_id, ConnectionSpec::tcp("192.0.2.1", 10110));
    core.apply(input, at(2));

    let input = ConnectionChanged::new(device_id, ConnectionState::Disconnected);
    core.apply(input, at(3));

    logs_assert(|lines| {
        insta::with_settings!({ filters => vec![TRACE_TIMESTAMP_FILTER] }, {
            insta::assert_snapshot!(lines.join("\n"));
        });
        Ok(())
    });
}

#[test]
fn set_external_device_enabled_resets_partial_decoder_state() {
    let (mut core, device_id) = core_with_external_device();

    let input = Bytes::new(device_id, &RMC[..24]);
    assert!(core.apply(input, at(0)).effects.is_empty());

    core.apply(SetExternalDeviceEnabled::disabled(device_id), at(1));
    core.apply(SetExternalDeviceEnabled::enabled(device_id), at(2));

    let input = Bytes::new(device_id, &RMC[24..]);
    assert!(core.apply(input, at(3)).effects.is_empty());
    assert_eq!(core.topics()[0], Topic::Instruments(Instruments::default()));
}

#[test]
#[traced_test]
fn set_external_device_enabled_resets_diagnostics_state() {
    let (mut core, device_id) = core_with_external_device();

    let input = ConnectionChanged::new(device_id, ConnectionState::Connected);
    core.apply(input, at(0));

    core.apply(Bytes::new(device_id, b"abc"), at(1));
    core.apply(SetExternalDeviceEnabled::disabled(device_id), at(2));
    core.apply(SetExternalDeviceEnabled::enabled(device_id), at(3));

    let input = ConnectionChanged::new(device_id, ConnectionState::Disconnected);
    core.apply(input, at(4));

    logs_assert(|lines| {
        insta::with_settings!({ filters => vec![TRACE_TIMESTAMP_FILTER] }, {
            insta::assert_snapshot!(lines.join("\n"));
        });
        Ok(())
    });
}
