use super::*;
fn setup() -> (tempfile::TempDir, Database) {
    let d = tempfile::tempdir().unwrap();
    let db = Database::open(&d.path().join("db/catalog.sqlite3")).unwrap();
    (d, db)
}
fn import(db: &Database, paths: Vec<PathBuf>) -> TaskAccepted {
    let state = ImportState::default();
    let selection = state.select(paths).unwrap();
    let request = StartImportRequest {
        request_id: Uuid::new_v4().to_string(),
        selection_token: selection.selection_token.unwrap(),
    };
    let paths = state.take(&request.selection_token).unwrap();
    let task = create_task(db, &request).unwrap();
    run_import(db, &task.task_id, paths).unwrap();
    task
}
fn listing(db: &Database, limit: u16, cursor: Option<String>) -> LibraryPage {
    library_list(
        db,
        LibraryListRequest {
            page: PageRequest { cursor, limit },
            query: None,
        },
    )
    .unwrap()
}
#[test]
fn recursive_import_hashes_deduplicates_and_preserves_sources() {
    let (d, db) = setup();
    let root = d.path().join("Coleção ∞");
    fs::create_dir_all(root.join("nested")).unwrap();
    fs::write(root.join("a.bin"), b"abc").unwrap();
    fs::write(root.join("nested/copy.bin"), b"abc").unwrap();
    fs::write(root.join("empty.bin"), b"").unwrap();
    let job = import(&db, vec![root.clone()]);
    let stats = db.info().unwrap();
    assert_eq!(stats.file_count, 2);
    assert_eq!(stats.location_count, 3);
    assert_eq!(stats.item_count, 1);
    let snapshot = task_get(&db, &job.task_id).unwrap();
    assert!(matches!(snapshot.status, TaskStatus::Succeeded));
    assert_eq!(snapshot.processed_count, "3");
    let page = listing(&db, 50, None);
    assert_eq!(page.items[0].title, "Coleção ∞");
    assert_eq!(page.items[0].total_bytes, "6");
    assert_eq!(page.items[0].file_count, 3);
    let c = db.connection.lock().unwrap();
    let hash: String = c
        .query_row("SELECT sha256 FROM files WHERE size_bytes=3", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(
        hash,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    drop(c);
    import(&db, vec![root.clone()]);
    assert_eq!(db.info().unwrap().item_count, 1);
    assert_eq!(fs::read(root.join("a.bin")).unwrap(), b"abc");
    fs::write(root.join("a.bin"), b"changed").unwrap();
    import(&db, vec![root]);
    assert_eq!(db.info().unwrap().item_count, 1);
    assert_eq!(db.info().unwrap().file_count, 3);
    assert_eq!(listing(&db, 50, None).items[0].total_bytes, "10");
}
#[test]
fn failure_leaves_no_partial_catalog_and_can_be_recovered_after_restart() {
    let (d, db) = setup();
    let file = d.path().join("valid.bin");
    fs::write(&file, b"keep").unwrap();
    let request = StartImportRequest {
        request_id: Uuid::new_v4().to_string(),
        selection_token: Uuid::new_v4().to_string(),
    };
    let job = create_task(&db, &request).unwrap();
    assert!(run_import(&db, &job.task_id, vec![file, d.path().join("missing")]).is_err());
    assert_eq!(db.info().unwrap().item_count, 0);
    assert_eq!(db.info().unwrap().file_count, 0);
    assert!(matches!(
        task_get(&db, &job.task_id).unwrap().status,
        TaskStatus::Failed
    ));
    let pending = create_task(
        &db,
        &StartImportRequest {
            request_id: Uuid::new_v4().to_string(),
            selection_token: Uuid::new_v4().to_string(),
        },
    )
    .unwrap();
    drop(db);
    let reopened = Database::open(&d.path().join("db/catalog.sqlite3")).unwrap();
    assert!(matches!(
        task_get(&reopened, &pending.task_id).unwrap().status,
        TaskStatus::Interrupted
    ));
}
#[test]
fn tokens_requests_and_pagination_reject_invalid_or_conflicting_values() {
    let (d, db) = setup();
    let state = ImportState::default();
    let f = d.path().join("one.bin");
    fs::write(&f, b"1").unwrap();
    assert!(state.select(vec![]).unwrap().cancelled);
    let selected = state.select(vec![f.clone()]).unwrap();
    let token = selected.selection_token.unwrap();
    state.take(&token).unwrap();
    assert!(state.take(&token).is_err());
    let expired = Uuid::new_v4().to_string();
    state.selections.lock().unwrap().insert(
        expired.clone(),
        Selection {
            paths: vec![f.clone()],
            created: Instant::now() - Duration::from_secs(301),
        },
    );
    assert!(state.take(&expired).is_err());
    let request = StartImportRequest {
        request_id: Uuid::new_v4().to_string(),
        selection_token: token,
    };
    validate_request(&request).unwrap();
    let job = create_task(&db, &request).unwrap();
    assert_eq!(
        existing(&db, &request).unwrap().unwrap().task_id,
        job.task_id
    );
    assert!(
        existing(
            &db,
            &StartImportRequest {
                selection_token: Uuid::new_v4().to_string(),
                ..request
            }
        )
        .is_err()
    );
    assert!(
        validate_request(&StartImportRequest {
            request_id: "bad".into(),
            selection_token: Uuid::new_v4().to_string()
        })
        .is_err()
    );
    import(&db, vec![f]);
    let f2 = d.path().join("two.bin");
    fs::write(&f2, b"2").unwrap();
    import(&db, vec![f2]);
    let first = listing(&db, 1, None);
    let next = listing(&db, 1, first.next_cursor.clone());
    assert_ne!(first.items[0].id, next.items[0].id);
    assert!(next.next_cursor.is_none());
    assert!(
        library_list(
            &db,
            LibraryListRequest {
                page: PageRequest {
                    cursor: first.next_cursor,
                    limit: 1
                },
                query: Some("different".into())
            }
        )
        .is_err()
    );
    assert!(
        tasks_list(
            &db,
            PageRequest {
                cursor: None,
                limit: 0
            }
        )
        .is_err()
    );
    assert!(task_get(&db, "01").is_err());
}
#[test]
fn empty_directories_and_locked_files_are_rejected() {
    let (d, db) = setup();
    let root = d.path().join("empty");
    fs::create_dir(&root).unwrap();
    let request = StartImportRequest {
        request_id: Uuid::new_v4().to_string(),
        selection_token: Uuid::new_v4().to_string(),
    };
    let job = create_task(&db, &request).unwrap();
    assert!(run_import(&db, &job.task_id, vec![root]).is_err());
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        let file = d.path().join("locked.bin");
        fs::write(&file, b"locked").unwrap();
        let _locked = File::options()
            .write(true)
            .share_mode(0)
            .open(&file)
            .unwrap();
        assert!(hash_file(&file).is_err());
    }
}
#[test]
fn sql_failure_rolls_back_items_content_locations_and_success_state() {
    let (d, db) = setup();
    let a = d.path().join("a.bin");
    let b = d.path().join("b.bin");
    fs::write(&a, b"one").unwrap();
    fs::write(&b, b"two").unwrap();
    db.connection.lock().unwrap().execute_batch("CREATE TRIGGER reject_second BEFORE INSERT ON collection_items WHEN NEW.title='b.bin' BEGIN SELECT RAISE(ABORT,'injected failure'); END;").unwrap();
    let task = create_task(
        &db,
        &StartImportRequest {
            request_id: Uuid::new_v4().to_string(),
            selection_token: Uuid::new_v4().to_string(),
        },
    )
    .unwrap();
    assert!(run_import(&db, &task.task_id, vec![a, b]).is_err());
    let stats = db.info().unwrap();
    assert_eq!(stats.item_count, 0);
    assert_eq!(stats.file_count, 0);
    assert_eq!(stats.location_count, 0);
    assert!(matches!(
        task_get(&db, &task.task_id).unwrap().status,
        TaskStatus::Failed
    ));
}

#[cfg(windows)]
#[test]
fn junctions_and_paths_through_junctions_are_rejected() {
    let (d, db) = setup();
    let outside = d.path().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("data.bin"), b"outside").unwrap();
    let selected = d.path().join("selected");
    fs::create_dir(&selected).unwrap();
    let link = selected.join("junction");
    let output = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&link)
        .arg(&outside)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "Cannot create temporary junction fixture"
    );
    assert!(reject_link(&link).is_err());
    assert!(hash_file(&link.join("data.bin")).is_err());
    let request = StartImportRequest {
        request_id: Uuid::new_v4().to_string(),
        selection_token: Uuid::new_v4().to_string(),
    };
    let task = create_task(&db, &request).unwrap();
    assert!(run_import(&db, &task.task_id, vec![selected]).is_err());
    assert_eq!(db.info().unwrap().file_count, 0);
    // Remove the junction itself, without traversing its target.
    fs::remove_dir(&link).unwrap();
    assert_eq!(fs::read(outside.join("data.bin")).unwrap(), b"outside");
}

