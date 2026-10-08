use crate::terrain::Terrain;
use rusqlite::{Connection, Transaction};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use updraft_core::{Sample, UtcInstant, Velocity};
use updraft_geo::LatLon;
use updraft_units::{Length, MslAltitude, Speed};

/// Each migration upgrades the database to the `user_version` that is its
/// position plus one.
const MIGRATIONS: &[&str] = &["CREATE TABLE samples (
    utc_ms INTEGER NOT NULL,
    latitude_deg REAL NOT NULL,
    longitude_deg REAL NOT NULL,
    altitude_msl_m REAL,
    vario_mps REAL,
    netto_mps REAL,
    relative_vario_mps REAL,
    terrain_elevation_m REAL,
    wind_east_mps REAL,
    wind_north_mps REAL
) STRICT"];

#[derive(Debug)]
pub enum RecordingWrite {
    StartRecording(Sample),
    RecordSample(Sample),
    DiscardRecording,
}

/// Stores the current flight recording in `state.sqlite`.
pub struct FlightRecording {
    connection: Connection,
    terrain: Arc<Mutex<Terrain>>,
}

impl FlightRecording {
    pub fn open(path: &Path, terrain: Arc<Mutex<Terrain>>) -> anyhow::Result<Self> {
        if let Some(directory) = path.parent() {
            std::fs::create_dir_all(directory)?;
        }
        let mut connection = Connection::open(path)?;
        connection.pragma_update_and_check(None, "journal_mode", "WAL", |_| Ok(()))?;
        connection.pragma_update(None, "synchronous", "NORMAL")?;
        let current: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        for (version, migration) in (1_i64..)
            .zip(MIGRATIONS)
            .skip_while(|(version, _)| *version <= current)
        {
            let transaction = connection.transaction()?;
            transaction.execute_batch(migration)?;
            transaction.pragma_update(None, "user_version", version)?;
            transaction.commit()?;
        }
        Ok(Self {
            connection,
            terrain,
        })
    }

    /// Runs one transaction for the write.
    pub fn write(&mut self, write: &RecordingWrite) -> rusqlite::Result<()> {
        let (deletes_samples, sample) = match write {
            RecordingWrite::StartRecording(sample) => (true, Some(sample)),
            RecordingWrite::RecordSample(sample) => (false, Some(sample)),
            RecordingWrite::DiscardRecording => (true, None),
        };
        let row = sample.map(|sample| (sample, terrain_elevation(&self.terrain, sample)));
        let transaction = self.connection.transaction()?;
        if deletes_samples {
            transaction.execute("DELETE FROM samples", [])?;
        }
        if let Some((sample, terrain_elevation)) = row {
            insert(&transaction, sample, terrain_elevation)?;
        }
        transaction.commit()
    }

    fn samples(&self) -> rusqlite::Result<Vec<Sample>> {
        let mut statement = self.connection.prepare(
            "SELECT utc_ms, latitude_deg, longitude_deg, altitude_msl_m, vario_mps, netto_mps,
                relative_vario_mps, wind_east_mps, wind_north_mps
            FROM samples ORDER BY rowid",
        )?;
        let speed = |value: Option<f64>| value.map(Speed::from_meters_per_second);
        let rows = statement.query_map([], |row| {
            let wind_east = speed(row.get(7)?);
            let wind_north = speed(row.get(8)?);
            Ok(Sample {
                utc: UtcInstant::from_unix_milliseconds(row.get(0)?),
                position: LatLon::from_degrees(row.get(1)?, row.get(2)?),
                altitude_msl: row
                    .get::<_, Option<f64>>(3)?
                    .map(|meters| MslAltitude::new(Length::from_meters(meters))),
                vario: speed(row.get(4)?),
                netto: speed(row.get(5)?),
                relative_vario: speed(row.get(6)?),
                wind: wind_east
                    .zip(wind_north)
                    .map(|(east, north)| Velocity { east, north }),
            })
        })?;
        rows.collect()
    }
}

