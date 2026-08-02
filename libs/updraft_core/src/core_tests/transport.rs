use super::super::*;
use super::support::*;
use crate::connection::{ConnectionSpec, ConnectionState};
use crate::external_device::ExternalDeviceConfig;
use crate::settings::{Locale, SettingsSnapshot};
use approx::assert_abs_diff_eq;
use tracing_test::traced_test;

#[test]
fn start_opens_only_enabled_external_devices() {
    let tcp = ConnectionSpec::tcp("127.0.0.1", 4353);
    let bluetooth = ConnectionSpec::bluetooth_spp("00:11:22:33:44:55");
    let mut core = Core::new(SettingsSnapshot {
        settings: Settings {
            locale: Some(Locale::De),
            ..Settings::default()
        },
        external_devices: vec![
            ExternalDeviceConfig {
                enabled: false,
                spec: tcp.clone(),
            },
            ExternalDeviceConfig {
                enabled: true,
                spec: bluetooth.clone(),
            },
        ],
    });

    let Some(Topic::ExternalDevices(devices)) = core
        .topics()
        .into_iter()
        .find(|topic| matches!(topic, Topic::ExternalDevices(_)))
    else {
        panic!("the configured external devices topic should be published");
    };
    assert_eq!(devices.len(), 2);
    assert_eq!(
        (devices[0].config.enabled, &devices[0].config.spec),
        (false, &tcp)
    );
    assert_eq!(
        (devices[1].config.enabled, &devices[1].config.spec),
        (true, &bluetooth),
    );
    assert_ne!(devices[0].device_id, devices[1].device_id);

    let enabled_device_id = devices[1].device_id;
    assert_eq!(
        core.apply(Start, at(0)).effects,
        vec![Effect::open(enabled_device_id, bluetooth)]
    );
}

#[test]
fn bytes_are_decoded_by_their_configured_device() {
    let mut core = Core::new(SettingsSnapshot {
        settings: Settings::default(),
        external_devices: vec![
            device_config(true, ConnectionSpec::tcp("127.0.0.1", 4353)),
            device_config(true, ConnectionSpec::bluetooth_spp("00:11:22:33:44:55")),
        ],
    });
    let tcp_device_id = device_id(&core, 0);
    let bluetooth_device_id = device_id(&core, 1);

    let input = Bytes::new(tcp_device_id, &RMC[..24]);
    assert!(core.apply(input, at(0)).effects.is_empty());

    let input = Bytes::new(bluetooth_device_id, &RMC[24..]);
    assert!(core.apply(input, at(1)).effects.is_empty());

    let input = Bytes::new(tcp_device_id, &RMC[24..]);
    let effects = core.apply(input, at(2)).effects;
    let [Effect::Emit(Topic::Instruments(instruments))] = effects.as_slice() else {
        panic!("the completed sentence should emit instruments");
    };
    let position = instruments.position.expect("RMC position");
    assert_abs_diff_eq!(position.latitude_degrees, 50.823, epsilon = 1e-3);
    assert_abs_diff_eq!(position.longitude_degrees, 6.186, epsilon = 1e-3);
}

#[test]
#[traced_test]
fn spp_lifecycle_reports_the_mac_address() {
    let mut core = Core::new(SettingsSnapshot {
        settings: Settings::default(),
        external_devices: vec![device_config(
            true,
            ConnectionSpec::bluetooth_spp("00:00:00:00:00:00"),
        )],
    });
    let device_id = device_id(&core, 0);

    let input = ConnectionChanged::new(device_id, ConnectionState::Connecting);
    core.apply(input, at(0));

    assert!(logs_contain(&format!("{device_id:?}")));
    assert!(logs_contain("00:00:00:00:00:00"));
}

#[test]
#[traced_test]
fn connection_lifecycle_reports_endpoint_and_delivered_bytes() {
    let (mut core, device_id) = core_with_external_device();

    let input = ConnectionChanged::new(device_id, ConnectionState::Connecting);
    core.apply(input, at(0));

    let input = ConnectionChanged::new(device_id, ConnectionState::Connected);
    core.apply(input, at(1));

    core.apply(Bytes::new(device_id, b"abc"), at(2));
    core.apply(Bytes::new(device_id, b"de"), at(3));

    let input = ConnectionChanged::new(device_id, ConnectionState::Disconnected);
    core.apply(input, at(4));

    logs_assert(|lines| {
        insta::with_settings!({ filters => vec![TRACE_TIMESTAMP_FILTER] }, {
            insta::assert_snapshot!(lines.join("\n"));
        });
        Ok(())
    });
}

#[test]
#[traced_test]
fn failed_attempt_is_debug_and_counters_reset_on_reconnect() {
    let (mut core, device_id) = core_with_external_device();

    let input = ConnectionChanged::new(device_id, ConnectionState::Disconnected);
    core.apply(input, at(0));

    for (millis, bytes) in [(1, b"abc".as_slice()), (4, b"de".as_slice())] {
        let input = ConnectionChanged::new(device_id, ConnectionState::Connecting);
        core.apply(input, at(millis));

        let input = ConnectionChanged::new(device_id, ConnectionState::Connected);
        core.apply(input, at(millis + 1));

        core.apply(Bytes::new(device_id, bytes), at(millis + 2));

        let input = ConnectionChanged::new(device_id, ConnectionState::Disconnected);
        core.apply(input, at(millis + 3));
    }

    logs_assert(|lines| {
        insta::with_settings!({ filters => vec![TRACE_TIMESTAMP_FILTER] }, {
            insta::assert_snapshot!(lines.join("\n"));
        });
        Ok(())
    });
}

#[test]
#[traced_test]
fn unknown_and_empty_bytes_produce_no_delivery_log() {
    let (mut core, device_id) = core_with_external_device();

    core.apply(Bytes::new(ExternalDeviceId(99), b"abc"), at(0));
    core.apply(Bytes::new(device_id, b""), at(1));

    assert!(!logs_contain("First bytes"));
}

#[test]
#[traced_test]
fn removed_connection_produces_no_further_diagnostics() {
    let (mut core, device_id) = core_with_external_device();

    let input = ConnectionChanged::new(device_id, ConnectionState::Connected);
    core.apply(input, at(0));

    core.apply(Bytes::new(device_id, b"abc"), at(1));
    assert!(core.external_devices.remove(device_id).is_some());

    let input = ConnectionChanged::new(device_id, ConnectionState::Connecting);
    core.apply(input, at(2));

    core.apply(Bytes::new(device_id, b"de"), at(3));

    logs_assert(|lines| {
        insta::with_settings!({ filters => vec![TRACE_TIMESTAMP_FILTER] }, {
            insta::assert_snapshot!(lines.join("\n"));
        });
        Ok(())
    });
}
