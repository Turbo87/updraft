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
    assert_ok!(remove_partial_downloads(directory.path()));
    let provider = directory.path().join("enroute");
    assert_ok!(fs::write(provider, b"not a directory"));
    assert_err!(installed_files(directory.path()));
    assert_err!(remove_partial_downloads(directory.path()));
}

#[test]
fn cleanup_removes_only_partial_downloads_and_can_repeat() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let partials = [
        "enroute/Europe/.updraft-download-one.part",
        "enroute/North America/United States/.updraft-download-two.part",
    ];
    let retained = [
        "enroute/Europe/Germany.mbtiles",
        "enroute/Europe/Germany.terrain",
        "enroute/Europe/Germany.mbtiles.disabled",
        "enroute/Europe/Germany.terrain.disabled",
        "enroute/Europe/other.part",
        "enroute/Europe/.updraft-download-one.part.backup",
        "other/Europe/.updraft-download-one.part",
        "enroute/Germany.mbtiles",
    ];
    for name in partials.into_iter().chain(retained) {
        let path = root.join(name);
        assert_ok!(fs::create_dir_all(path.parent().unwrap()));
        assert_ok!(fs::write(path, b"unchanged"));
    }
    let installed = assert_ok!(installed_files(root));
    for _ in 0..2 {
        assert_ok!(remove_partial_downloads(root));
        for name in partials {
            assert!(!root.join(name).exists());
        }
        for name in retained {
            assert_eq!(assert_ok!(fs::read(root.join(name))), b"unchanged");
        }
        assert_eq!(assert_ok!(installed_files(root)), installed);
    }
}

#[cfg(unix)]
#[test]
fn cleanup_does_not_follow_symlinks() {
    use std::os::unix::fs::symlink;
    let directory = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let parent = directory.path().join("enroute/Europe");
    assert_ok!(fs::create_dir_all(&parent));
    let target = outside.path().join(".updraft-download-outside.part");
    assert_ok!(fs::write(&target, b"untouched"));
    let link = parent.join(".updraft-download-link.part");
    assert_ok!(symlink(&target, &link));
    assert_ok!(symlink(outside.path(), parent.join("linked")));
    assert_ok!(remove_partial_downloads(directory.path()));
    assert_eq!(assert_ok!(fs::read(&target)), b"untouched");
    assert!(assert_ok!(fs::symlink_metadata(&link)).is_symlink());
    assert_ok!(fs::remove_dir_all(directory.path().join("enroute")));
    assert_ok!(symlink(outside.path(), directory.path().join("enroute")));
    assert_err!(remove_partial_downloads(directory.path()));
    assert_eq!(assert_ok!(fs::read(target)), b"untouched");
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
