use super::support::{
    EXTERNAL_DEVICE_ID_FILTER, core_with_external_device, core_with_two_external_devices,
    external_device_ids, mutation_effects,
};
use updraft_core::{
    AddExternalDevice, ConnectionSpec, Core, DeleteExternalDevice, EditExternalDevice, Effect,
    ExternalDeviceConfig, ExternalDeviceId, InvalidExternalDeviceOrder, ReorderExternalDevices,
    SetExternalDeviceEnabled, SettingsSnapshot, Timestamp, Topic, UnknownExternalDevice, Update,
};

#[test]
fn add_external_device_to_an_empty_list() {
    let mut core = Core::new(SettingsSnapshot::default());
    let spec = ConnectionSpec::tcp("127.0.0.1", 4353);

    let input = AddExternalDevice::new(spec.clone());
    let Update { effects, response } = core.apply(input, Timestamp::from_millis(0));
    assert_eq!(response, ExternalDeviceId(1));

    let [
        Effect::OpenConnection { device_id, .. },
        Effect::Emit(Topic::ExternalDevices(devices)),
        Effect::PersistSettings(_),
    ] = effects.as_slice()
    else {
        panic!("addition should open, publish, and persist");
    };
    assert_eq!(*device_id, response);
    assert_eq!(devices[0].device_id, response);

    insta::with_settings!({ filters => vec![EXTERNAL_DEVICE_ID_FILTER] }, {
        insta::assert_snapshot!(mutation_effects(&effects));
    });
}

#[test]
fn add_external_device_after_loaded_devices_uses_a_fresh_id() {
    let mut core = core_with_two_external_devices();
    let existing = external_device_ids(&core);

    let input = AddExternalDevice::new(ConnectionSpec::tcp("192.0.2.1", 10110));
    let Update { effects, response } = core.apply(input, Timestamp::from_millis(0));
    assert_eq!(response, ExternalDeviceId(3));

    let [
        Effect::OpenConnection { device_id, .. },
        Effect::Emit(Topic::ExternalDevices(devices)),
        Effect::PersistSettings(_),
    ] = effects.as_slice()
    else {
        panic!("addition should open, publish, and persist");
    };
    assert_ne!(existing[0], existing[1]);
    assert_eq!(devices[0].device_id, existing[0]);
    assert_eq!(devices[1].device_id, existing[1]);
    assert_eq!(devices[2].device_id, response);
    assert_eq!(*device_id, response);
    assert!(existing.iter().all(|existing| existing != &response));

    insta::with_settings!({ filters => vec![EXTERNAL_DEVICE_ID_FILTER] }, {
        insta::assert_snapshot!(mutation_effects(&effects));
    });
}

#[test]
fn delete_external_device_closes_an_enabled_device() {
    let (mut core, device_id) = core_with_external_device();

    let input = DeleteExternalDevice::new(device_id);
    let Update { effects, response } = core.apply(input, Timestamp::from_millis(0));
    assert_eq!(response, Ok(()));

    let Effect::CloseConnection {
        device_id: closed_id,
    } = &effects[0]
    else {
        panic!("deleting an enabled device should close it first");
    };
    assert_eq!(*closed_id, device_id);

    insta::with_settings!({ filters => vec![EXTERNAL_DEVICE_ID_FILTER] }, {
        insta::assert_snapshot!(mutation_effects(&effects));
    });
}

#[test]
fn delete_external_device_does_not_close_a_disabled_device() {
    let mut core = Core::new(SettingsSnapshot {
        settings: Default::default(),
        external_devices: vec![ExternalDeviceConfig {
            enabled: false,
            spec: ConnectionSpec::tcp("127.0.0.1", 4353),
        }],
    });
    let device_id = external_device_ids(&core)[0];

    let input = DeleteExternalDevice::new(device_id);
    let Update { effects, response } = core.apply(input, Timestamp::from_millis(0));
    assert_eq!(response, Ok(()));

    insta::with_settings!({ filters => vec![EXTERNAL_DEVICE_ID_FILTER] }, {
        insta::assert_snapshot!(mutation_effects(&effects));
    });
}

