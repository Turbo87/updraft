//! Enroute basemap and terrain catalog parsing and country assignments.

use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::num::NonZeroU64;
use time::{Date, macros::format_description};

pub mod catalog;
pub mod commands;
pub mod download;
pub mod queue;
mod regions;
pub mod storage;

fn http_client() -> reqwest::ClientBuilder {
    let roots = rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };
    let tls = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    reqwest::Client::builder().use_preconfigured_tls(tls)
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogEntry {
    pub path: &'static str,
    pub country_code: &'static str,
    pub continent: Continent,
    pub size: NonZeroU64,
    #[serde(serialize_with = "serialize_publication_date")]
    pub publication_date: Date,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Continent {
    Africa,
    Asia,
    Oceania,
    Europe,
    NorthAmerica,
    SouthAmerica,
}

#[derive(Deserialize)]
struct Catalog {
    maps: Vec<Value>,
}

#[derive(Deserialize)]
struct Entry {
    size: NonZeroU64,
    time: String,
}

/// Returns supported basemaps and terrain files in catalog-path order. Unknown paths are omitted.
///
/// Malformed catalog structure, invalid supported entries, and duplicate
/// supported paths return an error. Download locations come from the bundled
/// mapping, not the catalog's base URL.
pub fn parse_catalog(bytes: &[u8]) -> Result<Vec<CatalogEntry>> {
    let catalog: Catalog = serde_json::from_slice(bytes).context("Invalid Enroute catalog")?;
    let mut files = BTreeMap::new();
    for value in catalog.maps {
        let path = value
            .get("path")
            .and_then(Value::as_str)
            .context("Missing Enroute catalog path")?;
        let Some(&(path, country_code, continent)) =
            regions::REGIONS.iter().find(|(known, _, _)| *known == path)
        else {
            continue;
        };
        let entry: Entry = serde_json::from_value(value)
            .with_context(|| format!("Invalid Enroute entry: {path}"))?;
        ensure!(
            entry.time.len() == 8 && entry.time.bytes().all(|byte| byte.is_ascii_digit()),
            "Invalid Enroute date: {path}"
        );
        let publication_date = Date::parse(&entry.time, format_description!("[year][month][day]"))
            .with_context(|| format!("Invalid Enroute date: {path}"))?;
        let file = CatalogEntry {
            path,
            country_code,
            continent,
            size: entry.size,
            publication_date,
        };
        ensure!(
            files.insert(path, file).is_none(),
            "Duplicate Enroute path: {path}"
        );
    }
    Ok(files.into_values().collect())
}

fn serialize_publication_date<S: serde::Serializer>(
    date: &Date,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_str(date)
}

#[cfg(test)]
mod tests;
