use super::*;
fn example() -> Vec<u8> {
    include_bytes!("../../../../../packages/manifest-schema/examples/v1.example.json").to_vec()
}
#[test]
fn examples_roundtrip_and_shared_invalid_corpus_is_rejected() {
    let manifest = parse(&example()).unwrap();
    let bytes = serialize(&manifest).unwrap();
    assert_eq!(
        bytes,
        example(),
        "public example must match the Rust serializer"
    );
    assert_eq!(serialize(&parse(&bytes).unwrap()).unwrap(), bytes);
    let cases: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../packages/manifest-schema/invalid-cases.json"
    ))
    .unwrap();
    for case in cases.as_array().unwrap() {
        assert!(
            parse(&serde_json::to_vec(&case["manifest"]).unwrap()).is_err(),
            "{}",
            case["name"]
        );
    }
    let mut future = manifest.clone();
    future.schema_version = 2;
    assert!(matches!(
        validate(&future).unwrap_err().code,
        ErrorCode::ManifestVersionUnsupported
    ));
}
#[test]
fn preserves_unicode_extensions_precision_and_rejects_duplicate_fields_or_oversized_input() {
    let mut m = parse(&example()).unwrap();
    m.files[0].size_bytes = "9007199254740993".into();
    m.files[0].path = "Coleção ∞/日本語.bin".into();
    m.extensions.insert(
        "org.example.metadata".into(),
        serde_json::json!({"declared":true}),
    );
    let decoded = parse(&serialize(&m).unwrap()).unwrap();
    assert_eq!(
        decoded
            .files
            .iter()
            .find(|f| f.path.contains('∞'))
            .unwrap()
            .size_bytes,
        "9007199254740993"
    );
    assert_eq!(decoded.extensions, m.extensions);
    let duplicate =
        String::from_utf8(example())
            .unwrap()
            .replacen('{', "{\"format\":\"game4-manifest\",", 1);
    assert!(parse(duplicate.as_bytes()).is_err());
    assert!(parse(&vec![b' '; MAX_BYTES + 1]).is_err());
}
#[test]
fn catalog_export_is_stable_private_and_survives_restart_and_reimport() {
    let d = tempfile::tempdir().unwrap();
    let db_path = d.path().join("catalog.sqlite3");
    let db = Database::open(&db_path).unwrap();
    let root = d.path().join("Private source");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("abc.bin"), b"abc").unwrap();
    let request = crate::contracts::StartImportRequest {
        request_id: Uuid::new_v4().to_string(),
        selection_token: Uuid::new_v4().to_string(),
    };
    let task = crate::library::create_task(&db, &request).unwrap();
    crate::library::run_import(&db, &task.task_id, vec![root.clone()]).unwrap();
    let first = serialize(&from_catalog(&db, "1").unwrap()).unwrap();
    assert!(!String::from_utf8_lossy(&first).contains("Private source\\\\"));
    assert!(!String::from_utf8_lossy(&first).contains("path_key"));
    assert!(!String::from_utf8_lossy(&first).contains("selectionToken"));
    let manifest = parse(&first).unwrap();
    assert_eq!(manifest.files[0].path, "abc.bin");
    assert_eq!(
        manifest.files[0].sha256,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(serialize(&from_catalog(&db, "1").unwrap()).unwrap(), first);
    drop(db);
    let db = Database::open(&db_path).unwrap();
    assert_eq!(serialize(&from_catalog(&db, "1").unwrap()).unwrap(), first);
    let task = crate::library::create_task(
        &db,
        &crate::contracts::StartImportRequest {
            request_id: Uuid::new_v4().to_string(),
            selection_token: Uuid::new_v4().to_string(),
        },
    )
    .unwrap();
    crate::library::run_import(&db, &task.task_id, vec![root]).unwrap();
    assert_eq!(
        from_catalog(&db, "1").unwrap().collection.id,
        manifest.collection.id
    );
    assert!(matches!(
        from_catalog(&db, "999").unwrap_err().code,
        ErrorCode::NotFound
    ));
    assert!(item_id("01").is_err());
}
#[test]
fn export_creates_complete_json_and_never_overwrites_existing_data() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("manifest.v1.json");
    let bytes = serialize(&parse(&example()).unwrap()).unwrap();
    write_new(&path, &bytes).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert!(write_new(&path, &bytes).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let invalid_path = d.path().join("invalid.json");
    assert!(write_new(&invalid_path, b"{}").is_err());
    assert!(!invalid_path.exists());
    assert!(write_new(&d.path().join("source.bin"), &bytes).is_err());
}

#[test]
fn upgrades_v2_catalog_without_changing_existing_collection_rows() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("v2.sqlite3");
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute_batch(include_str!("../database/migrations/0001_initial.sql"))
        .unwrap();
    c.execute_batch(include_str!("../database/migrations/0002_import.sql"))
        .unwrap();
    c.execute(
        "INSERT INTO collection_items(id,title) VALUES (42,'Keep existing item')",
        [],
    )
    .unwrap();
    c.pragma_update(None, "user_version", 2).unwrap();
    drop(c);
    let db = Database::open(&path).unwrap();
    assert_eq!(db.info().unwrap().schema_version, 3);
    assert_eq!(db.info().unwrap().item_count, 1);
    let c = db.connection.lock().unwrap();
    let title: String = c
        .query_row("SELECT title FROM collection_items WHERE id=42", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(title, "Keep existing item");
    let identities: i64 = c
        .query_row("SELECT count(*) FROM collection_manifest_ids", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(identities, 0);
}
