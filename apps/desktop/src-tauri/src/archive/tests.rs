use super::*;
use crate::database::Database;
fn db() -> (tempfile::TempDir, Database) {
    let d = tempfile::tempdir().unwrap();
    let db = Database::open(&d.path().join("game4.sqlite3")).unwrap();
    (d, db)
}
fn setup(c: &mut Connection) -> (String, String, String) {
    let game = ArchiveGame {
        id: id(),
        title: "Synthetic homebrew".into(),
        year: Some(2026),
        developer: "Fixture".into(),
        publisher: String::new(),
        genre: "Puzzle".into(),
        description: String::new(),
        history: String::new(),
        created_at: String::new(),
        updated_at: String::new(),
    };
    let gid = game.id.clone();
    execute(c, ArchiveRequest::SaveGame { value: game }).unwrap();
    let release = ArchiveRelease {
        id: id(),
        game_id: gid.clone(),
        platform_id: "platform-20".into(),
        region: "World".into(),
        languages: "English".into(),
        title_id: String::new(),
    };
    let rid = release.id.clone();
    execute(c, ArchiveRequest::SaveRelease { value: release }).unwrap();
    let edition = ArchiveEdition {
        id: id(),
        release_id: rid,
        name: "Standard".into(),
        medium: "Digital".into(),
    };
    let eid = edition.id.clone();
    execute(c, ArchiveRequest::SaveEdition { value: edition }).unwrap();
    let base = content(&eid, "version", None);
    let bid = base.id.clone();
    execute(c, ArchiveRequest::SaveContent { value: base }).unwrap();
    (gid, eid, bid)
}
fn content(edition: &str, kind: &str, base: Option<String>) -> ArchiveContent {
    ArchiveContent {
        id: id(),
        edition_id: edition.into(),
        kind: kind.into(),
        name: kind.into(),
        version: "r1-NTSC".into(),
        build: String::new(),
        release_date: String::new(),
        format: String::new(),
        base_id: base,
        description: String::new(),
        changelog: String::new(),
        identifier: String::new(),
    }
}
#[test]
fn compatibility_is_explicit_and_dependencies_cannot_cycle() {
    let (_d, db) = db();
    let mut c = db.connection.lock().unwrap();
    let (g, e, b) = setup(&mut c);
    let update = content(&e, "update", Some(b.clone()));
    let u = update.id.clone();
    execute(&mut c, ArchiveRequest::SaveContent { value: update }).unwrap();
    let dlc = content(&e, "dlc", Some(b.clone()));
    let d = dlc.id.clone();
    execute(&mut c, ArchiveRequest::SaveContent { value: dlc }).unwrap();
    dependency(
        &c,
        &ArchiveDependency {
            content_id: d.clone(),
            required_id: u.clone(),
        },
    )
    .unwrap();
    assert!(
        dependency(
            &c,
            &ArchiveDependency {
                content_id: u.clone(),
                required_id: d
            }
        )
        .is_err()
    );
    let (_, other, otherbase) = setup(&mut c);
    assert!(
        execute(
            &mut c,
            ArchiveRequest::SaveContent {
                value: content(&other, "update", Some(b.clone()))
            }
        )
        .is_err()
    );
    assert!(
        execute(
            &mut c,
            ArchiveRequest::SaveContent {
                value: content(&e, "dlc", Some(otherbase))
            }
        )
        .is_err()
    );
    assert!(
        execute(
            &mut c,
            ArchiveRequest::SaveContent {
                value: content(&e, "update", None)
            }
        )
        .is_err()
    );
    let ArchiveResponse::Game {
        contents, files, ..
    } = detail(&c, &g).unwrap()
    else {
        panic!()
    };
    assert_eq!(contents.len(), 3);
    assert!(files.is_empty());
}
#[test]
fn catalog_roundtrip_is_offline_idempotent_and_cannot_overwrite_local_metadata() {
    let (_d, db) = db();
    let mut c = db.connection.lock().unwrap();
    let (g, e, b) = setup(&mut c);
    let update = content(&e, "update", Some(b));
    execute(&mut c, ArchiveRequest::SaveContent { value: update }).unwrap();
    let bytes = export_catalog(&c, "Synthetic catalog", "Test author").unwrap();
    let doc = parse_catalog(&bytes).unwrap();
    assert!(doc.data.preferences.is_empty());
    let before: u32 = c
        .query_row("SELECT count(*) FROM archive_games", [], |r| r.get(0))
        .unwrap();
    import_catalog(&mut c, &doc).unwrap();
    import_catalog(&mut c, &doc).unwrap();
    let after: u32 = c
        .query_row("SELECT count(*) FROM archive_games", [], |r| r.get(0))
        .unwrap();
    assert_eq!(after, before + 1);
    assert!(detail(&c, &g).is_ok());
    assert_eq!(
        c.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| r
            .get::<_, u32>(
            0
        ))
        .unwrap(),
        0
    );
}
#[test]
fn invalid_catalog_rolls_back_and_private_references_are_rejected() {
    let (_d, db) = db();
    let mut c = db.connection.lock().unwrap();
    setup(&mut c);
    let bytes = export_catalog(&c, "Catalog", "").unwrap();
    let mut doc = parse_catalog(&bytes).unwrap();
    doc.data.contents[0].edition_id = "missing-edition".into();
    assert!(import_catalog(&mut c, &doc).is_err());
    assert_eq!(
        c.query_row("SELECT count(*) FROM archive_catalogs", [], |r| r
            .get::<_, u32>(0))
            .unwrap(),
        0
    );
    let mut doc = parse_catalog(&bytes).unwrap();
    doc.schema_version = 999;
    assert!(validate_document(&doc).is_err());
    let mut doc = parse_catalog(&bytes).unwrap();
    doc.data.preferences.push(ArchivePreference {
        game_id: doc.data.games[0].id.clone(),
        favorite: true,
        in_library: true,
        visited_at: None,
    });
    assert!(validate_document(&doc).is_err());
}
#[test]
fn pagination_filters_and_preferences_are_real() {
    let (_d, db) = db();
    let mut c = db.connection.lock().unwrap();
    let (g, _, _) = setup(&mut c);
    let (q, total) = game_cards(
        &c,
        &ArchiveSearch {
            platform_id: "platform-20".into(),
            region: "World".into(),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(total, 1);
    assert_eq!(q[0].local_files, 0);
    let (_, total) = game_cards(
        &c,
        &ArchiveSearch {
            availability: "reference".into(),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(total, 0);
    execute(
        &mut c,
        ArchiveRequest::Preference {
            game_id: g,
            favorite: true,
            in_library: true,
        },
    )
    .unwrap();
    assert_eq!(
        game_cards(
            &c,
            &ArchiveSearch {
                scope: "favorites".into(),
                ..Default::default()
            }
        )
        .unwrap()
        .1,
        1
    );
}
#[test]
fn migration_keeps_original_collection_and_archive_survives_reopen() {
    let (d, db) = db();
    let p = db.path.clone();
    {
        let mut c = db.connection.lock().unwrap();
        c.execute(
            "INSERT INTO collection_items(title) VALUES('Existing original')",
            [],
        )
        .unwrap();
        setup(&mut c);
    }
    drop(db);
    let db = Database::open(&p).unwrap();
    assert_eq!(db.info().unwrap().item_count, 1);
    let c = db.connection.lock().unwrap();
    assert_eq!(game_cards(&c, &ArchiveSearch::default()).unwrap().1, 1);
    drop(c);
    drop(db);
    drop(d);
}
#[test]
fn source_is_a_reference_not_proof_of_availability() {
    let (_d, db) = db();
    let mut c = db.connection.lock().unwrap();
    let (g, _, b) = setup(&mut c);
    let source = ArchiveSource {
        id: id(),
        content_id: b,
        magnet: "magnet:?xt=urn:btih:0123456789012345678901234567890123456789".into(),
        license: "CC0 synthetic fixture".into(),
        attribution: String::new(),
        authorized: true,
    };
    execute(&mut c, ArchiveRequest::SaveSource { value: source }).unwrap();
    let ArchiveResponse::Game { card, states, .. } = detail(&c, &g).unwrap() else {
        panic!()
    };
    assert_eq!(card.source_count, 1);
    assert_eq!(card.local_files, 0);
    assert!(states.is_empty());
    let doc = parse_catalog(&export_catalog(&c, "Test", "").unwrap()).unwrap();
    assert!(!doc.data.p2p_sources[0].authorized);
}

#[test]
fn public_catalog_fixture_is_readable_and_duplicate_ids_are_rejected() {
    let bytes = include_bytes!("../../../../../packages/g4lib-format/synthetic.g4lib");
    let mut doc = parse_catalog(bytes).unwrap();
    assert_eq!(doc.data.contents.len(), 3);
    doc.data.games.push(doc.data.games[0].clone());
    assert!(validate_document(&doc).is_err());
}

#[test]
fn large_catalog_reads_only_a_bounded_page() {
    let (_d, db) = db();
    let mut c = db.connection.lock().unwrap();
    let tx = c.transaction().unwrap();
    {
        let mut q=tx.prepare("INSERT INTO archive_games(id,title,developer,publisher,genre,description,history) VALUES(?1,?2,'','','','','')").unwrap();
        for n in 0..10000 {
            q.execute(params![
                format!("fixture-{n}"),
                format!("Synthetic game {n:05}")
            ])
            .unwrap();
        }
    }
    tx.commit().unwrap();
    let (rows, total) = game_cards(
        &c,
        &ArchiveSearch {
            offset: 48,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(total, 10000);
    assert_eq!(rows.len(), 48);
    assert_eq!(rows[0].game.title, "Synthetic game 00048");
    let (rows, total) = game_cards(
        &c,
        &ArchiveSearch {
            search: "09999".into(),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(total, 1);
    assert_eq!(rows.len(), 1);
}

#[test]
fn repeated_manifest_attachment_is_idempotent_and_dependencies_do_not_cross_editions() {
    let (_d, db) = db();
    let mut c = db.connection.lock().unwrap();
    let (_g, e, b) = setup(&mut c);
    let bytes = include_bytes!("../../../../../packages/g4e-format/examples/original-files.g4e");
    attach_ever(&mut c, &b, "Synthetic fixture permission", true, bytes).unwrap();
    attach_ever(&mut c, &b, "Synthetic fixture permission", true, bytes).unwrap();
    assert_eq!(
        c.query_row("SELECT count(*) FROM archive_preservation_files", [], |r| r
            .get::<_, u32>(0))
            .unwrap(),
        3
    );
    assert_eq!(
        c.query_row("SELECT count(*) FROM archive_p2p_sources", [], |r| r
            .get::<_, u32>(0))
            .unwrap(),
        1
    );
    let release: String = c
        .query_row(
            "SELECT release_id FROM archive_editions WHERE id=?",
            [e],
            |r| r.get(0),
        )
        .unwrap();
    let other = ArchiveEdition {
        id: id(),
        release_id: release,
        name: "Different edition".into(),
        medium: "Digital".into(),
    };
    other.put(&c).unwrap();
    let otherbase = content(&other.id, "version", None);
    otherbase.put(&c).unwrap();
    assert!(
        dependency(
            &c,
            &ArchiveDependency {
                content_id: b,
                required_id: otherbase.id
            }
        )
        .is_err()
    );
}
