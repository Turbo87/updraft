# Configurable Bluetooth service UUID

## Purpose

Updraft uses the standard SPP UUID for each Android Bluetooth connection.
The macOS NMEA simulator must use a custom RFCOMM service UUID.

This change lets one Bluetooth configuration select a service UUID.
The JSON field is optional.

## Glossary

- **RFCOMM**: The Bluetooth Classic protocol that supplies a byte stream.
- **Service UUID**: A value that identifies one Bluetooth service.
- **SPP**: The Serial Port Profile that uses RFCOMM.

## Scope

This change provides these items:

- One optional `serviceUuid` field for a Bluetooth connection.
- The standard SPP UUID when the field is absent.
- The configured UUID when the field is present.
- A secure RFCOMM socket for both UUID choices.
- Existing retry, cancellation, and event behavior.
- Automated tests for settings and UUID selection.
- A physical test with the macOS NMEA simulator.

This change does not provide these items:

- A configuration user interface.
- More than one pending or active Android SPP attempt.
- Bluetooth discovery or pairing controls.
- BLE support.
- Insecure RFCOMM fallback.

## Stored configuration

The stored Bluetooth object adds one optional JSON field.
`ConnectionSpec` stores the selected UUID as a required `Uuid`:

```rust
use uuid::{Uuid, uuid};

pub const STANDARD_SPP_SERVICE_UUID: Uuid =
    uuid!("00001101-0000-1000-8000-00805F9B34FB");

#[serde(tag = "type", rename_all_fields = "camelCase")]
pub enum ConnectionSpec {
    #[serde(rename = "tcp")]
    Tcp { host: String, port: u16 },

    #[serde(rename = "bluetooth")]
    BluetoothSpp {
        address: String,

        #[serde(
            default = "default_spp_service_uuid",
            skip_serializing_if = "is_standard_spp_service_uuid"
        )]
        service_uuid: Uuid,
    },
}

fn default_spp_service_uuid() -> Uuid {
    STANDARD_SPP_SERVICE_UUID
}

fn is_standard_spp_service_uuid(service_uuid: &Uuid) -> bool {
    *service_uuid == STANDARD_SPP_SERVICE_UUID
}
```

The default function returns `STANDARD_SPP_SERVICE_UUID`.
The comparison function checks for the same value.

The existing `bluetooth_spp()` constructor sets the standard UUID.
Code can construct the variant directly for a custom service.

The `uuid` crate parses the JSON field during deserialization.
Its Serde implementation writes a lowercase, hyphenated string.

The generated TypeScript field is `serviceUuid?: string`.
The `ts-rs` UUID implementation maps `Uuid` to `string`.
The Serde attributes make this field optional.

An existing standard SPP row stays unchanged:

```json
{
  "enabled": true,
  "type": "bluetooth",
  "address": "00:11:22:33:44:55"
}
```

A macOS simulator row includes the custom service:

```json
{
  "enabled": true,
  "type": "bluetooth",
  "address": "00:11:22:33:44:55",
  "serviceUuid": "e56617bf-f548-4f7c-9cef-4a26eec19b04"
}
```

The address in these examples is not a real device address.

## Data flow

The core sends the selected `Uuid` through the existing Rust SPP transport.
The Rust mobile plugin keeps this type in `StartSppAttemptArgs`.
Serde converts the `Uuid` to a string in the mobile invocation payload.
Kotlin sends the string to `SessionService`.
`SessionService` sends the string to `SppSource`.

The change does not change SPP attempt ownership.
Android still permits only one pending or active SPP attempt.

## Android socket selection

`SppSource` receives the selected socket UUID.
It uses this sequence:

1. Parse `serviceUuid` with `UUID.fromString()`.
2. Create a secure socket with `createRfcommSocketToServiceRecord()`.

The standard SPP UUID is:

`00001101-0000-1000-8000-00805F9B34FB`

## Error behavior

An invalid UUID makes the complete settings file invalid.
`SettingsFile` writes a warning and loads the default settings snapshot.
It does not replace an invalid UUID with the standard SPP UUID.
An invalid UUID does not reach the Android worker.

Routine diagnostics do not contain NMEA payload data.

## Compatibility

Existing settings files do not contain `serviceUuid`.
Serde replaces the absent field with the typed standard SPP UUID.
Serialization omits the field when it contains the standard SPP UUID.

Rust stores and sends the selected UUID as `Uuid`.
The core, driver, and Rust mobile plugin do not contain raw UUID strings.
Serialization normalizes a configured UUID to lowercase hyphenated form.

The ESP32 simulator continues to use the standard SPP UUID.
Its stored configuration does not require a change.

## Automated tests

Rust tests check these behaviors:

- Old Bluetooth JSON loads the standard SPP UUID.
- Invalid Bluetooth UUID JSON makes the settings file invalid.
- Standard Bluetooth JSON does not contain `serviceUuid`.
- Custom Bluetooth JSON contains the normalized UUID.
- Settings round trips preserve the selected UUID.
- The generated TypeScript field is optional.
- The transport sends the selected `Uuid` to the mobile plugin boundary.

Android tests check these behaviors:

- The standard SPP UUID creates the standard socket request.
- A custom UUID creates the custom socket request.
- An invalid value causes an error.
- The socket remains secure.

Existing SPP retry, cancellation, and event tests must stay green.

## Physical acceptance

The physical test uses one SPP connection at a time.

First, Updraft connects to the ESP32 without `serviceUuid`.
The test checks the connection and the first received bytes.

Next, Updraft connects to the Mac with the custom UUID.
The test checks the connection and the first received bytes.
The test keeps the connection active through the 301-second playback loop.

The test then restarts Updraft.
The new Mac connection must start with batch one.

The logs must not contain NMEA payload data.

## Limits

This proof does not test parallel SPP connections.
It provides the second physical source for later multi-SPP work.