/// Opens the flight recording and reads its samples for the restore.
///
/// The returned writer stores the recording effects in order on a blocking
/// worker. Without a usable database, the restore is empty and the writer
/// drops all writes.
pub fn load(
    path: PathBuf,
    terrain: Arc<Mutex<Terrain>>,
) -> (Vec<Sample>, impl Fn(RecordingWrite) + Send + 'static) {
    let (recording, samples) = match FlightRecording::open(&path, terrain) {
        Ok(recording) => {
            let samples = recording.samples().unwrap_or_else(|error| {
                tracing::error!(path = %path.display(), %error, "Could not read the flight recording");
                Vec::new()
            });
            (Some(recording), samples)
        }
        Err(error) => {
            tracing::error!(path = %path.display(), %error, "Could not open the flight recording");
            (None, Vec::new())
        }
    };
    let (sender, receiver) = std::sync::mpsc::channel();

    tauri::async_runtime::spawn_blocking(move || {
        let Some(mut recording) = recording else {
            return;
        };
        for write in receiver {
            if let Err(error) = recording.write(&write) {
                tracing::error!(path = %path.display(), %error, "Could not write the flight recording");
            }
        }
    });

    let writer = move |write| {
        let _ = sender.send(write);
    };
    (samples, writer)
}

fn terrain_elevation(terrain: &Mutex<Terrain>, sample: &Sample) -> Option<f64> {
    let terrain = terrain.lock().expect("terrain access should not panic");
    terrain.elevation(sample.position).unwrap_or_else(|error| {
        tracing::warn!(%error, "Could not sample terrain elevation for the flight recording");
        None
    })
}

