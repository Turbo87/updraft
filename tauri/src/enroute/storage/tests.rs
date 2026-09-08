use super::*;
use claims::{assert_err, assert_ok};
use std::fs;

#[test]
fn discovers_managed_paths_without_adopting_development_files() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    for name in [
        "enroute/Europe/Germany.mbtiles",
        "enroute/Europe/Germany.terrain",
        "enroute/North America/United States/Georgia.mbtiles",
        "enroute/Asia/Georgia.mbtiles",
        "enroute/Germany.mbtiles",
        "enroute/Germany.terrain",
        "other/Europe/Germany.mbtiles",
        "enroute/Europe/Germany.mbtiles.disabled",
        "enroute/Europe/download.tmp",
    ] {
        let path = root.join(name);
        assert_ok!(fs::create_dir_all(path.parent().unwrap()));
        assert_ok!(fs::write(path, b"unopened"));
    }
    let files = assert_ok!(installed_files(root));
    assert_eq!(
        files.keys().map(String::as_str).collect::<Vec<_>>(),
        [
            "enroute/Asia/Georgia.mbtiles",
            "enroute/Europe/Germany.mbtiles",
            "enroute/Europe/Germany.terrain",
            "enroute/North America/United States/Georgia.mbtiles",
        ]
    );
    for (id, path) in &files {
        assert_eq!(*path, root.join(id));
        assert_eq!(assert_ok!(fs::read(path)), b"unopened");
    }
    assert_eq!(assert_ok!(installed_files(root)), files);
    assert!(root.join("enroute/Germany.mbtiles").exists());
}

#[test]
fn missing_storage_is_empty_but_invalid_storage_fails() {
    let directory = tempfile::tempdir().unwrap();
    assert!(assert_ok!(installed_files(directory.path())).is_empty());
    let provider = directory.path().join("enroute");
    assert_ok!(fs::write(provider, b"not a directory"));
    assert_err!(installed_files(directory.path()));
}

#[cfg(unix)]
#[test]
fn does_not_follow_file_directory_or_provider_symlinks() {
    use std::os::unix::fs::symlink;
    let directory = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let europe = directory.path().join("enroute/Europe");
    assert_ok!(fs::create_dir_all(&europe));
    let target = outside.path().join("Germany.mbtiles");
    assert_ok!(fs::write(&target, b"unopened"));
    assert_ok!(symlink(&target, europe.join("Germany.mbtiles")));
    assert_ok!(symlink(outside.path(), europe.join("linked")));
    assert!(assert_ok!(installed_files(directory.path())).is_empty());
    assert_ok!(fs::remove_dir_all(directory.path().join("enroute")));
    assert_ok!(symlink(outside.path(), directory.path().join("enroute")));
    assert_err!(installed_files(directory.path()));
}
