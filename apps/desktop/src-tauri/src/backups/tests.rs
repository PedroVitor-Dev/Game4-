use super::*;
fn setup() -> (tempfile::TempDir, Database, ImportState) {
    let d = tempfile::tempdir().unwrap();
    let db = Database::open(&d.path().join("database/game4.sqlite3")).unwrap();
    (d, db, ImportState::default())
}
fn import(db: &Database, path: &Path) {
    let request = StartImportRequest {
        request_id: Uuid::new_v4().to_string(),
        selection_token: Uuid::new_v4().to_string(),
    };
    let task = crate::library::create_task(db, &request).unwrap();
    crate::library::run_import(db, &task.task_id, vec![path.to_path_buf()]).unwrap();
}
fn source(d: &Path, name: &str) -> std::path::PathBuf {
    let p = d.join(name);
    std::fs::write(&p, b"abc").unwrap();
    p
}
fn fingerprint(db: &Database) -> Vec<u8> {
    Data::read(&db.connection.lock().unwrap())
        .unwrap()
        .fingerprint()
        .unwrap()
}

#[test]
fn full_backup_restores_metadata_hashes_paths_history_and_portable_identity() {
    let (d, db, imports) = setup();
    let original = source(d.path(), "original.bin");
    import(&db, &original);
    db.connection.lock().unwrap().execute("UPDATE collection_items SET platform='Test platform',notes='Private note',declared_source='Synthetic bytes' WHERE id=1",[]).unwrap();
    let identity = crate::manifests::from_catalog(&db, "1")
        .unwrap()
        .collection
        .id;
    let backup_path = d.path().join("library.json");
    save(&db, &imports, &backup_path).unwrap();
    let backup = read_backup(&backup_path).unwrap();
    assert_eq!(backup.data.file_locations.len(), 1);
    assert_eq!(backup.data.tasks.len(), 1);
    import(&db, &source(d.path(), "second.bin"));
    db.connection
        .lock()
        .unwrap()
        .execute(
            "UPDATE collection_items SET notes='Changed later' WHERE id=1",
            [],
        )
        .unwrap();
    let before = fingerprint(&db);
    let state = BackupState::default();
    let preview = state.prepare(&db, &imports, &backup_path).unwrap();
    assert_eq!(preview.item_count, 1);
    assert_eq!(preview.file_count, 1);
    assert_eq!(preview.location_count, 1);
    let result = state
        .restore(&db, &imports, &preview.restore_token)
        .unwrap();
    assert_eq!(result.item_count, 1);
    assert_eq!(db.info().unwrap().item_count, 1);
    assert_eq!(db.info().unwrap().task_count, 1);
    let restored = Data::read(&db.connection.lock().unwrap()).unwrap();
    assert_eq!(
        restored.collection_items[0].notes.as_deref(),
        Some("Private note")
    );
    assert_eq!(
        restored.collection_items[0].platform.as_deref(),
        Some("Test platform")
    );
    assert_eq!(
        restored.files[0].sha256,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(restored.file_locations[0].verification_status, "unverified");
    assert!(restored.file_locations[0].last_verified_at.is_none());
    assert_eq!(restored.collection_manifest_ids[0].manifest_id, identity);
    let recovery = db
        .path
        .parent()
        .unwrap()
        .join("backups")
        .join(&result.recovery_file_name);
    assert_eq!(
        read_backup(&recovery).unwrap().data.fingerprint().unwrap(),
        before
    );
    assert_eq!(std::fs::read(&original).unwrap(), b"abc");
    assert!(matches!(
        state
            .restore(&db, &imports, &preview.restore_token)
            .unwrap_err()
            .code,
        ErrorCode::RestoreStale
    ));
    let path = db.path.clone();
    drop(db);
    let reopened = Database::open(&path).unwrap();
    assert_eq!(reopened.info().unwrap().item_count, 1);
    assert_eq!(
        crate::manifests::from_catalog(&reopened, "1")
            .unwrap()
            .collection
            .id,
        identity
    );
}
#[test]
fn malformed_incompatible_or_inconsistent_backups_never_change_live_data() {
    let (d, db, imports) = setup();
    import(&db, &source(d.path(), "one.bin"));
    let original = fingerprint(&db);
    let snapshot = Backup::snapshot(&db.connection.lock().unwrap()).unwrap();
    let mut incompatible = snapshot.clone();
    incompatible.backup_version = 99;
    assert!(matches!(
        Backup::parse(&incompatible.bytes().unwrap())
            .unwrap_err()
            .code,
        ErrorCode::BackupVersionUnsupported
    ));
    incompatible = snapshot.clone();
    incompatible.database_schema_version = 99;
    assert!(matches!(
        Backup::parse(&incompatible.bytes().unwrap())
            .unwrap_err()
            .code,
        ErrorCode::BackupVersionUnsupported
    ));
    let mut dangling = snapshot.clone();
    dangling.data.file_locations[0].file_id = Decimal(999);
    assert!(Backup::parse(&dangling.bytes().unwrap()).is_err());
    let mut duplicate = snapshot.clone();
    duplicate.data.files.push(duplicate.data.files[0].clone());
    assert!(Backup::parse(&duplicate.bytes().unwrap()).is_err());
    let mut bad_task = snapshot.clone();
    bad_task.data.tasks[0].updated_at = "invalid".into();
    assert!(Backup::parse(&bad_task.bytes().unwrap()).is_err());
    bad_task.data.tasks[0].updated_at = "2026-99-99T99:99:99.999Z".into();
    assert!(Backup::parse(&bad_task.bytes().unwrap()).is_err());
    let mut invalid_uuid = snapshot.clone();
    invalid_uuid.data.collection_manifest_ids.push(Identity {
        item_id: Decimal(1),
        manifest_id: "bad".into(),
    });
    assert!(Backup::parse(&invalid_uuid.bytes().unwrap()).is_err());
    let mut unknown = serde_json::to_value(&snapshot).unwrap();
    unknown["executeSql"] = serde_json::json!("DELETE FROM files");
    assert!(Backup::parse(&serde_json::to_vec(&unknown).unwrap()).is_err());
    assert!(Backup::parse(b"{}").is_err());
    assert!(Backup::parse(&vec![b' '; MAX_BYTES + 1]).is_err());
    let bad = d.path().join("bad.json");
    std::fs::write(&bad, b"{}").unwrap();
    assert!(BackupState::default().prepare(&db, &imports, &bad).is_err());
    assert_eq!(fingerprint(&db), original);
}
#[test]
fn stale_expired_and_busy_restore_requests_are_rejected() {
    let (d, db, imports) = setup();
    import(&db, &source(d.path(), "one.bin"));
    let backup = d.path().join("backup.json");
    save(&db, &imports, &backup).unwrap();
    let state = BackupState::default();
    let preview = state.prepare(&db, &imports, &backup).unwrap();
    db.connection
        .lock()
        .unwrap()
        .execute("UPDATE collection_items SET notes='changed' WHERE id=1", [])
        .unwrap();
    let changed = fingerprint(&db);
    assert!(matches!(
        state
            .restore(&db, &imports, &preview.restore_token)
            .unwrap_err()
            .code,
        ErrorCode::RestoreStale
    ));
    assert_eq!(fingerprint(&db), changed);
    let p = state.prepare(&db, &imports, &backup).unwrap();
    state.prepared.lock().unwrap().as_mut().unwrap().created =
        Instant::now() - Duration::from_secs(301);
    assert!(matches!(
        state
            .restore(&db, &imports, &p.restore_token)
            .unwrap_err()
            .code,
        ErrorCode::RestoreStale
    ));
    {
        let _worker = imports.worker.lock().unwrap();
        assert!(matches!(
            save(&db, &imports, &d.path().join("busy.json"))
                .unwrap_err()
                .code,
            ErrorCode::LibraryBusy
        ));
    }
    crate::library::create_task(
        &db,
        &StartImportRequest {
            request_id: Uuid::new_v4().to_string(),
            selection_token: Uuid::new_v4().to_string(),
        },
    )
    .unwrap();
    assert!(matches!(
        state.prepare(&db, &imports, &backup).unwrap_err().code,
        ErrorCode::LibraryBusy
    ));
    assert_eq!(db.info().unwrap().item_count, 1);
}
#[test]
fn write_failure_or_restore_sql_failure_preserves_the_entire_previous_catalog() {
    let (d, db, imports) = setup();
    import(&db, &source(d.path(), "one.bin"));
    let backup = d.path().join("backup.json");
    save(&db, &imports, &backup).unwrap();
    let original_bytes = std::fs::read(&backup).unwrap();
    assert!(save(&db, &imports, &backup).is_err());
    assert_eq!(std::fs::read(&backup).unwrap(), original_bytes);
    import(&db, &source(d.path(), "two.bin"));
    let before = fingerprint(&db);
    let state = BackupState::default();
    let p = state.prepare(&db, &imports, &backup).unwrap();
    let recovery_directory = db.path.parent().unwrap().join("backups");
    std::fs::write(&recovery_directory, b"block directory creation").unwrap();
    assert!(matches!(
        state
            .restore(&db, &imports, &p.restore_token)
            .unwrap_err()
            .code,
        ErrorCode::BackupWriteFailed
    ));
    assert_eq!(fingerprint(&db), before);
    std::fs::remove_file(&recovery_directory).unwrap();
    let p = state.prepare(&db, &imports, &backup).unwrap();
    db.connection.lock().unwrap().execute_batch("CREATE TRIGGER reject_restore BEFORE INSERT ON files BEGIN SELECT RAISE(ABORT,'injected restore failure'); END;").unwrap();
    assert!(matches!(
        state
            .restore(&db, &imports, &p.restore_token)
            .unwrap_err()
            .code,
        ErrorCode::RestoreFailed
    ));
    assert_eq!(fingerprint(&db), before);
    assert_eq!(std::fs::read_dir(recovery_directory).unwrap().count(), 1);
}
#[test]
fn restored_unfinished_tasks_are_interrupted_and_old_picker_tokens_are_invalidated() {
    let (d, db, imports) = setup();
    let file = source(d.path(), "one.bin");
    import(&db, &file);
    let mut backup = Backup::snapshot(&db.connection.lock().unwrap()).unwrap();
    backup.data.tasks[0].status = "running".into();
    backup.data.tasks[0].finished_at = None;
    let old_revision = backup.data.tasks[0].revision.0;
    let path = d.path().join("running.json");
    write_new(&path, &backup.bytes().unwrap()).unwrap();
    let selection = imports.select(vec![file]).unwrap();
    let token = selection.selection_token.unwrap();
    let state = BackupState::default();
    let preview = state.prepare(&db, &imports, &path).unwrap();
    state
        .restore(&db, &imports, &preview.restore_token)
        .unwrap();
    let task = crate::library::task_get(&db, "1").unwrap();
    assert!(matches!(task.status, TaskStatus::Interrupted));
    assert_eq!(task.revision, (old_revision + 1).to_string());
    assert!(imports.take(&token).is_err());
}
#[test]
fn empty_library_roundtrip_and_exact_decimal_identifiers_are_supported() {
    let (d, db, imports) = setup();
    let path = d.path().join("empty.json");
    save(&db, &imports, &path).unwrap();
    let backup = read_backup(&path).unwrap();
    assert!(backup.data.files.is_empty());
    let state = BackupState::default();
    let preview = state.prepare(&db, &imports, &path).unwrap();
    assert_eq!(preview.item_count, 0);
    state
        .restore(&db, &imports, &preview.restore_token)
        .unwrap();
    let max = Decimal(i64::MAX);
    assert_eq!(
        serde_json::to_string(&max).unwrap(),
        "\"9223372036854775807\""
    );
    assert_eq!(
        serde_json::from_str::<Decimal>("\"9223372036854775807\"")
            .unwrap()
            .0,
        i64::MAX
    );
    assert!(serde_json::from_str::<Decimal>("1").is_err());
    assert!(serde_json::from_str::<Decimal>("\"01\"").is_err());
}

#[test]
fn schema3_backup_v1_restores_without_signature_claims() {
    let (d, db, imports) = setup();
    let file = source(d.path(), "legacy.nes");
    import(&db, &file);
    let backup = Backup::snapshot(&db.connection.lock().unwrap()).unwrap();
    let mut legacy = serde_json::to_value(&backup).unwrap();
    legacy["backupVersion"] = serde_json::json!(1);
    legacy["databaseSchemaVersion"] = serde_json::json!(3);
    legacy["data"]
        .as_object_mut()
        .unwrap()
        .remove("fileIdentifications");
    let path = d.path().join("legacy.json");
    std::fs::write(&path, serde_json::to_vec(&legacy).unwrap()).unwrap();
    let state = BackupState::default();
    let preview = state.prepare(&db, &imports, &path).unwrap();
    state
        .restore(&db, &imports, &preview.restore_token)
        .unwrap();
    let c = db.connection.lock().unwrap();
    let count: i64 = c
        .query_row("SELECT count(*) FROM file_identifications", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(count, 0);
    let files = crate::identification::files(&c, 1, "", 50).unwrap();
    assert!(matches!(
        files[0].format_evidence,
        IdentificationEvidence::Extension
    ));
    assert!(files[0].detector_version.is_none());
}

#[test]
fn manual_verification_survives_backup_and_restored_copies_require_recheck() {
    let (d, db, imports) = setup();
    let original = source(d.path(), "verify.bin");
    import(&db, &original);
    std::fs::write(&original, b"xyz").unwrap();
    let request = StartVerifyRequest {
        request_id: Uuid::new_v4().to_string(),
        item_id: "1".into(),
    };
    let (task, _) = crate::verification::accept(&db, &request).unwrap();
    crate::verification::run(&db, &task.task_id, "1").unwrap();
    assert_eq!(crate::verification::summary(&db, "1").unwrap().changed, 1);
    let path = d.path().join("verified-backup.json");
    save(&db, &imports, &path).unwrap();
    let state = BackupState::default();
    let preview = state.prepare(&db, &imports, &path).unwrap();
    state
        .restore(&db, &imports, &preview.restore_token)
        .unwrap();
    assert_eq!(
        crate::verification::summary(&db, "1").unwrap().unverified,
        1
    );
    assert!(
        crate::verification::summary(&db, "1")
            .unwrap()
            .last_checked_at
            .is_none()
    );
    assert!(!crate::verification::accept(&db, &request).unwrap().1);
    let next = StartVerifyRequest {
        request_id: Uuid::new_v4().to_string(),
        item_id: "1".into(),
    };
    let (task, _) = crate::verification::accept(&db, &next).unwrap();
    crate::verification::run(&db, &task.task_id, "1").unwrap();
    assert_eq!(crate::verification::summary(&db, "1").unwrap().changed, 1);
}
