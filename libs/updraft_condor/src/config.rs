//! The `updraft_condor.ini` file next to the executable, and the detection
//! of the values it leaves empty.

use crate::ini::Ini;
use anyhow::{Context as _, Result};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

pub const FILE_NAME: &str = "updraft_condor.ini";

const DEFAULT_NMEA_LISTEN: &str = "127.0.0.1:4353";
const DEFAULT_UDP_LISTEN: &str = "127.0.0.1:55278";
const DEFAULT_OUTPUT_LISTEN: &str = "0.0.0.0:4354";

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    /// The Condor 3 installation folder. `None` asks for detection.
    pub condor_folder: Option<PathBuf>,
    /// The own competition number. `None` asks for detection.
    pub competition_number: Option<String>,
    /// Where HW VSP3 delivers the Condor NMEA stream.
    pub nmea_listen: SocketAddr,
    /// Where Condor sends its UDP datagrams.
    pub udp_listen: SocketAddr,
    /// Where Updraft connects.
    pub output_listen: SocketAddr,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            condor_folder: None,
            competition_number: None,
            nmea_listen: DEFAULT_NMEA_LISTEN.parse().unwrap(),
            udp_listen: DEFAULT_UDP_LISTEN.parse().unwrap(),
            output_listen: DEFAULT_OUTPUT_LISTEN.parse().unwrap(),
        }
    }
}

impl Config {
    /// Reads the file, or writes the defaults when it does not exist.
    pub fn load_or_create(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::parse(&text).with_context(|| format!("invalid {}", path.display())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let config = Self::default();
                config.save(path)?;
                Ok(config)
            }
            Err(error) => Err(error).with_context(|| format!("failed to read {}", path.display())),
        }
    }

    pub fn parse(text: &str) -> Result<Self> {
        let ini = Ini::parse(text);
        let defaults = Self::default();
        let address = |section: &str, key: &str, default: SocketAddr| -> Result<SocketAddr> {
            match ini.get(section, key).filter(|value| !value.is_empty()) {
                Some(value) => value
                    .parse()
                    .with_context(|| format!("{section}.{key} is not an address: {value}")),
                None => Ok(default),
            }
        };
        let text_value = |section: &str, key: &str| {
            ini.get(section, key)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
        };

        Ok(Self {
            condor_folder: text_value("Condor", "Folder").map(PathBuf::from),
            competition_number: text_value("Condor", "CompetitionNumber"),
            nmea_listen: address("Input", "NmeaListen", defaults.nmea_listen)?,
            udp_listen: address("Input", "UdpListen", defaults.udp_listen)?,
            output_listen: address("Output", "NmeaListen", defaults.output_listen)?,
        })
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        std::fs::write(path, self.to_ini())
            .with_context(|| format!("failed to write {}", path.display()))
    }

    pub fn to_ini(&self) -> String {
        let folder = self
            .condor_folder
            .as_deref()
            .map(|path| path.display().to_string())
            .unwrap_or_default();
        let competition_number = self.competition_number.as_deref().unwrap_or_default();
        format!(
            "[Condor]\n\
             ; Condor 3 installation folder. Detected when empty.\n\
             Folder={folder}\n\
             ; Own competition number for the Spectate file. Detected from the pilot profile when empty.\n\
             CompetitionNumber={competition_number}\n\
             \n\
             [Input]\n\
             ; HW VSP3 connects here with the NMEA stream from Condor.\n\
             NmeaListen={}\n\
             ; Must match Host and Port in Condor's UDP.ini.\n\
             UdpListen={}\n\
             \n\
             [Output]\n\
             ; Updraft connects here. 0.0.0.0 accepts connections from other devices.\n\
             NmeaListen={}\n",
            self.nmea_listen, self.udp_listen, self.output_listen
        )
    }
}

/// A Condor folder contains the `Settings` folder that Condor creates.
pub fn is_condor_folder(path: &Path) -> bool {
    path.join("Settings").is_dir()
}

/// The default Condor 3 installation folder, when it exists.
pub fn detect_condor_folder() -> Option<PathBuf> {
    let candidate = PathBuf::from(r"C:\Condor3");
    is_condor_folder(&candidate).then_some(candidate)
}

