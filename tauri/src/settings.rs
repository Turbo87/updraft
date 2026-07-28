use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_ok;
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
}
