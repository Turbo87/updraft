use std::fs::File;
use std::io::{BufReader, Write};
use std::path::PathBuf;
use tempfile::NamedTempFile;
use updraft_core::SettingsSnapshot;

const FILE_NAME: &str = "settings.json";

#[derive(Clone, Debug)]
pub struct SettingsFile {
    path: PathBuf,
}

impl SettingsFile {
    pub fn new(config_dir: impl Into<PathBuf>) -> Self {
        Self {
            path: config_dir.into().join(FILE_NAME),
        }
    }

    pub fn load(&self) -> SettingsSnapshot {
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return SettingsSnapshot::default();
            }
            Err(error) => {
                tracing::warn!(path = %self.path.display(), %error, "Could not load settings");
                return SettingsSnapshot::default();
            }
        };

        serde_json::from_reader(BufReader::new(file)).unwrap_or_else(|error| {
            tracing::warn!(path = %self.path.display(), %error, "Could not load settings");
            SettingsSnapshot::default()
        })
    }

    fn write(&self, snapshot: SettingsSnapshot) -> std::io::Result<()> {
        let directory = self
            .path
            .parent()
            .expect("settings path always includes its configuration directory");
        std::fs::create_dir_all(directory)?;

        let mut temporary = NamedTempFile::new_in(directory)?;
        serde_json::to_writer_pretty(&mut temporary, &snapshot).map_err(std::io::Error::other)?;
        writeln!(temporary)?;
        temporary.persist(&self.path).map_err(|error| error.error)?;

        Ok(())
    }

    pub fn writer(self) -> impl Fn(SettingsSnapshot) + Send + 'static {
        let (sender, receiver) = std::sync::mpsc::channel();

        tauri::async_runtime::spawn_blocking(move || {
            for snapshot in receiver {
                if let Err(error) = self.write(snapshot) {
                    tracing::warn!(
                        path = %self.path.display(),
                        %error,
                        "Could not persist settings"
                    );
                }
            }
        });

        move |snapshot| {
            if sender.send(snapshot).is_err() {
                tracing::warn!("Could not queue settings persistence");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};
    use tempfile::tempdir;
    use tracing_test::traced_test;
    use updraft_core::{
        AltitudeUnit, ConnectionSpec, DistanceUnit, ExternalDeviceConfig, Locale,
        STANDARD_SPP_SERVICE_UUID, Settings, SettingsSnapshot, SpeedUnit, UnitSettings,
        VerticalSpeedUnit,
    };
    use uuid::uuid;

    #[test]
    fn missing_file_loads_default_settings() {
        let directory = assert_ok!(tempdir());
        let file = SettingsFile::new(directory.path());

        assert_eq!(file.load(), SettingsSnapshot::default());
        assert!(!directory.path().join("settings.json").exists());
    }

    #[test]
    fn file_without_units_keeps_locale_and_external_devices() {
        let directory = assert_ok!(tempdir());
        assert_ok!(std::fs::write(
            directory.path().join("settings.json"),
            concat!(
                "{\"locale\":\"de\",\"externalDevices\":[{",
                "\"enabled\":true,",
                "\"type\":\"tcp\",",
                "\"host\":\"127.0.0.1\",",
                "\"port\":4353",
                "}]}"
            ),
        ));
        let file = SettingsFile::new(directory.path());

        assert_eq!(
            file.load(),
            SettingsSnapshot {
                settings: Settings {
                    locale: Some(Locale::De),
                    units: UnitSettings::default(),
                    ..Settings::default()
                },
                external_devices: vec![ExternalDeviceConfig {
                    enabled: true,
                    spec: ConnectionSpec::tcp("127.0.0.1", 4353),
                }],
            }
        );
    }

    #[test]
    fn partial_units_file_defaults_missing_selections() {
        let directory = assert_ok!(tempdir());
        assert_ok!(std::fs::write(
            directory.path().join("settings.json"),
            r#"{"locale":"de","units":{"altitude":"ft"}}"#,
        ));
        let file = SettingsFile::new(directory.path());

        assert_eq!(
            file.load(),
            SettingsSnapshot {
                settings: Settings {
                    locale: Some(Locale::De),
                    units: UnitSettings {
                        altitude: AltitudeUnit::Feet,
                        ..UnitSettings::default()
                    },
                    ..Settings::default()
                },
                external_devices: Vec::new(),
            }
        );
    }

    #[test]
    fn bluetooth_without_service_uuid_loads_the_standard_uuid() {
        let directory = assert_ok!(tempdir());
        assert_ok!(std::fs::write(
            directory.path().join("settings.json"),
            concat!(
                "{\"externalDevices\":[{",
                "\"enabled\":true,",
                "\"type\":\"bluetooth\",",
                "\"address\":\"00:11:22:33:44:55\"",
                "}]}"
            ),
        ));
        let file = SettingsFile::new(directory.path());

        assert_eq!(
            file.load(),
            SettingsSnapshot {
                settings: Settings::default(),
                external_devices: vec![ExternalDeviceConfig {
                    enabled: true,
                    spec: ConnectionSpec::BluetoothSpp {
                        address: "00:11:22:33:44:55".to_owned(),
                        service_uuid: STANDARD_SPP_SERVICE_UUID,
                    },
                }],
            }
        );
    }

    #[test]
    #[traced_test]
    fn bluetooth_with_invalid_service_uuid_warns_and_loads_defaults() {
        let directory = assert_ok!(tempdir());
        assert_ok!(std::fs::write(
            directory.path().join("settings.json"),
            concat!(
                "{\"externalDevices\":[{",
                "\"enabled\":true,",
                "\"type\":\"bluetooth\",",
                "\"address\":\"00:11:22:33:44:55\",",
                "\"serviceUuid\":\"invalid\"",
                "}]}"
            ),
        ));
        let file = SettingsFile::new(directory.path());

        assert_eq!(file.load(), SettingsSnapshot::default());
        assert!(logs_contain("Could not load settings"));
    }

    #[test]
    #[traced_test]
    fn malformed_file_warns_and_loads_defaults() {
        let directory = assert_ok!(tempdir());
        assert_ok!(std::fs::write(
            directory.path().join("settings.json"),
            r#"{"locale":"es"}"#,
        ));
        let file = SettingsFile::new(directory.path());

        assert_eq!(file.load(), SettingsSnapshot::default());
        assert!(logs_contain("Could not load settings"));
        assert_eq!(
            assert_ok!(std::fs::read_to_string(
                directory.path().join("settings.json")
            )),
            r#"{"locale":"es"}"#
        );
    }

    #[test]
    #[traced_test]
    fn unknown_unit_warns_and_loads_defaults() {
        let directory = assert_ok!(tempdir());
        assert_ok!(std::fs::write(
            directory.path().join("settings.json"),
            r#"{"locale":"de","units":{"altitude":"yards"}}"#,
        ));
        let file = SettingsFile::new(directory.path());

        assert_eq!(file.load(), SettingsSnapshot::default());
        assert!(logs_contain("Could not load settings"));
        assert_eq!(
            assert_ok!(std::fs::read_to_string(
                directory.path().join("settings.json")
            )),
            r#"{"locale":"de","units":{"altitude":"yards"}}"#
        );
    }

    #[test]
    fn writing_creates_the_directory_and_settings_file() {
        let parent = assert_ok!(tempdir());
        let config_dir = parent.path().join("missing");
        let file = SettingsFile::new(&config_dir);
        let snapshot = SettingsSnapshot {
            settings: Settings {
                locale: Some(Locale::De),
                polar: assert_ok!(updraft_core::PolarId::try_from("LS 8-18".to_owned())),
                units: UnitSettings {
                    altitude: AltitudeUnit::Feet,
                    distance: DistanceUnit::NauticalMiles,
                    speed: SpeedUnit::Knots,
                    vertical_speed: VerticalSpeedUnit::FeetPerMinute,
                },
            },
            external_devices: vec![
                ExternalDeviceConfig {
                    enabled: true,
                    spec: ConnectionSpec::tcp("127.0.0.1", 4353),
                },
                ExternalDeviceConfig {
                    enabled: false,
                    spec: ConnectionSpec::bluetooth_spp("00:11:22:33:44:55"),
                },
                ExternalDeviceConfig {
                    enabled: true,
                    spec: ConnectionSpec::BluetoothSpp {
                        address: "00:11:22:33:44:66".to_owned(),
                        service_uuid: uuid!("e56617bf-f548-4f7c-9cef-4a26eec19b04"),
                    },
                },
            ],
        };

        assert_ok!(file.write(snapshot.clone()));
        let contents = assert_ok!(std::fs::read_to_string(config_dir.join("settings.json")));
        insta::assert_snapshot!(contents);
        assert_eq!(file.load(), snapshot);
    }

    #[test]
    fn writing_atomically_replaces_the_previous_snapshot() {
        let directory = assert_ok!(tempdir());
        let file = SettingsFile::new(directory.path());

        assert_ok!(file.write(SettingsSnapshot {
            settings: Settings {
                locale: Some(Locale::De),
                ..Settings::default()
            },
            external_devices: vec![ExternalDeviceConfig {
                enabled: true,
                spec: ConnectionSpec::tcp("127.0.0.1", 4353),
            }],
        }));
        assert_ok!(file.write(SettingsSnapshot {
            settings: Settings {
                locale: Some(Locale::En),
                ..Settings::default()
            },
            external_devices: Vec::new(),
        }));

        assert_eq!(
            file.load(),
            SettingsSnapshot {
                settings: Settings {
                    locale: Some(Locale::En),
                    ..Settings::default()
                },
                external_devices: Vec::new(),
            }
        );
    }

    #[test]
    fn writing_fails_when_the_configuration_path_is_not_a_directory() {
        let directory = assert_ok!(tempdir());
        let config_dir = directory.path().join("not-a-directory");
        assert_ok!(std::fs::write(&config_dir, b"file"));
        let file = SettingsFile::new(config_dir);

        assert_err!(file.write(SettingsSnapshot::default()));
    }
}