fn insert(
    transaction: &Transaction<'_>,
    sample: &Sample,
    terrain_elevation: Option<f64>,
) -> rusqlite::Result<()> {
    let meters_per_second = |speed: Option<Speed>| speed.map(Speed::as_meters_per_second);
    transaction.execute(
        "INSERT INTO samples VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        rusqlite::params![
            sample.utc.unix_milliseconds(),
            sample.position.latitude().as_degrees(),
            sample.position.longitude().as_degrees(),
            sample
                .altitude_msl
                .map(|altitude| altitude.into_inner().as_meters()),
            meters_per_second(sample.vario),
            meters_per_second(sample.netto),
            meters_per_second(sample.relative_vario),
            terrain_elevation,
            meters_per_second(sample.wind.map(|wind| wind.east)),
            meters_per_second(sample.wind.map(|wind| wind.north)),
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::tests::{elevation_webp, write_terrain};
    use RecordingWrite::{DiscardRecording, RecordSample, StartRecording};
    use claims::{assert_err, assert_ok};

    /// Tile 7/64/45 covers this position.
    const COVERED: (f64, f64) = (46.0, 2.0);

    fn terrain(directory: &Path) -> Arc<Mutex<Terrain>> {
        std::fs::create_dir_all(directory.join("enroute/Europe")).unwrap();
        write_terrain(
            &directory.join("enroute/Europe/a.terrain"),
            &[(7, 64, 82, &elevation_webp(100))],
        );
        Arc::new(Mutex::new(assert_ok!(Terrain::load(directory))))
    }

    fn sample(utc: i64, (latitude, longitude): (f64, f64)) -> Sample {
        Sample {
            utc: UtcInstant::from_unix_milliseconds(utc),
            position: LatLon::from_degrees(latitude, longitude),
            altitude_msl: Some(MslAltitude::new(Length::from_meters(1200.0))),
            vario: Some(Speed::from_meters_per_second(1.5)),
            netto: Some(Speed::from_meters_per_second(2.0)),
            relative_vario: Some(Speed::from_meters_per_second(1.0)),
            wind: Some(Velocity {
                east: Speed::from_meters_per_second(-3.0),
                north: Speed::from_meters_per_second(4.0),
            }),
        }
    }

    /// Reads the committed rows through a separate connection.
    fn rows(path: &Path) -> Vec<Vec<rusqlite::types::Value>> {
        let connection = assert_ok!(Connection::open(path));
        let mut statement = assert_ok!(connection.prepare("SELECT * FROM samples ORDER BY rowid"));
        let columns = statement.column_count();
        let rows = assert_ok!(statement.query_map([], |row| {
            (0..columns).map(|index| row.get(index)).collect()
        }));
        assert_ok!(rows.collect())
    }

    #[test]
    fn migrates_a_new_database_in_a_new_directory_to_version_1() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("data/state.sqlite");
        assert_ok!(FlightRecording::open(&path, terrain(directory.path())));

        let connection = assert_ok!(Connection::open(&path));
        let version: u32 =
            assert_ok!(connection.pragma_query_value(None, "user_version", |row| row.get(0)));
        assert_eq!(version, 1);
        let journal_mode: String =
            assert_ok!(connection.pragma_query_value(None, "journal_mode", |row| row.get(0)));
        assert_eq!(journal_mode, "wal");
        let schema: Vec<String> = assert_ok!(
            assert_ok!(connection.prepare("SELECT sql FROM sqlite_schema"))
                .query_map([], |row| row.get(0))
                .and_then(Iterator::collect)
        );
        insta::assert_snapshot!(schema.join("\n"));
    }

    #[test]
    fn commits_each_write_with_the_terrain_elevation_of_the_sample() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.sqlite");
        let mut recording = assert_ok!(FlightRecording::open(&path, terrain(directory.path())));

        assert_ok!(recording.write(&StartRecording(sample(1_000, COVERED))));
        assert_eq!(rows(&path).len(), 1);
        let uncovered = (50.0, 20.0);
        assert_ok!(recording.write(&RecordSample(sample(2_000, uncovered))));
        insta::assert_debug_snapshot!(rows(&path));
    }

    #[test]
    fn start_recording_replaces_all_samples() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.sqlite");
        let mut recording = assert_ok!(FlightRecording::open(&path, terrain(directory.path())));
        for utc in [1_000, 2_000] {
            assert_ok!(recording.write(&RecordSample(sample(utc, COVERED))));
        }

        assert_ok!(recording.write(&StartRecording(sample(3_000, COVERED))));

        let utc: Vec<_> = rows(&path).into_iter().map(|row| row[0].clone()).collect();
        assert_eq!(utc, [rusqlite::types::Value::Integer(3_000)]);
    }

    #[test]
    fn failed_start_recording_keeps_the_previous_samples() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.sqlite");
        let mut recording = assert_ok!(FlightRecording::open(&path, terrain(directory.path())));
        assert_ok!(recording.write(&RecordSample(sample(1_000, COVERED))));

        // SQLite stores NaN as NULL, which the `NOT NULL` latitude rejects.
        let invalid = sample(2_000, (f64::NAN, 2.0));
        let error = assert_err!(recording.write(&StartRecording(invalid)));
        assert_eq!(
            error.to_string(),
            "NOT NULL constraint failed: samples.latitude_deg"
        );

        assert_eq!(rows(&path).len(), 1);
    }

    #[test]
    fn discard_recording_deletes_all_samples() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.sqlite");
        let mut recording = assert_ok!(FlightRecording::open(&path, terrain(directory.path())));
        for utc in [1_000, 2_000] {
            assert_ok!(recording.write(&RecordSample(sample(utc, COVERED))));
        }

        assert_ok!(recording.write(&DiscardRecording));

        assert_eq!(rows(&path), Vec::<Vec<rusqlite::types::Value>>::new());
    }

    #[test]
    fn load_reads_the_samples_in_recording_order() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.sqlite");
        let terrain = terrain(directory.path());
        let mut recording = assert_ok!(FlightRecording::open(&path, terrain.clone()));
        let uncovered = (50.0, 20.0);
        let mut partial = sample(2_000, uncovered);
        partial.altitude_msl = None;
        partial.wind = None;
        assert_ok!(recording.write(&StartRecording(sample(1_000, COVERED))));
        assert_ok!(recording.write(&RecordSample(partial)));
        drop(recording);

        let (samples, _) = load(path, terrain);

        assert_eq!(samples, [sample(1_000, COVERED), partial]);
    }

    #[test]
    #[tracing_test::traced_test]
    fn load_without_a_usable_database_restores_nothing() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("file");
        std::fs::write(&file, "").unwrap();

        let (samples, _) = load(file.join("state.sqlite"), terrain(directory.path()));

        assert_eq!(samples, []);
        assert!(logs_contain("ERROR"));
        assert!(logs_contain("Could not open the flight recording"));
    }
}
