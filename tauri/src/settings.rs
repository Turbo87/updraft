use std::fs::File;
use std::io::{BufReader, Write};
use std::path::PathBuf;
use tempfile::NamedTempFile;
use updraft_core::Settings;

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

    pub fn load(&self) -> Settings {
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Settings::default();
            }
            Err(error) => {
                tracing::warn!(path = %self.path.display(), %error, "Could not load settings");
                return Settings::default();
            }
        };

        serde_json::from_reader(BufReader::new(file)).unwrap_or_else(|error| {
            tracing::warn!(path = %self.path.display(), %error, "Could not load settings");
            Settings::default()
        })
    }

    fn write(&self, settings: Settings) -> std::io::Result<()> {
        let directory = self
            .path
            .parent()
            .expect("settings path always includes its configuration directory");
        std::fs::create_dir_all(directory)?;

        let mut temporary = NamedTempFile::new_in(directory)?;
        serde_json::to_writer_pretty(&mut temporary, &settings).map_err(std::io::Error::other)?;
        writeln!(temporary)?;
        temporary.persist(&self.path).map_err(|error| error.error)?;

        Ok(())
    }

    pub fn writer(self) -> impl Fn(Settings) + Send + 'static {
        let (sender, receiver) = std::sync::mpsc::channel();

        tauri::async_runtime::spawn_blocking(move || {
            for settings in receiver {
                if let Err(error) = self.write(settings) {
                    tracing::warn!(
                        path = %self.path.display(),
                        %error,
                        "Could not persist settings"
                    );
                }
            }
        });

        move |settings| {
            if sender.send(settings).is_err() {
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
    use updraft_core::Locale;

    #[test]
    fn missing_file_loads_default_settings() {
        let directory = assert_ok!(tempdir());
        let file = SettingsFile::new(directory.path());

        assert_eq!(file.load(), Settings::default());
        assert!(!directory.path().join("settings.json").exists());
    }

    #[test]
    fn valid_file_loads_the_explicit_locale() {
        let directory = assert_ok!(tempdir());
        assert_ok!(std::fs::write(
            directory.path().join("settings.json"),
            r#"{"locale":"de"}"#,
        ));
        let file = SettingsFile::new(directory.path());

        assert_eq!(
            file.load(),
            Settings {
                locale: Some(Locale::De),
            }
        );
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

        assert_eq!(file.load(), Settings::default());
        assert!(logs_contain("Could not load settings"));
        assert_eq!(
            assert_ok!(std::fs::read_to_string(
                directory.path().join("settings.json")
            )),
            r#"{"locale":"es"}"#
        );
    }

    #[test]
    fn writing_creates_the_directory_and_settings_file() {
        let parent = assert_ok!(tempdir());
        let config_dir = parent.path().join("missing");
        let file = SettingsFile::new(&config_dir);
        let settings = Settings {
            locale: Some(Locale::De),
        };

        assert_ok!(file.write(settings));
        assert_eq!(file.load(), settings);
        assert!(config_dir.join("settings.json").exists());
    }

    #[test]
    fn writing_atomically_replaces_the_previous_snapshot() {
        let directory = assert_ok!(tempdir());
        let file = SettingsFile::new(directory.path());

        assert_ok!(file.write(Settings {
            locale: Some(Locale::De),
        }));
        assert_ok!(file.write(Settings {
            locale: Some(Locale::En),
        }));

        assert_eq!(
            file.load(),
            Settings {
                locale: Some(Locale::En),
            }
        );
    }

    #[test]
    fn writing_fails_when_the_configuration_path_is_not_a_directory() {
        let directory = assert_ok!(tempdir());
        let config_dir = directory.path().join("not-a-directory");
        assert_ok!(std::fs::write(&config_dir, b"file"));
        let file = SettingsFile::new(config_dir);

        assert_err!(file.write(Settings::default()));
    }
}