#[test]
fn delete_external_device_with_an_unknown_id_is_a_no_op() {
    let mut core = Core::new(SettingsSnapshot::default());
    let unknown = ExternalDeviceId(u32::MAX);

    let input = DeleteExternalDevice::new(unknown);
    let Update { effects, response } = core.apply(input, Timestamp::from_millis(0));
    assert_eq!(response, Err(UnknownExternalDevice { device_id: unknown }));
    assert!(effects.is_empty());
}

#[test]
fn reorder_external_devices_publishes_and_persists_the_new_order() {
    let mut core = Core::new(SettingsSnapshot {
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
    });
    let original = external_device_ids(&core);
    let order = vec![original[1], original[0]];

    let input = ReorderExternalDevices::new(order.clone());
    let Update { effects, response } = core.apply(input, Timestamp::from_millis(0));
    assert_eq!(response, Ok(()));

    let [
        Effect::Emit(Topic::ExternalDevices(devices)),
        Effect::PersistSettings(_),
    ] = effects.as_slice()
    else {
        panic!("reordering should publish and persist");
    };
    assert_eq!(
        devices
            .iter()
            .map(|device| device.device_id)
            .collect::<Vec<_>>(),
        order
    );

    insta::with_settings!({ filters => vec![EXTERNAL_DEVICE_ID_FILTER] }, {
        insta::assert_snapshot!(mutation_effects(&effects));
    });
}

#[test]
fn reorder_external_devices_with_the_current_order_is_a_no_op() {
    let mut core = Core::new(SettingsSnapshot {
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
    });
    let order = external_device_ids(&core);

    let input = ReorderExternalDevices::new(order);
    let Update { effects, response } = core.apply(input, Timestamp::from_millis(0));
    assert_eq!(response, Ok(()));
    assert!(effects.is_empty());
}

#[test]
fn reorder_external_devices_rejects_unknown_missing_and_duplicate_ids() {
    let mut unknown = core_with_two_external_devices();
    let unknown_ids = external_device_ids(&unknown);
    let input = ReorderExternalDevices::new(vec![unknown_ids[0], ExternalDeviceId(u32::MAX)]);
    let Update { effects, response } = unknown.apply(input, Timestamp::from_millis(0));
    assert_eq!(response, Err(InvalidExternalDeviceOrder));
    assert!(effects.is_empty());

    let mut missing = core_with_two_external_devices();
    let missing_ids = external_device_ids(&missing);
    let input = ReorderExternalDevices::new(vec![missing_ids[0]]);
    let Update { effects, response } = missing.apply(input, Timestamp::from_millis(0));
    assert_eq!(response, Err(InvalidExternalDeviceOrder));
    assert!(effects.is_empty());

    let mut duplicate = core_with_two_external_devices();
    let duplicate_ids = external_device_ids(&duplicate);
    let input = ReorderExternalDevices::new(vec![duplicate_ids[0], duplicate_ids[0]]);
    let Update { effects, response } = duplicate.apply(input, Timestamp::from_millis(0));
    assert_eq!(response, Err(InvalidExternalDeviceOrder));
    assert!(effects.is_empty());
}

#[test]
fn edit_external_device_with_an_identical_spec_is_a_no_op() {
    let (mut core, device_id) = core_with_external_device();

    let input = EditExternalDevice::new(device_id, ConnectionSpec::tcp("127.0.0.1", 4353));
    let Update { effects, response } = core.apply(input, Timestamp::from_millis(0));
    assert_eq!(response, Ok(()));
    assert!(effects.is_empty());
}

