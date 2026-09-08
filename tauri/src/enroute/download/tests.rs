use super::*;
use crate::enroute::{BasemapEntry, parse_catalog, storage::installed_files};
use claims::{assert_err, assert_le, assert_ok};
use std::fs::{self, FileTimes};
use std::io::Write;
use std::time::{Duration, SystemTime};

fn entry() -> BasemapEntry {
    let json = br#"{"maps":[{"path":"Europe/Germany.mbtiles","size":10,"time":"20260908"}]}"#;
    assert_ok!(parse_catalog(json)).remove(0)
}

#[test]
fn unfinished_downloads_stay_out_of_inventory_and_preserve_installed_files() {
    let directory = tempfile::tempdir().unwrap();
    let entry = entry();
    let parent = directory.path().join("enroute/Europe");
    assert_ok!(fs::create_dir_all(&parent));
    let destination = parent.join("Germany.mbtiles");
    assert_ok!(fs::write(&destination, b"installed"));
    let modified = assert_ok!(assert_ok!(fs::metadata(&destination)).modified());
    let mut download = assert_ok!(BasemapDownload::new(directory.path(), &entry));
    assert_ok!(download.file_mut().write_all(b"partial"));
    assert_eq!(assert_ok!(installed_files(directory.path())).len(), 1);
    assert_eq!(assert_ok!(fs::read(&destination)), b"installed");
    drop(download);
    assert_eq!(assert_ok!(fs::read(&destination)), b"installed");
    assert_eq!(
        assert_ok!(assert_ok!(fs::metadata(&destination)).modified()),
        modified
    );
    assert_eq!(assert_ok!(fs::read_dir(parent)).count(), 1);
}

#[test]
fn installation_replaces_bytes_without_validation_or_changing_activation() {
    let entry = entry();
    for (installed, disabled) in [(false, false), (true, false), (true, true)] {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("enroute/Europe/Germany.mbtiles");
        let mut download = assert_ok!(BasemapDownload::new(directory.path(), &entry));
        let marker = destination.with_extension("mbtiles.disabled");
        if installed {
            assert_ok!(fs::write(&destination, b"old"));
        }
        if disabled {
            assert_ok!(fs::write(&marker, b""));
        }
        assert_ok!(download.file_mut().write_all(b"not a database"));
        let times = FileTimes::new().set_modified(SystemTime::UNIX_EPOCH);
        assert_ok!(download.file_mut().set_times(times));
        if installed {
            assert_eq!(assert_ok!(fs::read(&destination)), b"old");
        } else {
            assert!(assert_ok!(installed_files(directory.path())).is_empty());
        }
        let before = SystemTime::now();
        assert_ok!(download.install());
        let modified = assert_ok!(assert_ok!(fs::metadata(&destination)).modified());
        let before = assert_ok!(before.duration_since(SystemTime::UNIX_EPOCH)).as_secs();
        let installed_at = assert_ok!(modified.duration_since(SystemTime::UNIX_EPOCH)).as_secs();
        assert_le!(before, installed_at);
        assert_le!(modified, SystemTime::now());
        assert_eq!(assert_ok!(fs::read(&destination)), b"not a database");
        assert_eq!(marker.exists(), disabled);
    }
}

#[test]
fn installation_failure_discards_the_temporary_file() {
    let directory = tempfile::tempdir().unwrap();
    let mut download = assert_ok!(BasemapDownload::new(directory.path(), &entry()));
    assert_ok!(download.file_mut().write_all(b"complete"));
    let parent = directory.path().join("enroute/Europe");
    let destination = parent.join("Germany.mbtiles");
    assert_ok!(fs::create_dir(&destination));
    assert_err!(download.install());
    assert!(destination.is_dir());
    assert_eq!(assert_ok!(fs::read_dir(parent)).count(), 1);
}

#[test]
fn update_detection_compares_publication_midnight_with_local_modification_time() {
    let entry = entry();
    let publication: SystemTime = time::macros::datetime!(2026-09-08 0:00 UTC).into();
    assert!(entry.update_available(publication - Duration::from_nanos(1)));
    assert!(!entry.update_available(publication));
    assert!(!entry.update_available(publication + Duration::from_nanos(1)));
}
