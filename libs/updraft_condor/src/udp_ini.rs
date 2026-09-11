//! The check of Condor's `UDP.ini`, and the edit that enables the values
//! this tool needs.

use crate::ini::Ini;
use anyhow::{Context as _, Result};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

/// Condor's default is one millisecond. This tool reads one value, so a
/// new file starts at ten datagrams per second.
const NEW_FILE_INTERVAL_MS: u32 = 100;

#[derive(Debug, PartialEq)]
pub enum Problem {
    Missing,
    Disabled,
    ExtendedData1Off,
    /// The file sends to another address than the tool listens on.
    Address(String),
}

impl std::fmt::Display for Problem {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing => formatter.write_str("the file does not exist"),
            Self::Disabled => formatter.write_str("UDP output is disabled (Enabled=1 is required)"),
            Self::ExtendedData1Off => {
                formatter.write_str("the MacCready value is not sent (ExtendedData1=1 is required)")
            }
            Self::Address(address) => write!(formatter, "it sends to {address} instead"),
        }
    }
}

pub fn path(condor_folder: &Path) -> PathBuf {
    condor_folder.join("Settings").join("UDP.ini")
}

/// The problems that keep the tool from receiving the MacCready value.
pub fn check(path: &Path, expected: SocketAddr) -> Vec<Problem> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return vec![Problem::Missing];
    };
    let ini = Ini::parse(&text);
    let mut problems = Vec::new();
    if ini.get("General", "Enabled") != Some("1") {
        problems.push(Problem::Disabled);
    }
    if ini.get("Misc", "ExtendedData1") != Some("1") {
        problems.push(Problem::ExtendedData1Off);
    }
    let host = ini.get("Connection", "Host").unwrap_or_default();
    let port = ini.get("Connection", "Port").unwrap_or_default();
    let address = format!("{host}:{port}");
    if address.parse::<SocketAddr>().ok() != Some(expected) {
        problems.push(Problem::Address(address));
    }
    problems
}

/// Writes the values the tool needs. Other lines of an existing file stay
/// as they are.
pub fn enable(path: &Path, expected: SocketAddr) -> Result<()> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => format!(
            "[General]\r\n\r\n[Connection]\r\n\r\n[Misc]\r\nSendIntervalMs={NEW_FILE_INTERVAL_MS}\r\n"
        ),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", path.display()));
        }
    };
    let text = set_key(&text, "General", "Enabled", "1");
    let text = set_key(&text, "Connection", "Host", &expected.ip().to_string());
    let text = set_key(&text, "Connection", "Port", &expected.port().to_string());
    let text = set_key(&text, "Misc", "ExtendedData1", "1");
    std::fs::write(path, text).with_context(|| format!("failed to write {}", path.display()))
}

/// Sets one key, keeping every other line. A missing key is added at the
/// end of its section, and a missing section is added at the end.
fn set_key(text: &str, section: &str, key: &str, value: &str) -> String {
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let mut current = String::new();
    let mut section_start = None;
    let mut section_end = None;
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if let Some(name) = trimmed
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            if section_start.is_some() && section_end.is_none() {
                section_end = Some(index);
            }
            current = name.trim().to_owned();
            if current.eq_ignore_ascii_case(section) && section_start.is_none() {
                section_start = Some(index);
            }
            continue;
        }
        if !current.eq_ignore_ascii_case(section) {
            continue;
        }
        if let Some((existing_key, _)) = trimmed.split_once('=')
            && existing_key.trim().eq_ignore_ascii_case(key)
        {
            lines[index] = format!("{key}={value}");
            return join(&lines, newline);
        }
    }

    match section_start {
        Some(start) => {
            let mut insert_at = section_end.unwrap_or(lines.len());
            while insert_at > start + 1 && lines[insert_at - 1].trim().is_empty() {
                insert_at -= 1;
            }
            lines.insert(insert_at, format!("{key}={value}"));
        }
        None => {
            if lines.last().is_some_and(|line| !line.trim().is_empty()) {
                lines.push(String::new());
            }
            lines.push(format!("[{section}]"));
            lines.push(format!("{key}={value}"));
        }
    }
    join(&lines, newline)
}

fn join(lines: &[String], newline: &str) -> String {
    let mut text = lines.join(newline);
    text.push_str(newline);
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_ok;

    const EXPECTED: &str = "127.0.0.1:55278";
    const MANUAL: &str = "[General]\r\nEnabled=1\r\n[Connection]\r\nHost=127.0.0.1\r\nPort=55278\r\n[Misc]\r\nSendIntervalMs=1\r\nExtendedData=0\r\nExtendedData1=0\r\nLogToFile=0\r\n";

    fn expected() -> SocketAddr {
        EXPECTED.parse().unwrap()
    }

    #[test]
    fn reports_every_problem() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("UDP.ini");
        assert_eq!(check(&path, expected()), vec![Problem::Missing]);

        std::fs::write(&path, MANUAL).unwrap();
        assert_eq!(check(&path, expected()), vec![Problem::ExtendedData1Off]);

        std::fs::write(
            &path,
            "[General]\nEnabled=0\n[Connection]\nHost=192.168.1.5\nPort=4000\n[Misc]\nExtendedData1=1\n",
        )
        .unwrap();
        assert_eq!(
            check(&path, expected()),
            vec![
                Problem::Disabled,
                Problem::Address("192.168.1.5:4000".to_owned())
            ]
        );
    }

    #[test]
    fn enables_the_manual_file_and_keeps_the_other_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("UDP.ini");
        std::fs::write(&path, MANUAL).unwrap();
        assert_ok!(enable(&path, expected()));
        insta::assert_snapshot!(std::fs::read_to_string(&path).unwrap());
        assert!(check(&path, expected()).is_empty());
    }

    #[test]
    fn creates_a_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("UDP.ini");
        assert_ok!(enable(&path, expected()));
        insta::assert_snapshot!(std::fs::read_to_string(&path).unwrap());
        assert!(check(&path, expected()).is_empty());
    }

    #[test]
    fn adds_missing_keys_and_sections_in_place() {
        let text = "[General]\nEnabled=0\n\n[Misc]\nLogToFile=0\n";
        let text = set_key(text, "general", "enabled", "1");
        let text = set_key(&text, "Misc", "ExtendedData1", "1");
        let text = set_key(&text, "Connection", "Port", "55278");
        assert_eq!(
            text,
            "[General]\nenabled=1\n\n[Misc]\nLogToFile=0\nExtendedData1=1\n\n[Connection]\nPort=55278\n"
        );
    }
}
