//! A minimal INI reader for the files that Condor and this tool use.
//!
//! Sections are `[Name]` lines. Entries are `Key=Value` lines. Lines that
//! start with `;` or `#` are comments. Section and key lookups ignore
//! letter case. Values keep their case and lose surrounding whitespace.

pub struct Ini {
    entries: Vec<Entry>,
}

struct Entry {
    section: String,
    key: String,
    value: String,
}

impl Ini {
    pub fn parse(text: &str) -> Self {
        let mut section = String::new();
        let mut entries = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                continue;
            }
            if let Some(name) = line
                .strip_prefix('[')
                .and_then(|rest| rest.strip_suffix(']'))
            {
                section = name.trim().to_owned();
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                entries.push(Entry {
                    section: section.clone(),
                    key: key.trim().to_owned(),
                    value: value.trim().to_owned(),
                });
            }
        }
        Self { entries }
    }

    /// The value of the last entry with this section and key.
    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .rev()
            .find(|entry| {
                entry.section.eq_ignore_ascii_case(section) && entry.key.eq_ignore_ascii_case(key)
            })
            .map(|entry| entry.value.as_str())
    }

    /// The value of the last entry with this key in any section.
    pub fn get_any(&self, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .rev()
            .find(|entry| entry.key.eq_ignore_ascii_case(key))
            .map(|entry| entry.value.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some_eq};

    #[test]
    fn reads_sections_keys_and_comments() {
        let ini = Ini::parse(
            "; comment\n[General]\nEnabled=1\n# another\n[Connection]\nHost = 127.0.0.1 \nPort=55278\nnoise\n",
        );
        assert_some_eq!(ini.get("general", "enabled"), "1");
        assert_some_eq!(ini.get("Connection", "Host"), "127.0.0.1");
        assert_some_eq!(ini.get("Connection", "PORT"), "55278");
        assert_none!(ini.get("General", "Port"));
        assert_some_eq!(ini.get_any("port"), "55278");
    }

    #[test]
    fn keeps_the_last_duplicate_and_empty_values() {
        let ini = Ini::parse("[A]\nKey=1\nKey=2\nEmpty=\n");
        assert_some_eq!(ini.get("A", "Key"), "2");
        assert_some_eq!(ini.get("A", "Empty"), "");
    }
}
