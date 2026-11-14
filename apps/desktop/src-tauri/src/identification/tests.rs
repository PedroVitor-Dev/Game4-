use super::*;
fn bytes(prefix: &[u8], size: usize) -> Vec<u8> {
    let mut b = vec![0; size];
    b[..prefix.len()].copy_from_slice(prefix);
    b
}
fn setup() -> (tempfile::TempDir, crate::database::Database) {
    let d = tempfile::tempdir().unwrap();
    let db = crate::database::Database::open(&d.path().join("catalog.sqlite3")).unwrap();
    (d, db)
}
fn import(db: &crate::database::Database, path: &std::path::Path) {
    let task = crate::library::create_task(
        db,
        &StartImportRequest {
            request_id: uuid::Uuid::new_v4().to_string(),
            selection_token: uuid::Uuid::new_v4().to_string(),
        },
    )
    .unwrap();
    crate::library::run_import(db, &task.task_id, vec![path.to_path_buf()]).unwrap();
}
#[test]
fn signatures_cover_rom_endianness_archives_executables_and_disc_headers() {
    for (prefix, size, expected) in [
        (b"NES\x1a".as_slice(), 16, FileFormat::Ines),
        (&[0x80, 0x37, 0x12, 0x40], 64, FileFormat::N64Z64),
        (&[0x37, 0x80, 0x40, 0x12], 64, FileFormat::N64V64),
        (&[0x40, 0x12, 0x37, 0x80], 64, FileFormat::N64N64),
        (b"PK\x03\x04", 30, FileFormat::Zip),
        (b"PK\x05\x06", 22, FileFormat::Zip),
        (b"7z\xbc\xaf\x27\x1c", 32, FileFormat::SevenZip),
        (b"Rar!\x1a\x07\x00", 7, FileFormat::Rar),
        (b"Rar!\x1a\x07\x01\x00", 8, FileFormat::Rar),
        (b"\x1f\x8b\x08", 10, FileFormat::Gzip),
        (b"MComprHD", 16, FileFormat::Chd),
        (b"PS-X EXE", 0x800, FileFormat::PsxExe),
        (b"XBEH", 0x178, FileFormat::Xbe),
        (b"IWAD", 12, FileFormat::DoomWad),
    ] {
        assert_eq!(signature(&bytes(prefix, size)).unwrap().format, expected);
    }
    let mut pe = bytes(b"MZ", 128);
    pe[60..64].copy_from_slice(&64u32.to_le_bytes());
    pe[64..68].copy_from_slice(b"PE\0\0");
    assert_eq!(signature(&pe).unwrap().format, FileFormat::Pe);
    let mut elf = bytes(b"\x7fELF", 64);
    elf[4] = 2;
    elf[5] = 1;
    elf[6] = 1;
    assert_eq!(signature(&elf).unwrap().format, FileFormat::Elf);
    let mut iso = vec![0; 34816];
    iso[32768] = 1;
    iso[32769..32774].copy_from_slice(b"CD001");
    iso[32774] = 1;
    let disc = signature(&iso).unwrap();
    assert_eq!(disc.format, FileFormat::Iso9660);
    assert!(disc.platform.is_none());
}
#[test]
fn truncated_and_conflicting_names_never_override_header_evidence() {
    assert!(signature(b"NES\x1a").is_none());
    assert!(signature(b"MZ").is_none());
    assert!(signature(&bytes(b"7z\xbc\xaf\x27\x1c", 6)).is_none());
    let renamed = classify(
        "renamed.gba".into(),
        64,
        "a".repeat(64),
        Some("ines".into()),
        Some(1),
    )
    .unwrap();
    assert_eq!(renamed.platform, Some(PlatformId::Nes));
    assert_eq!(renamed.format_evidence, IdentificationEvidence::Signature);
    let archive = classify(
        "archive.nes".into(),
        100,
        "a".repeat(64),
        Some("zip".into()),
        Some(1),
    )
    .unwrap();
    assert_eq!(archive.format, FileFormat::Zip);
    assert!(archive.platform.is_none());
    let hint = classify("UPPER.GBA".into(), 3, "a".repeat(64), None, None).unwrap();
    assert_eq!(hint.platform, Some(PlatformId::GameBoyAdvance));
    assert_eq!(hint.format_evidence, IdentificationEvidence::Extension);
    for path in [
        "ambiguous.iso",
        "ambiguous.bin",
        "ambiguous.rom",
        "archive.zip",
        "README.md",
    ] {
        assert!(
            classify(path.into(), 3, "a".repeat(64), None, None)
                .unwrap()
                .platform
                .is_none()
        );
    }
    let empty = classify("empty.nes".into(), 0, "a".repeat(64), None, None).unwrap();
    assert_eq!(empty.format, FileFormat::Unknown);
    assert!(empty.platform.is_none());
    let mut pe = bytes(b"MZ", 64);
    pe[60..64].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(signature(&pe).is_none());
}
#[test]
fn import_persists_content_evidence_but_keeps_filename_hints_independent() {
    let (d, db) = setup();
    let root = d.path().join("Mixed");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("renamed.gba"), bytes(b"NES\x1a", 64)).unwrap();
    std::fs::write(root.join("copy.nes"), bytes(b"NES\x1a", 64)).unwrap();
    std::fs::write(root.join("game.GBA"), b"abc").unwrap();
    std::fs::write(root.join("ambiguous.iso"), b"abc").unwrap();
    std::fs::write(root.join("raw.bin"), b"abc").unwrap();
    std::fs::write(root.join("empty.gb"), b"").unwrap();
    import(&db, &root);
    {
        let c = db.connection.lock().unwrap();
        let rows = files(&c, 1, "", 50).unwrap();
        assert_eq!(rows.len(), 6);
        assert_eq!(
            rows.iter()
                .find(|r| r.path == "renamed.gba")
                .unwrap()
                .platform,
            Some(PlatformId::Nes)
        );
        assert_eq!(
            rows.iter().find(|r| r.path == "game.GBA").unwrap().platform,
            Some(PlatformId::GameBoyAdvance)
        );
        assert!(
            rows.iter()
                .find(|r| r.path == "ambiguous.iso")
                .unwrap()
                .platform
                .is_none()
        );
        let (count, total, formats, platforms) = summary(&c, 1).unwrap();
        assert_eq!(count, 6);
        assert_eq!(total, "137");
        assert!(
            formats
                .iter()
                .any(|r| r.format == FileFormat::Ines && r.file_count == 2)
        );
        assert_eq!(platforms.len(), 2);
        let content_count: i64 = c
            .query_row("SELECT count(*) FROM file_identifications", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(content_count, 3);
        c.execute(
            "UPDATE collection_items SET platform='User declared platform' WHERE id=1",
            [],
        )
        .unwrap();
    }
    import(&db, &root);
    let page = crate::library::library_list(
        &db,
        LibraryListRequest {
            query: None,
            page: PageRequest {
                cursor: None,
                limit: 50,
            },
        },
    )
    .unwrap();
    assert_eq!(
        page.items[0].platform.as_deref(),
        Some("User declared platform")
    );
    assert_eq!(page.items[0].detected_platforms.len(), 2);
    let manifest = crate::manifests::from_catalog(&db, "1").unwrap();
    assert_eq!(manifest.schema_version, 1);
    assert_eq!(
        manifest.extensions["org.game4.identification"]["files"]
            .as_array()
            .unwrap()
            .len(),
        6
    );
    let backup_path = d.path().join("identification-backup.json");
    let import_state = crate::library::ImportState::default();
    crate::backups::save(&db, &import_state, &backup_path).unwrap();
    db.connection
        .lock()
        .unwrap()
        .execute("DELETE FROM file_identifications", [])
        .unwrap();
    let backup_state = crate::backups::BackupState::default();
    let preview = backup_state
        .prepare(&db, &import_state, &backup_path)
        .unwrap();
    backup_state
        .restore(&db, &import_state, &preview.restore_token)
        .unwrap();
    let c = db.connection.lock().unwrap();
    let restored = files(&c, 1, "", 50).unwrap();
    let renamed = restored.iter().find(|r| r.path == "renamed.gba").unwrap();
    assert_eq!(renamed.platform, Some(PlatformId::Nes));
    assert_eq!(renamed.format_evidence, IdentificationEvidence::Signature);
    assert_eq!(renamed.detector_version, Some(1));
}
#[test]
fn details_are_paginated_and_cursors_are_bound_to_the_item() {
    let (d, db) = setup();
    let root = d.path().join("files");
    std::fs::create_dir(&root).unwrap();
    for name in ["a.nes", "b.gba", "日本語.bin"] {
        std::fs::write(root.join(name), b"abc").unwrap();
    }
    import(&db, &root);
    let c = db.connection.lock().unwrap();
    let first = item_files(
        &c,
        LibraryItemFilesRequest {
            item_id: "1".into(),
            page: PageRequest {
                cursor: None,
                limit: 1,
            },
        },
    )
    .unwrap();
    assert_eq!(first.files[0].path, "a.nes");
    let second = item_files(
        &c,
        LibraryItemFilesRequest {
            item_id: "1".into(),
            page: PageRequest {
                cursor: first.next_cursor.clone(),
                limit: 1,
            },
        },
    )
    .unwrap();
    assert_eq!(second.files[0].path, "b.gba");
    assert!(
        item_files(
            &c,
            LibraryItemFilesRequest {
                item_id: "1".into(),
                page: PageRequest {
                    cursor: Some("item-files-v1-2:61".into()),
                    limit: 1
                }
            }
        )
        .is_err()
    );
    assert!(
        item_files(
            &c,
            LibraryItemFilesRequest {
                item_id: "1".into(),
                page: PageRequest {
                    cursor: None,
                    limit: 0
                }
            }
        )
        .is_err()
    );
    assert!(matches!(
        item_files(
            &c,
            LibraryItemFilesRequest {
                item_id: "99".into(),
                page: PageRequest {
                    cursor: None,
                    limit: 1
                }
            }
        )
        .unwrap_err()
        .code,
        ErrorCode::NotFound
    ));
}
#[test]
fn totals_use_exact_wide_arithmetic_instead_of_sqlite_sum() {
    let (_d, db) = setup();
    let c = db.connection.lock().unwrap();
    c.execute(
        "INSERT INTO files(id,sha256,size_bytes) VALUES (1,?1,?2)",
        rusqlite::params!["a".repeat(64), i64::MAX],
    )
    .unwrap();
    c.execute(
        "INSERT INTO collection_items(id,title) VALUES (1,'Large')",
        [],
    )
    .unwrap();
    c.execute(
        "INSERT INTO collection_item_files VALUES (1,1,'a.bin'),(1,1,'b.bin')",
        [],
    )
    .unwrap();
    let (count, total, _, _) = summary(&c, 1).unwrap();
    assert_eq!(count, 2);
    assert_eq!(total, "18446744073709551614");
}
#[test]
fn schema3_migration_preserves_catalog_and_does_not_invent_signature_observations() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("legacy.sqlite3");
    let c = Connection::open(&path).unwrap();
    c.execute_batch(include_str!("../database/migrations/0001_initial.sql"))
        .unwrap();
    c.execute_batch(include_str!("../database/migrations/0002_import.sql"))
        .unwrap();
    c.execute_batch(include_str!(
        "../database/migrations/0003_manifest_identity.sql"
    ))
    .unwrap();
    c.execute(
        "INSERT INTO files(id,sha256,size_bytes) VALUES (1,?1,3)",
        ["a".repeat(64)],
    )
    .unwrap();
    c.execute_batch("INSERT INTO collection_items(id,title,platform) VALUES (1,'Keep','Declared'); INSERT INTO collection_item_files VALUES (1,1,'old.nes'); PRAGMA user_version=3;").unwrap();
    drop(c);
    let db = crate::database::Database::open(&path).unwrap();
    assert_eq!(db.info().unwrap().schema_version, 5);
    let c = db.connection.lock().unwrap();
    let records = files(&c, 1, "", 50).unwrap();
    assert_eq!(
        records[0].format_evidence,
        IdentificationEvidence::Extension
    );
    assert!(records[0].detector_version.is_none());
}