#[test]
fn catalog_and_task_queries_finish_while_background_sha256_is_in_progress() {
    use std::sync::{Arc, mpsc};
    let (d, db) = setup();
    let db = Arc::new(db);
    let file = d.path().join("stream.bin");
    // Cross multiple read-buffer boundaries with a partial final chunk.
    let payload = vec![0x5au8; 3 * 1024 * 1024 + 17];
    fs::write(&file, &payload).unwrap();
    let expected = format!("{:x}", Sha256::digest(&payload));
    let task = create_task(
        &db,
        &StartImportRequest {
            request_id: Uuid::new_v4().to_string(),
            selection_token: Uuid::new_v4().to_string(),
        },
    )
    .unwrap();
    let id = task.task_id.clone();
    let worker_db = Arc::clone(&db);
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    // Use the same blocking executor as the production IPC command.
    let worker = tauri::async_runtime::spawn_blocking(move || {
        let mut first = true;
        run_import_observed(&worker_db, &id, vec![file], &mut || {
            if first {
                first = false;
                entered_tx.send(()).unwrap();
                release_rx.recv_timeout(Duration::from_secs(10)).unwrap();
            }
        })
    });
    entered_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    let read_db = Arc::clone(&db);
    let read_id = task.task_id.clone();
    let (read_tx, read_rx) = mpsc::channel();
    let reader = tauri::async_runtime::spawn_blocking(move || {
        let snapshot = task_get(&read_db, &read_id).unwrap();
        assert!(matches!(snapshot.status, TaskStatus::Running));
        assert_eq!(snapshot.processed_count, "0");
        assert_eq!(snapshot.total_count.as_deref(), Some("1"));
        assert!(listing(&read_db, 50, None).items.is_empty());
        assert_eq!(read_db.info().unwrap().file_count, 0);
        read_tx.send(()).unwrap();
    });
    // The hash worker cannot finish until explicitly released. This proves
    // reads are independent of hashing, rather than merely racing a fast hash.
    let read_result = read_rx.recv_timeout(Duration::from_secs(5));
    release_tx.send(()).unwrap();
    tauri::async_runtime::block_on(worker).unwrap().unwrap();
    tauri::async_runtime::block_on(reader).unwrap();
    read_result.expect("catalog/task reads blocked behind hashing");
    let snapshot = task_get(&db, &task.task_id).unwrap();
    assert!(matches!(snapshot.status, TaskStatus::Succeeded));
    assert_eq!(snapshot.processed_count, "1");
    let hash: String = db
        .connection
        .lock()
        .unwrap()
        .query_row("SELECT sha256 FROM files", [], |row| row.get(0))
        .unwrap();
    assert_eq!(hash, expected);
}