/// The result of looking for the competition number in the pilot profiles.
#[derive(Debug, PartialEq)]
pub enum CompetitionNumber {
    Found(String),
    /// No profile folder, no profile, or no competition number in the
    /// single profile.
    NotFound,
    /// More than one profile. The user has to choose.
    Ambiguous(usize),
}

/// Reads the competition number from `Condor3\Pilots\<profile>\pilot.ini`
/// under the Documents folder when exactly one profile exists.
pub fn detect_competition_number(documents: &Path) -> CompetitionNumber {
    let pilots = documents.join("Condor3").join("Pilots");
    let Ok(entries) = std::fs::read_dir(&pilots) else {
        return CompetitionNumber::NotFound;
    };
    let profiles: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    match profiles.as_slice() {
        [] => CompetitionNumber::NotFound,
        [profile] => competition_number_from_profile(profile)
            .map_or(CompetitionNumber::NotFound, CompetitionNumber::Found),
        profiles => CompetitionNumber::Ambiguous(profiles.len()),
    }
}

fn competition_number_from_profile(profile: &Path) -> Option<String> {
    let text = std::fs::read_to_string(profile.join("pilot.ini")).ok()?;
    let ini = Ini::parse(&text);
    ini.get_any("CN")
        .or_else(|| ini.get_any("CompetitionNumber"))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok, assert_some_eq};

    #[test]
    fn defaults_round_trip_through_the_file() {
        let config = Config::default();
        insta::assert_snapshot!(config.to_ini());
        assert_ok!(Config::parse(&config.to_ini()));
        assert_eq!(assert_ok!(Config::parse(&config.to_ini())), config);
    }

    #[test]
    fn file_values_override_defaults() {
        let text = "[Condor]\nFolder=D:\\Games\\Condor3\nCompetitionNumber=XY\n[Output]\nNmeaListen=0.0.0.0:5000\n";
        let config = assert_ok!(Config::parse(text));
        assert_some_eq!(
            config.condor_folder.clone(),
            PathBuf::from("D:\\Games\\Condor3")
        );
        assert_some_eq!(config.competition_number.clone(), "XY".to_owned());
        assert_eq!(config.output_listen, "0.0.0.0:5000".parse().unwrap());
        assert_eq!(config.nmea_listen, Config::default().nmea_listen);
        assert_eq!(assert_ok!(Config::parse(&config.to_ini())), config);
    }

    #[test]
    fn rejects_an_invalid_address() {
        assert_err!(Config::parse("[Input]\nNmeaListen=localhost\n"));
    }

    #[test]
    fn creates_the_file_on_first_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        let config = assert_ok!(Config::load_or_create(&path));
        assert_eq!(config, Config::default());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), config.to_ini());
    }

    #[test]
    fn detects_the_competition_number_from_a_single_profile() {
        let documents = tempfile::tempdir().unwrap();
        let profile = documents.path().join("Condor3/Pilots/Doe_Jane");
        std::fs::create_dir_all(&profile).unwrap();
        std::fs::write(profile.join("pilot.ini"), "[Pilot]\nCN=JD\n").unwrap();
        assert_eq!(
            detect_competition_number(documents.path()),
            CompetitionNumber::Found("JD".to_owned())
        );

        let second = documents.path().join("Condor3/Pilots/Roe_Richard");
        std::fs::create_dir_all(&second).unwrap();
        assert_eq!(
            detect_competition_number(documents.path()),
            CompetitionNumber::Ambiguous(2)
        );
    }

    #[test]
    fn missing_profiles_are_not_found() {
        let documents = tempfile::tempdir().unwrap();
        assert_eq!(
            detect_competition_number(documents.path()),
            CompetitionNumber::NotFound
        );

        let profile = documents.path().join("Condor3/Pilots/Doe_Jane");
        std::fs::create_dir_all(&profile).unwrap();
        std::fs::write(profile.join("pilot.ini"), "[Pilot]\nName=Jane\n").unwrap();
        assert_eq!(
            detect_competition_number(documents.path()),
            CompetitionNumber::NotFound
        );
    }

    #[test]
    fn a_condor_folder_has_a_settings_folder() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!is_condor_folder(dir.path()));
        std::fs::create_dir(dir.path().join("Settings")).unwrap();
        assert!(is_condor_folder(dir.path()));
    }
}