#[test]
fn edit_external_device_restarts_an_enabled_device_with_the_same_id() {
    let (mut core, device_id) = core_with_external_device();

    let input = EditExternalDevice::new(device_id, ConnectionSpec::tcp("192.0.2.1", 10110));
    let Update { effects, response } = core.apply(input, Timestamp::from_millis(0));
    assert_eq!(response, Ok(()));

    let [
        Effect::CloseConnection {
            device_id: closed_id,
        },
        Effect::OpenConnection {
            device_id: opened_id,
            ..
        },
        Effect::Emit(Topic::ExternalDevices(devices)),
        Effect::PersistSettings(_),
    ] = effects.as_slice()
    else {
        panic!("editing an enabled device should close, open, publish, and persist");
    };
    assert_eq!(*closed_id, device_id);
    assert_eq!(*opened_id, device_id);
    assert_eq!(devices[0].device_id, device_id);

    insta::with_settings!({ filters => vec![EXTERNAL_DEVICE_ID_FILTER] }, {
        insta::assert_snapshot!(mutation_effects(&effects));
    });
}

#[test]
fn edit_external_device_switches_between_transport_types() {
    let (mut tcp_core, tcp_device_id) = core_with_external_device();
    let input = EditExternalDevice::new(
        tcp_device_id,
        ConnectionSpec::bluetooth_spp("00:11:22:33:44:55"),
    );
    let Update {
        effects: tcp_to_bluetooth,
        response,
    } = tcp_core.apply(input, Timestamp::from_millis(0));
    assert_eq!(response, Ok(()));
    let [
        Effect::CloseConnection {
            device_id: tcp_closed_id,
        },
        Effect::OpenConnection {
            device_id: bluetooth_opened_id,
            ..
        },
        Effect::Emit(Topic::ExternalDevices(bluetooth_devices)),
        Effect::PersistSettings(_),
    ] = tcp_to_bluetooth.as_slice()
    else {
        panic!("TCP-to-Bluetooth edit should close, open, publish, and persist");
    };
    assert_eq!(*tcp_closed_id, tcp_device_id);
    assert_eq!(*bluetooth_opened_id, tcp_device_id);
    assert_eq!(bluetooth_devices[0].device_id, tcp_device_id);

    let mut bluetooth_core = Core::new(SettingsSnapshot {
        settings: Default::default(),
        external_devices: vec![ExternalDeviceConfig {
            enabled: true,
            spec: ConnectionSpec::bluetooth_spp("00:11:22:33:44:55"),
        }],
    });
    let bluetooth_device_id = external_device_ids(&bluetooth_core)[0];
    let input =
        EditExternalDevice::new(bluetooth_device_id, ConnectionSpec::tcp("127.0.0.1", 4353));
    let Update {
        effects: bluetooth_to_tcp,
        response,
    } = bluetooth_core.apply(input, Timestamp::from_millis(0));
    assert_eq!(response, Ok(()));
    let [
        Effect::CloseConnection {
            device_id: bluetooth_closed_id,
        },
        Effect::OpenConnection {
            device_id: tcp_opened_id,
            ..
        },
        Effect::Emit(Topic::ExternalDevices(tcp_devices)),
        Effect::PersistSettings(_),
    ] = bluetooth_to_tcp.as_slice()
    else {
        panic!("Bluetooth-to-TCP edit should close, open, publish, and persist");
    };
    assert_eq!(*bluetooth_closed_id, bluetooth_device_id);
    assert_eq!(*tcp_opened_id, bluetooth_device_id);
    assert_eq!(tcp_devices[0].device_id, bluetooth_device_id);

    insta::with_settings!({ filters => vec![EXTERNAL_DEVICE_ID_FILTER] }, {
        insta::assert_snapshot!(
            "edit_external_device_tcp_to_bluetooth",
            mutation_effects(&tcp_to_bluetooth)
        );
        insta::assert_snapshot!(
            "edit_external_device_bluetooth_to_tcp",
            mutation_effects(&bluetooth_to_tcp)
        );
    });
}

