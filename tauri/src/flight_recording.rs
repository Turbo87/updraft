use crate::terrain::Terrain;
use rusqlite::{Connection, OpenFlags, Transaction};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use updraft_core::{Sample, TrailSample, UtcInstant, Velocity};
use updraft_geo::LatLon;
use updraft_units::{Length, MslAltitude, Speed};

/// Each migration upgrades the database to the `user_version` that is its
/// position plus one.
const MIGRATIONS: &[&str] = &[
    "CREATE TABLE samples (
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
) STRICT",
    // Each existing sample uses its fix UTC as the system UTC.
    "CREATE TABLE samples_2 (
    utc_ms INTEGER NOT NULL,
    latitude_deg REAL NOT NULL,
    longitude_deg REAL NOT NULL,
    altitude_msl_m REAL,
    vario_mps REAL,
    netto_mps REAL,
    relative_vario_mps REAL,
    terrain_elevation_m REAL,
    wind_east_mps REAL,
    wind_north_mps REAL,
    system_utc_ms INTEGER NOT NULL
) STRICT;
INSERT INTO samples_2 SELECT *, utc_ms FROM samples ORDER BY rowid;
DROP TABLE samples;
ALTER TABLE samples_2 RENAME TO samples;",
];

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
    write_fails: bool,
}

impl FlightRecording {
    /// Replaces an unusable database with an empty one.
    pub fn open(path: &Path, terrain: Arc<Mutex<Terrain>>) -> anyhow::Result<Self> {
        if let Some(directory) = path.parent() {
            std::fs::create_dir_all(directory)?;
        }
        let (mut connection, current) = match open_usable(path) {
            Ok(opened) => opened,
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "Deleting the unusable flight recording");
                for suffix in ["", "-wal", "-shm"] {
                    let mut file = path.as_os_str().to_owned();
                    file.push(suffix);
                    if let Err(error) = std::fs::remove_file(file)
                        && error.kind() != std::io::ErrorKind::NotFound
                    {
                        return Err(error.into());
                    }
                }
                open_usable(path)?
            }
        };
        connection.pragma_update_and_check(None, "journal_mode", "WAL", |_| Ok(()))?;
        connection.pragma_update(None, "synchronous", "NORMAL")?;
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
            write_fails: false,
        })
    }

    /// Runs one transaction for the write. `system_utc` is the shell UTC at
    /// which the core emitted the write.
    pub fn write(
        &mut self,
        write: &RecordingWrite,
        system_utc: UtcInstant,
    ) -> rusqlite::Result<()> {
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
            insert(&transaction, sample, terrain_elevation, system_utc)?;
        }
        transaction.commit()
    }

    /// Writes the effect. Logs only the first failure of a series and the
    /// next success, because a source can give several samples each second.
    pub fn record(&mut self, write: RecordingWrite, system_utc: UtcInstant) {
        match (self.write(&write, system_utc), self.write_fails) {
            (Err(error), false) => {
                tracing::warn!(%error, "Could not write the flight recording");
                self.write_fails = true;
            }
            (Ok(()), true) => {
                tracing::info!("Resumed writing the flight recording");
                self.write_fails = false;
            }
            _ => {}
        }
    }

    fn stored_recording(&self) -> rusqlite::Result<StoredRecording> {
        let mut statement = self.connection.prepare(&format!(
            "SELECT {SAMPLE_COLUMNS}, system_utc_ms FROM samples ORDER BY rowid"
        ))?;
        let mut recording = StoredRecording::default();
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            recording.samples.push(sample(row)?);
            recording.stored_utc = Some(UtcInstant::from_unix_milliseconds(row.get(9)?));
        }
        Ok(recording)
    }
}

/// The flight recording that `load()` reads for the restore.
#[derive(Debug, Default, PartialEq)]
pub struct StoredRecording {
    /// The samples in recording order.
    pub samples: Vec<Sample>,
    /// The shell UTC at which the shell stored the last sample.
    pub stored_utc: Option<UtcInstant>,
}

/// The columns that `sample()` reads, in order.
const SAMPLE_COLUMNS: &str = "utc_ms, latitude_deg, longitude_deg, altitude_msl_m, vario_mps,
    netto_mps, relative_vario_mps, wind_east_mps, wind_north_mps";

