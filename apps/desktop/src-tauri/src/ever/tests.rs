use super::*;
use std::fs;
fn example(name: &str) -> EverManifest {
    let bytes = match name {
        "original" => {
            include_bytes!("../../../../../packages/g4e-format/examples/original-files.g4e")
                .as_slice()
        }
        _ => include_bytes!("../../../../../packages/g4e-format/examples/package.g4e").as_slice(),
    };
    parse(bytes).unwrap()
}
fn fixture(root: &Path) {
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::create_dir_all(root.join("game")).unwrap();
    fs::write(root.join("docs/日本語.txt"), "Preservação ∞\n").unwrap();
    fs::write(root.join("empty.bin"), b"").unwrap();
    fs::write(root.join("game/hello.txt"), b"Game4 package fixture\n").unwrap();
}
#[test]
fn public_examples_and_shared_semantic_corpus_match_node_contract() {
    for name in ["original", "package"] {
        let doc = example(name);
        assert_eq!(
            serialize(&parse(&serialize(&doc).unwrap()).unwrap()).unwrap(),
            serialize(&doc).unwrap()
        );
    }
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../../../packages/g4e-format/invalid-cases.json"
    ))
    .unwrap();
    for c in corpus.as_array().unwrap() {
        assert!(
            parse(&serde_json::to_vec(&c["document"]).unwrap()).is_err(),
            "{}",
            c["name"]
        );
    }
}
#[test]
fn strict_json_rejects_duplicates_unpaired_unicode_missing_null_and_depth() {
    let bytes = serialize(&example("original")).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    let duplicate = text.replacen(
        "\"schemaVersion\": 1",
        "\"schemaVersion\": 1, \"schemaVer\\u0073ion\": 1",
        1,
    );
    assert!(parse(duplicate.as_bytes()).is_err());
    let mut v = serde_json::to_value(example("original")).unwrap();
    v["torrents"][0]
        .as_object_mut()
        .unwrap()
        .remove("metainfoFile");
    assert!(parse(&serde_json::to_vec(&v).unwrap()).is_err());
    assert!(parse(br#"{"x":"\ud800"}"#).is_err());
    assert!(parse(&[0xef, 0xbb, 0xbf, b'{', b'}']).is_err());
    let deep = format!("{}0{}", "[".repeat(66), "]".repeat(66));
    assert!(matches!(
        parse(deep.as_bytes()).unwrap_err().code,
        ErrorCode::G4eLimitExceeded
    ));
}
#[test]
fn independent_torrents_verify_exact_originals_and_complete_package() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("originals");
    fixture(&root);
    verify(
        &example("original"),
        include_bytes!("../../../../../packages/g4e-format/examples/original-files.torrent"),
        &root,
    )
    .unwrap();
    let package = d.path().join("synthetic-collection.g4");
    fs::write(
        &package,
        include_bytes!("../../../../../packages/g4-format/fixtures/v1-minimal.g4"),
    )
    .unwrap();
    verify(
        &example("package"),
        include_bytes!("../../../../../packages/g4e-format/examples/package.torrent"),
        &package,
    )
    .unwrap();
    fs::write(root.join("game/hello.txt"), b"changed").unwrap();
    assert!(
        verify(
            &example("original"),
            include_bytes!("../../../../../packages/g4e-format/examples/original-files.torrent"),
            &root
        )
        .is_err()
    );
}
#[test]
fn create_originals_does_not_copy_payload_and_hashes_pieces_across_files() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("source");
    fs::create_dir(&root).unwrap();
    let a = vec![3u8; PIECE_LENGTH - 7];
    let b = vec![9u8; 31];
    fs::write(root.join("a.bin"), &a).unwrap();
    fs::create_dir(root.join("nested")).unwrap();
    fs::write(root.join("nested/β.bin"), &b).unwrap();
    fs::write(root.join("z-empty.bin"), b"").unwrap();
    let mut manifest = example("original").manifest;
    manifest.files = [
        ("a.bin", a.clone()),
        ("nested/β.bin", b.clone()),
        ("z-empty.bin", vec![]),
    ]
    .into_iter()
    .map(|(name, data)| ManifestFile {
        path: name.into(),
        size_bytes: data.len().to_string(),
        sha256: format!("{:x}", Sha256::digest(&data)),
    })
    .collect();
    let before = fs::metadata(root.join("a.bin"))
        .unwrap()
        .modified()
        .unwrap();
    let c = create(&manifest, &root).unwrap();
    assert_eq!(c.torrent, create(&manifest, &root).unwrap().torrent);
    verify(&c.document, &c.torrent, &root).unwrap();
    let joined = [a, b].concat();
    let expected: Vec<u8> = joined
        .chunks(PIECE_LENGTH)
        .flat_map(|chunk| Sha1::digest(chunk).to_vec())
        .collect();
    let mut marker = b"6:pieces40:".to_vec();
    marker.extend_from_slice(&expected);
    assert!(c.torrent.windows(marker.len()).any(|v| v == marker));
    assert_eq!(fs::read_dir(&root).unwrap().count(), 3);
    assert_eq!(
        fs::metadata(root.join("a.bin"))
            .unwrap()
            .modified()
            .unwrap(),
        before
    );
    let pair = d.path().join("share.g4e");
    let (_, torrent) = write_pair(&pair, &c).unwrap();
    let readback = read(&pair).unwrap();
    verify(&readback, &fs::read(d.path().join(torrent)).unwrap(), &root).unwrap();
    assert!(write_pair(&pair, &c).is_err());
    assert_eq!(fs::read_dir(&root).unwrap().count(), 3);
}
#[test]
fn exact_piece_boundary_and_empty_file_have_no_spurious_final_piece() {
    let d = tempfile::tempdir().unwrap();
    let source = d.path().join("test.bin");
    let mut m = example("original").manifest;
    for data in [vec![], vec![8u8; PIECE_LENGTH], vec![8u8; PIECE_LENGTH + 1]] {
        fs::write(&source, &data).unwrap();
        m.files = vec![ManifestFile {
            path: "test.bin".into(),
            size_bytes: data.len().to_string(),
            sha256: format!("{:x}", Sha256::digest(&data)),
        }];
        let c = create(&m, &source).unwrap();
        verify(&c.document, &c.torrent, &source).unwrap();
        let expected: Vec<u8> = data
            .chunks(PIECE_LENGTH)
            .flat_map(|chunk| Sha1::digest(chunk).to_vec())
            .collect();
        let mut marker = format!("6:pieces{}:", expected.len()).into_bytes();
        marker.extend_from_slice(&expected);
        assert!(c.torrent.windows(marker.len()).any(|v| v == marker));
    }
}
#[test]
fn stale_and_missing_inputs_fail_without_repair() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("originals");
    fixture(&root);
    let m = example("original").manifest;
    fs::write(root.join("game/hello.txt"), b"Game4 package fixturX\n").unwrap();
    assert!(create(&m, &root).is_err());
    assert_eq!(
        fs::read(root.join("game/hello.txt")).unwrap(),
        b"Game4 package fixturX\n"
    );
    fs::remove_file(root.join("empty.bin")).unwrap();
    assert!(create(&m, &root).is_err());
    assert!(!root.join("empty.bin").exists());
}
#[test]
fn package_creation_is_explicit_and_checks_embedded_collection_identity() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("test.g4");
    let bytes = include_bytes!("../../../../../packages/g4-format/fixtures/v1-minimal.g4");
    fs::write(&path, bytes).unwrap();
    let c = create_package(&path).unwrap();
    assert!(matches!(
        c.document.representation,
        EverRepresentation::G4Package
    ));
    verify(&c.document, &c.torrent, &path).unwrap();
    let mut changed = c.document;
    changed.manifest.collection.id = Uuid::new_v4().to_string();
    assert!(verify(&changed, &c.torrent, &path).is_err());
    assert_eq!(fs::read(&path).unwrap(), bytes);
}
#[test]
fn mismatched_torrent_and_malformed_bencoding_are_rejected() {
    let d = tempfile::tempdir().unwrap();
    fixture(d.path());
    let raw = include_bytes!("../../../../../packages/g4e-format/examples/original-files.torrent");
    let mut with_comment = b"d7:comment1:x".to_vec();
    with_comment.extend_from_slice(&raw[1..]);
    assert!(verify(&example("original"), &with_comment, d.path()).is_err());
    let mut magnet_only = example("original");
    magnet_only.torrents[0].metainfo_file = None;
    verify(&magnet_only, &with_comment, d.path()).unwrap(); // Unchanged info bytes; only the full-file companion digest changed.

    let mut bytes =
        include_bytes!("../../../../../packages/g4e-format/examples/original-files.torrent")
            .to_vec();
    bytes[10] ^= 1;
    assert!(verify(&example("original"), &bytes, d.path()).is_err());
    for invalid in [
        b"di0ei1ee".as_slice(),
        b"d4:infoi01ee",
        b"d4:infoi0e4:infoi0ee",
        b"d4:infod6:pieces999999:ee",
    ] {
        assert!(metainfo::bind(&example("original"), invalid).is_err());
    }
}
#[test]
fn catalog_export_uses_existing_native_paths_and_keeps_reference_health() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("originals");
    fixture(&root);
    let db = Database::open(&d.path().join("catalog.sqlite3")).unwrap();
    let request = StartImportRequest {
        request_id: Uuid::new_v4().to_string(),
        selection_token: Uuid::new_v4().to_string(),
    };
    let job = library::create_task(&db, &request).unwrap();
    library::run_import(&db, &job.task_id, vec![root.clone()]).unwrap();
    let c = from_catalog(&db, "1").unwrap();
    verify(&c.document, &c.torrent, &root).unwrap();
    assert_eq!(crate::verification::summary(&db, "1").unwrap().verified, 3);
    fs::write(root.join("empty.bin"), b"changed").unwrap();
    assert!(from_catalog(&db, "1").is_err());
    assert_eq!(crate::verification::summary(&db, "1").unwrap().verified, 3);
}

#[cfg(windows)]
#[test]
fn exclusively_locked_original_fails_without_changing_bytes() {
    use std::os::windows::fs::OpenOptionsExt;
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("originals");
    fixture(&root);
    let source = root.join("game/hello.txt");
    let locked = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&source)
        .unwrap();
    assert!(create(&example("original").manifest, &root).is_err());
    drop(locked);
    assert_eq!(fs::read(source).unwrap(), b"Game4 package fixture\n");
}