#[test]
fn edit_external_device_updates_a_disabled_device_without_transport_effects() {
    let mut core = Core::new(SettingsSnapshot {
        settings: Default::default(),
        external_devices: vec![ExternalDeviceConfig {
            enabled: false,
            spec: ConnectionSpec::tcp("127.0.0.1", 4353),
        }],
    });
    let device_id = external_device_ids(&core)[0];

    let input = EditExternalDevice::new(device_id, ConnectionSpec::tcp("192.0.2.1", 10110));
    let Update { effects, response } = core.apply(input, Timestamp::from_millis(0));
    assert_eq!(response, Ok(()));

    let [
        Effect::Emit(Topic::ExternalDevices(devices)),
        Effect::PersistSettings(_),
    ] = effects.as_slice()
    else {
        panic!("editing a disabled device should publish and persist");
    };
    assert_eq!(devices[0].device_id, device_id);

    insta::with_settings!({ filters => vec![EXTERNAL_DEVICE_ID_FILTER] }, {
        insta::assert_snapshot!(mutation_effects(&effects));
    });
}

#[test]
fn edit_external_device_with_an_unknown_id_is_a_no_op() {
    let mut core = Core::new(SettingsSnapshot::default());
    let unknown = ExternalDeviceId(u32::MAX);

    let input = EditExternalDevice::new(unknown, ConnectionSpec::tcp("127.0.0.1", 4353));
    let Update { effects, response } = core.apply(input, Timestamp::from_millis(0));
    assert_eq!(response, Err(UnknownExternalDevice { device_id: unknown }));
    assert!(effects.is_empty());
}

#[test]
fn set_external_device_enabled_disables_an_enabled_device() {
    let (mut core, device_id) = core_with_external_device();

    let input = SetExternalDeviceEnabled::disabled(device_id);
    let Update { effects, response } = core.apply(input, Timestamp::from_millis(0));
    assert_eq!(response, Ok(()));

    let [
        Effect::CloseConnection {
            device_id: closed_id,
        },
        Effect::Emit(Topic::ExternalDevices(devices)),
        Effect::PersistSettings(_),
    ] = effects.as_slice()
    else {
        panic!("disabling should close, publish, and persist");
    };
    assert_eq!(*closed_id, device_id);
    assert_eq!(devices[0].device_id, device_id);

    insta::with_settings!({ filters => vec![EXTERNAL_DEVICE_ID_FILTER] }, {
        insta::assert_snapshot!(mutation_effects(&effects));
    });
}

#[test]
fn set_external_device_enabled_enables_a_disabled_device() {
    let mut core = Core::new(SettingsSnapshot {
        settings: Default::default(),
        external_devices: vec![ExternalDeviceConfig {
            enabled: false,
            spec: ConnectionSpec::tcp("127.0.0.1", 4353),
        }],
    });
    let device_id = external_device_ids(&core)[0];

    let input = SetExternalDeviceEnabled::enabled(device_id);
    let Update { effects, response } = core.apply(input, Timestamp::from_millis(0));
    assert_eq!(response, Ok(()));

    let [
        Effect::OpenConnection {
            device_id: opened_id,
            ..
        },
        Effect::Emit(Topic::ExternalDevices(devices)),
        Effect::PersistSettings(_),
    ] = effects.as_slice()
    else {
        panic!("enabling should open, publish, and persist");
    };
    assert_eq!(*opened_id, device_id);
    assert_eq!(devices[0].device_id, device_id);

    insta::with_settings!({ filters => vec![EXTERNAL_DEVICE_ID_FILTER] }, {
        insta::assert_snapshot!(mutation_effects(&effects));
    });
}

#[test]
fn set_external_device_enabled_with_the_current_state_is_a_no_op() {
    let (mut core, device_id) = core_with_external_device();

    let input = SetExternalDeviceEnabled::enabled(device_id);
    let Update { effects, response } = core.apply(input, Timestamp::from_millis(0));
    assert_eq!(response, Ok(()));
    assert!(effects.is_empty());
}

#[test]
fn set_external_device_enabled_with_an_unknown_id_is_a_no_op() {
    let mut core = Core::new(SettingsSnapshot::default());
    let unknown = ExternalDeviceId(u32::MAX);

    let input = SetExternalDeviceEnabled::enabled(unknown);
    let Update { effects, response } = core.apply(input, Timestamp::from_millis(0));
    assert_eq!(response, Err(UnknownExternalDevice { device_id: unknown }));
    assert!(effects.is_empty());
}