fn sample(row: &rusqlite::Row<'_>) -> rusqlite::Result<Sample> {
    let speed = |value: Option<f64>| value.map(Speed::from_meters_per_second);
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
}

/// Opens the flight recording and reads it for the restore.
///
/// The returned writer stores the recording effects in order on a blocking
/// worker. When the database cannot be opened, the restore is empty and the
/// writer drops all writes.
pub fn load(
    path: PathBuf,
    terrain: Arc<Mutex<Terrain>>,
) -> (StoredRecording, impl Fn(RecordingWrite) + Send + 'static) {
    let (recording, stored) = match FlightRecording::open(&path, terrain) {
        Ok(recording) => {
            let stored = recording.stored_recording().unwrap_or_else(|error| {
                tracing::error!(path = %path.display(), %error, "Could not read the flight recording");
                StoredRecording::default()
            });
            (Some(recording), stored)
        }
        Err(error) => {
            tracing::error!(path = %path.display(), %error, "Could not open the flight recording");
            (None, StoredRecording::default())
        }
    };
    let (sender, receiver) = std::sync::mpsc::channel();

    tauri::async_runtime::spawn_blocking(move || {
        let Some(mut recording) = recording else {
            return;
        };
        for (write, system_utc) in receiver {
            recording.record(write, system_utc);
        }
    });

    let writer = move |write| {
        let system_utc = UtcInstant::from_offset_date_time(time::OffsetDateTime::now_utc());
        let _ = sender.send((write, system_utc));
    };
    (stored, writer)
}

/// The path of `state.sqlite`.
pub struct FlightRecordingPath(pub PathBuf);

/// Reads the samples with a UTC at or after `since` in Unix milliseconds.
///
/// The read uses a separate read-only connection, so it does not wait for
/// the writer.
pub fn trail_samples(path: &Path, since: i64) -> rusqlite::Result<Vec<TrailSample>> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut statement = connection.prepare(&format!(
        "SELECT {SAMPLE_COLUMNS}, terrain_elevation_m FROM samples
        WHERE utc_ms >= ?1 ORDER BY rowid"
    ))?;
    statement
        .query_map([since], |row| {
            Ok(TrailSample::new(&sample(row)?, row.get(9)?))
        })?
        .collect()
}

