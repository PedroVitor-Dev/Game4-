use super::*;
#[test]
fn defaults_are_private_and_do_not_create_or_move_files() {
    let d = tempfile::tempdir().unwrap();
    let storage = StorageSettings::new(d.path().to_path_buf());
    let info = storage.get().unwrap();
    assert!(info.downloads.is_none() && info.exports.is_none() && info.backups.is_none());
    assert_eq!(info.app_data_directory, d.path().to_string_lossy());
    assert!(
        storage
            .initial_directory(StorageDirectoryKind::Downloads)
            .unwrap()
            .is_none()
    );
    assert_eq!(fs::read_dir(d.path()).unwrap().count(), 0);
}
#[test]
fn choices_persist_independently_and_reset_keeps_all_content() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("app");
    fs::create_dir(&root).unwrap();
    let chosen = d.path().join("Games β");
    fs::create_dir(&chosen).unwrap();
    fs::write(chosen.join("keep.bin"), b"original").unwrap();
    let settings = StorageSettings::new(root.clone());
    settings
        .change(StorageDirectoryKind::Downloads, Some(chosen.clone()))
        .unwrap();
    settings
        .change(StorageDirectoryKind::Exports, Some(chosen.clone()))
        .unwrap();
    let restarted = StorageSettings::new(root);
    let info = restarted.get().unwrap();
    assert!(info.downloads.unwrap().available);
    assert!(info.exports.is_some() && info.backups.is_none());
    assert_eq!(
        restarted
            .initial_directory(StorageDirectoryKind::Downloads)
            .unwrap(),
        Some(chosen.clone())
    );
    let reset = restarted
        .change(StorageDirectoryKind::Downloads, None)
        .unwrap();
    assert!(reset.downloads.is_none() && reset.exports.is_some());
    assert_eq!(fs::read(chosen.join("keep.bin")).unwrap(), b"original");
    assert_eq!(fs::read_dir(chosen).unwrap().count(), 1);
}
#[test]
fn missing_or_invalid_directories_do_not_replace_valid_preferences() {
    let d = tempfile::tempdir().unwrap();
    let storage = StorageSettings::new(d.path().to_path_buf());
    let chosen = d.path().join("games");
    fs::create_dir(&chosen).unwrap();
    storage
        .change(StorageDirectoryKind::Downloads, Some(chosen.clone()))
        .unwrap();
    let before = fs::read(storage.path()).unwrap();
    for path in [
        PathBuf::from("relative"),
        d.path().join("missing"),
        storage.path(),
    ] {
        assert!(
            storage
                .change(StorageDirectoryKind::Downloads, Some(path))
                .is_err()
        );
        assert_eq!(fs::read(storage.path()).unwrap(), before);
    }
    fs::rename(&chosen, d.path().join("moved")).unwrap();
    assert!(!storage.get().unwrap().downloads.unwrap().available);
    assert!(
        storage
            .initial_directory(StorageDirectoryKind::Downloads)
            .is_err()
    );
    assert!(!chosen.exists());
    storage
        .change(StorageDirectoryKind::Downloads, None)
        .unwrap();
    assert!(
        storage
            .initial_directory(StorageDirectoryKind::Downloads)
            .unwrap()
            .is_none()
    );
}
#[test]
fn malformed_future_duplicate_and_oversized_settings_are_preserved() {
    let d = tempfile::tempdir().unwrap();
    let storage = StorageSettings::new(d.path().to_path_buf());
    fs::create_dir_all(storage.path().parent().unwrap()).unwrap();
    for raw in [
        br#"{"version":99,"downloads":null,"exports":null,"backups":null}"#.to_vec(),
        br#"{"version":1,"version":1,"downloads":null,"exports":null,"backups":null}"#.to_vec(),
        b"not-json".to_vec(),
        vec![b' '; MAX_SETTINGS as usize + 1],
    ] {
        fs::write(storage.path(), &raw).unwrap();
        assert!(storage.get().is_err());
        assert!(
            storage
                .change(StorageDirectoryKind::Downloads, None)
                .is_err()
        );
        assert_eq!(fs::read(storage.path()).unwrap(), raw);
    }
}
#[test]
fn storage_write_failure_preserves_configuration() {
    let d = tempfile::tempdir().unwrap();
    let storage = StorageSettings::new(d.path().to_path_buf());
    fs::write(d.path().join("settings"), b"keep").unwrap();
    assert!(
        storage
            .change(
                StorageDirectoryKind::Downloads,
                Some(d.path().to_path_buf())
            )
            .is_err()
    );
    assert_eq!(fs::read(d.path().join("settings")).unwrap(), b"keep");
}
#[cfg(windows)]
#[test]
fn junction_targets_are_rejected_without_touching_external_files() {
    let d = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("keep.bin"), b"external").unwrap();
    let link = d.path().join("junction");
    let status = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&link)
        .arg(outside.path())
        .output()
        .unwrap();
    assert!(status.status.success());
    let storage = StorageSettings::new(d.path().to_path_buf());
    assert!(
        storage
            .change(StorageDirectoryKind::Downloads, Some(link.clone()))
            .is_err()
    );
    assert!(!storage.path().exists());
    assert_eq!(
        fs::read(outside.path().join("keep.bin")).unwrap(),
        b"external"
    );
    fs::remove_dir(link).unwrap();
}

#[cfg(windows)]
#[test]
fn locked_preferences_keep_the_previous_atomic_snapshot() {
    let d = tempfile::tempdir().unwrap();
    let storage = StorageSettings::new(d.path().to_path_buf());
    storage
        .change(
            StorageDirectoryKind::Downloads,
            Some(d.path().to_path_buf()),
        )
        .unwrap();
    let before = fs::read(storage.path()).unwrap();
    let locked = library::open_source(&storage.path()).unwrap();
    assert!(
        storage
            .change(StorageDirectoryKind::Downloads, None)
            .is_err()
    );
    assert_eq!(fs::read(storage.path()).unwrap(), before);
    assert!(storage.get().unwrap().downloads.is_some());
    drop(locked);
    assert!(
        storage
            .change(StorageDirectoryKind::Downloads, None)
            .unwrap()
            .downloads
            .is_none()
    );
}