/// Opens the database and returns its `user_version`. Fails when the
/// database does not pass `PRAGMA quick_check` or is newer than the migrations.
fn open_usable(path: &Path) -> anyhow::Result<(Connection, i64)> {
    let connection = Connection::open(path)?;
    let check: Vec<String> = connection
        .prepare("PRAGMA quick_check")?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    anyhow::ensure!(check == ["ok"], "quick check failed: {}", check.join(" "));
    let current: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let supported = MIGRATIONS.len() as i64;
    anyhow::ensure!(
        current <= supported,
        "user version {current} is newer than {supported}"
    );
    Ok((connection, current))
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
    system_utc: UtcInstant,
) -> rusqlite::Result<()> {
    let meters_per_second = |speed: Option<Speed>| speed.map(Speed::as_meters_per_second);
    transaction.execute(
        "INSERT INTO samples VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
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
            system_utc.unix_milliseconds(),
        ],
    )?;
    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::terrain::tests::{elevation_webp, write_terrain};
    use RecordingWrite::{DiscardRecording, RecordSample, StartRecording};
    use claims::{assert_err, assert_ok};

    pub const SYSTEM_UTC: UtcInstant = UtcInstant::from_unix_milliseconds(9_000);

    /// Tile 7/64/45 covers this position.
    pub const COVERED: (f64, f64) = (46.0, 2.0);

    pub fn terrain(directory: &Path) -> Arc<Mutex<Terrain>> {
        std::fs::create_dir_all(directory.join("enroute/Europe")).unwrap();
        write_terrain(
            &directory.join("enroute/Europe/a.terrain"),
            &[(7, 64, 82, &elevation_webp(100))],
        );
        Arc::new(Mutex::new(assert_ok!(Terrain::load(directory))))
    }

    pub fn sample(utc: i64, (latitude, longitude): (f64, f64)) -> Sample {
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
    fn migrates_a_new_database_in_a_new_directory_to_version_2() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("data/state.sqlite");
        assert_ok!(FlightRecording::open(&path, terrain(directory.path())));

        let connection = assert_ok!(Connection::open(&path));
        let version: u32 =
            assert_ok!(connection.pragma_query_value(None, "user_version", |row| row.get(0)));
        assert_eq!(version, 2);
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
    fn migration_2_keeps_the_samples_and_uses_their_utc_as_the_system_utc() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.sqlite");
        let connection = assert_ok!(Connection::open(&path));
        assert_ok!(connection.execute_batch(MIGRATIONS[0]));
        assert_ok!(connection.execute_batch(
            "INSERT INTO samples VALUES (2000, 46.0, 2.0, NULL, NULL, NULL, NULL, NULL, NULL, NULL);
            INSERT INTO samples VALUES (1000, 50.0, 20.0, 1200.0, 1.5, 2.0, 1.0, 100.0, -3.0, 4.0);
            PRAGMA user_version = 1;"
        ));
        drop(connection);

        assert_ok!(FlightRecording::open(&path, terrain(directory.path())));

        assert_eq!(user_version(&path), 2);
        insta::assert_debug_snapshot!(rows(&path));
    }

    #[test]
    fn commits_each_write_with_the_terrain_elevation_of_the_sample() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.sqlite");
        let mut recording = assert_ok!(FlightRecording::open(&path, terrain(directory.path())));

        assert_ok!(recording.write(&StartRecording(sample(1_000, COVERED)), SYSTEM_UTC));
        assert_eq!(rows(&path).len(), 1);
        let uncovered = (50.0, 20.0);
        assert_ok!(recording.write(&RecordSample(sample(2_000, uncovered)), SYSTEM_UTC));
        insta::assert_debug_snapshot!(rows(&path));
    }

    #[test]
    fn start_recording_replaces_all_samples() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.sqlite");
        let mut recording = assert_ok!(FlightRecording::open(&path, terrain(directory.path())));
        for utc in [1_000, 2_000] {
            assert_ok!(recording.write(&RecordSample(sample(utc, COVERED)), SYSTEM_UTC));
        }

        assert_ok!(recording.write(&StartRecording(sample(3_000, COVERED)), SYSTEM_UTC));

        let utc: Vec<_> = rows(&path).into_iter().map(|row| row[0].clone()).collect();
        assert_eq!(utc, [rusqlite::types::Value::Integer(3_000)]);
    }

    #[test]
    fn failed_start_recording_keeps_the_previous_samples() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.sqlite");
        let mut recording = assert_ok!(FlightRecording::open(&path, terrain(directory.path())));
        assert_ok!(recording.write(&RecordSample(sample(1_000, COVERED)), SYSTEM_UTC));

        // SQLite stores NaN as NULL, which the `NOT NULL` latitude rejects.
        let invalid = sample(2_000, (f64::NAN, 2.0));
        let error = assert_err!(recording.write(&StartRecording(invalid), SYSTEM_UTC));
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
            assert_ok!(recording.write(&RecordSample(sample(utc, COVERED)), SYSTEM_UTC));
        }

        assert_ok!(recording.write(&DiscardRecording, SYSTEM_UTC));

        assert_eq!(rows(&path), Vec::<Vec<rusqlite::types::Value>>::new());
    }

    #[test]
    fn load_reads_the_samples_in_recording_order_and_the_last_stored_utc() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.sqlite");
        let terrain = terrain(directory.path());
        let mut recording = assert_ok!(FlightRecording::open(&path, terrain.clone()));
        let uncovered = (50.0, 20.0);
        let mut partial = sample(2_000, uncovered);
        partial.altitude_msl = None;
        partial.wind = None;
        assert_ok!(recording.write(&StartRecording(sample(1_000, COVERED)), SYSTEM_UTC));
        let stored_utc = UtcInstant::from_unix_milliseconds(9_500);
        assert_ok!(recording.write(&RecordSample(partial), stored_utc));
        drop(recording);

        let (stored, _) = load(path, terrain);

        assert_eq!(
            stored,
            StoredRecording {
                samples: vec![sample(1_000, COVERED), partial],
                stored_utc: Some(stored_utc),
            }
        );
    }

    #[test]
    #[tracing_test::traced_test]
    fn load_without_a_usable_database_restores_nothing() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("file");
        std::fs::write(&file, "").unwrap();

        let (stored, _) = load(file.join("state.sqlite"), terrain(directory.path()));

        assert_eq!(stored, StoredRecording::default());
        assert!(logs_contain("ERROR"));
        assert!(logs_contain("Could not open the flight recording"));
    }

    /// Accepts only the expected log lines, each with its level and message.
    fn only_logs(lines: &[&str], expected: &[(&str, &str)]) -> Result<(), String> {
        let matches = lines.len() == expected.len()
            && lines.iter().zip(expected).all(|(line, (level, message))| {
                line.contains(&format!(" {level} ")) && line.contains(message)
            });
        if matches {
            Ok(())
        } else {
            Err(format!("expected {expected:?}, got {lines:#?}"))
        }
    }

    fn user_version(path: &Path) -> i64 {
        let connection = assert_ok!(Connection::open(path));
        assert_ok!(connection.pragma_query_value(None, "user_version", |row| row.get(0)))
    }

    #[test]
    #[tracing_test::traced_test]
    fn load_replaces_a_corrupt_database_with_an_empty_one() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.sqlite");
        let terrain = terrain(directory.path());
        let mut recording = assert_ok!(FlightRecording::open(&path, terrain.clone()));
        assert_ok!(recording.write(&StartRecording(sample(1_000, COVERED)), SYSTEM_UTC));
        drop(recording);
        // Page 2 is the root page of the `samples` table.
        let mut bytes = std::fs::read(&path).unwrap();
        bytes[4096..8192].fill(0xff);
        std::fs::write(&path, bytes).unwrap();

        let (stored, _) = load(path.clone(), terrain);

        assert_eq!(stored, StoredRecording::default());
        assert_eq!(rows(&path), Vec::<Vec<rusqlite::types::Value>>::new());
        assert_eq!(user_version(&path), 2);
        logs_assert(|lines| {
            only_logs(lines, &[("WARN", "Deleting the unusable flight recording")])
        });
    }

    #[test]
    #[tracing_test::traced_test]
    fn load_replaces_a_database_with_a_newer_version_with_an_empty_one() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.sqlite");
        let terrain = terrain(directory.path());
        let mut recording = assert_ok!(FlightRecording::open(&path, terrain.clone()));
        assert_ok!(recording.write(&StartRecording(sample(1_000, COVERED)), SYSTEM_UTC));
        drop(recording);
        let connection = assert_ok!(Connection::open(&path));
        assert_ok!(connection.pragma_update(None, "user_version", 3));
        drop(connection);

        let (stored, _) = load(path.clone(), terrain);

        assert_eq!(stored, StoredRecording::default());
        assert_eq!(rows(&path), Vec::<Vec<rusqlite::types::Value>>::new());
        assert_eq!(user_version(&path), 2);
        logs_assert(|lines| {
            only_logs(lines, &[("WARN", "Deleting the unusable flight recording")])
        });
    }

    #[test]
    #[tracing_test::traced_test]
    fn writes_continue_after_a_write_failure() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.sqlite");
        let mut recording = assert_ok!(FlightRecording::open(&path, terrain(directory.path())));
        recording.record(StartRecording(sample(1_000, COVERED)), SYSTEM_UTC);

        // SQLite stores NaN as NULL, which the `NOT NULL` latitude rejects.
        recording.record(RecordSample(sample(2_000, (f64::NAN, 2.0))), SYSTEM_UTC);
        recording.record(RecordSample(sample(3_000, (f64::NAN, 2.0))), SYSTEM_UTC);
        recording.record(RecordSample(sample(4_000, COVERED)), SYSTEM_UTC);
        recording.record(RecordSample(sample(5_000, COVERED)), SYSTEM_UTC);

        let utc: Vec<_> = rows(&path).into_iter().map(|row| row[0].clone()).collect();
        assert_eq!(
            utc,
            [1_000, 4_000, 5_000].map(rusqlite::types::Value::Integer)
        );
        logs_assert(|lines| {
            only_logs(
                lines,
                &[
                    ("WARN", "Could not write the flight recording"),
                    ("INFO", "Resumed writing the flight recording"),
                ],
            )
        });
    }
}
