use super::*;
use std::{fs, io::Cursor, sync::Arc};
fn manifest() -> Manifest {
    manifests::parse(include_bytes!(
        "../../../../../packages/g4-format/fixtures/manifest.json"
    ))
    .unwrap()
}
fn payload() -> Vec<(String, Vec<u8>)> {
    vec![
        (
            "docs/日本語.txt".into(),
            "Preservação ∞\n".as_bytes().to_vec(),
        ),
        ("empty.bin".into(), vec![]),
        ("game/hello.txt".into(), b"Game4 package fixture\n".to_vec()),
    ]
}
fn archive(entries: Vec<(String, Vec<u8>)>) -> Vec<u8> {
    let mut z = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    for (name, bytes) in entries {
        z.start_file(name, options).unwrap();
        z.write_all(&bytes).unwrap();
    }
    z.finish().unwrap().into_inner()
}
fn entries(m: &Manifest) -> Vec<(String, Vec<u8>)> {
    let mut v = vec![
        (
            "game4.json".into(),
            serde_json::to_vec(&descriptor()).unwrap(),
        ),
        ("manifest.json".into(), manifests::serialize(m).unwrap()),
    ];
    v.extend(payload().into_iter().map(|(p, b)| (format!("data/{p}"), b)));
    v
}
fn read_bytes(bytes: &[u8]) -> Result<VerifiedPackage> {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("test.g4");
    fs::write(&path, bytes).unwrap();
    read(&path)
}
fn sources(d: &Path) -> Vec<Arc<Vec<PathBuf>>> {
    payload()
        .into_iter()
        .map(|(name, data)| {
            let path = d.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, data).unwrap();
            Arc::new(vec![path])
        })
        .collect()
}
#[test]
fn independent_python_zip32_and_zip64_fixtures_are_readable() {
    for bytes in [
        include_bytes!("../../../../../packages/g4-format/fixtures/v1-minimal.g4").as_slice(),
        include_bytes!("../../../../../packages/g4-format/fixtures/v1-zip64.g4").as_slice(),
    ] {
        let p = read_bytes(bytes).unwrap();
        assert_eq!(p.manifest.files.len(), 3);
        assert_eq!(p.total_bytes, 40);
    }
}
#[test]
fn streaming_creation_roundtrips_and_is_deterministic_without_overwriting() {
    let d = tempfile::tempdir().unwrap();
    let m = manifest();
    let paths = sources(d.path());
    let a = d.path().join("a.g4");
    let b = d.path().join("b.g4");
    create(&a, &m, &paths).unwrap();
    create(&b, &m, &paths).unwrap();
    assert_eq!(fs::read(&a).unwrap(), fs::read(&b).unwrap());
    assert_eq!(
        read(&a).unwrap().manifest.collection.title,
        m.collection.title
    );
    assert!(matches!(
        create(&a, &m, &paths).unwrap_err().code,
        ErrorCode::PackageWriteFailed
    ));
    assert!(create(&d.path().join("wrong.zip"), &m, &paths).is_err());
    assert_eq!(fs::read(&paths[2][0]).unwrap(), b"Game4 package fixture\n");
}
#[test]
fn mismatched_or_missing_sources_leave_no_package_or_temp_files_and_fallback_works() {
    let d = tempfile::tempdir().unwrap();
    let m = manifest();
    let mut paths = sources(d.path());
    let out = d.path().join("out.g4");
    fs::write(&paths[2][0], b"changed").unwrap();
    assert!(create(&out, &m, &paths).is_err());
    assert!(!out.exists());
    assert!(
        !fs::read_dir(d.path()).unwrap().any(|e| e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".tmp"))
    );
    let good = d.path().join("good");
    fs::write(&good, b"Game4 package fixture\n").unwrap();
    paths[2] = Arc::new(vec![paths[2][0].clone(), good]);
    create(&out, &m, &paths).unwrap();
    assert!(read(&out).is_ok());
    fs::remove_file(&paths[0][0]).unwrap();
    assert!(create(&d.path().join("missing.g4"), &m, &paths).is_err());
}
#[test]
fn profile_rejects_traversal_case_collisions_prefixes_and_oversized_payloads() {
    let mut m = manifest();
    m.files[0].path = "../bad".into();
    assert!(validate_manifest(&m).is_err());
    let mut m = manifest();
    m.files[0].path = "GAME/HELLO.TXT".into();
    assert!(validate_manifest(&m).is_err());
    let mut m = manifest();
    m.files[0].path = "GAME".into();
    assert!(validate_manifest(&m).is_err());
    let mut m = manifest();
    m.files[0].path = "a".repeat(1025);
    assert!(validate_manifest(&m).is_err());
    let mut m = manifest();
    m.files[0].size_bytes = (MAX_PAYLOAD + 1).to_string();
    assert!(matches!(
        validate_manifest(&m).unwrap_err().code,
        ErrorCode::PackageLimitExceeded
    ));
}
#[test]
fn rejects_corruption_wrong_hash_and_manifest_size() {
    let mut bytes =
        include_bytes!("../../../../../packages/g4-format/fixtures/v1-minimal.g4").to_vec();
    let marker = b"Game4 package fixture\n";
    let offset = bytes
        .windows(marker.len())
        .position(|b| b == marker)
        .unwrap();
    bytes[offset] ^= 1;
    assert!(matches!(
        read_bytes(&bytes).unwrap_err().code,
        ErrorCode::PackageIntegrityFailed
    ));
    let mut m = manifest();
    m.files[2].sha256 = "a".repeat(64);
    assert!(matches!(
        read_bytes(&archive(entries(&m))).unwrap_err().code,
        ErrorCode::PackageIntegrityFailed
    ));
    m.files[2].size_bytes = "1".into();
    assert!(read_bytes(&archive(entries(&m))).is_err());
}
#[test]
fn rejects_future_descriptor_unknown_fields_missing_extra_and_unsafe_entries() {
    let m = manifest();
    let mut e = entries(&m);
    let mut d = descriptor();
    d.package_version = 2;
    e[0].1 = serde_json::to_vec(&d).unwrap();
    assert!(matches!(
        read_bytes(&archive(e)).unwrap_err().code,
        ErrorCode::PackageVersionUnsupported
    ));
    let mut e = entries(&m);
    e[0].1=br#"{"$schema":"urn:game4:package:1","format":"game4-package","packageVersion":1,"manifest":"manifest.json","payloadPrefix":"data/","extra":true}"#.to_vec();
    assert!(read_bytes(&archive(e)).is_err());
    let mut e = entries(&m);
    e.pop();
    assert!(read_bytes(&archive(e)).is_err());
    let mut e = entries(&m);
    e.push(("extra.txt".into(), b"not declared".to_vec()));
    assert!(read_bytes(&archive(e)).is_err());
    let mut e = entries(&m);
    e[2].0 = "../escape".into();
    assert!(read_bytes(&archive(e)).is_err());
    let mut e = entries(&m);
    e[2].0 = "/absolute".into();
    assert!(read_bytes(&archive(e)).is_err());
    let mut e = entries(&m);
    e[2].0 = "data/docs/".into();
    assert!(read_bytes(&archive(e)).is_err());
}
#[test]
fn rejects_split_truncated_appended_and_hostile_directory_headers_before_indexing() {
    let source = include_bytes!("../../../../../packages/g4-format/fixtures/v1-minimal.g4");
    assert!(read_bytes(&source[..source.len() - 3]).is_err());
    let mut bytes = source.to_vec();
    bytes.push(0);
    assert!(read_bytes(&bytes).is_err());
    let mut bytes = source.to_vec();
    let end = bytes.len() - 22;
    bytes[end + 4] = 1;
    assert!(read_bytes(&bytes).is_err());
    let mut bytes =
        include_bytes!("../../../../../packages/g4-format/fixtures/v1-zip64.g4").to_vec();
    let at = bytes.windows(4).position(|v| v == b"PK\x06\x06").unwrap();
    bytes[at + 24..at + 32].copy_from_slice(&u64::MAX.to_le_bytes());
    bytes[at + 32..at + 40].copy_from_slice(&u64::MAX.to_le_bytes());
    assert!(matches!(
        read_bytes(&bytes).unwrap_err().code,
        ErrorCode::PackageLimitExceeded
    ));
}
#[test]
fn rejects_duplicate_names_local_header_mismatch_encryption_and_compression() {
    let mut e = entries(&manifest());
    e.push(("data/other.bin".into(), vec![]));
    let mut bytes = archive(e);
    // Two equal-length ZIP names, in both local and central records.
    for i in 0..=bytes.len() - 14 {
        if &bytes[i..i + 14] == b"data/other.bin" {
            bytes[i..i + 14].copy_from_slice(b"data/empty.bin");
        }
    }
    assert!(read_bytes(&bytes).is_err());
    let source = include_bytes!("../../../../../packages/g4-format/fixtures/v1-minimal.g4");
    let mut bytes = source.to_vec();
    bytes[30] = b'X';
    assert!(read_bytes(&bytes).is_err());
    for (local, central, value) in [(6, 8, 1), (8, 10, 8)] {
        let mut bytes = source.to_vec();
        let cd = bytes.windows(4).position(|v| v == b"PK\x01\x02").unwrap();
        bytes[local] = value;
        bytes[cd + central] = value;
        assert!(read_bytes(&bytes).is_err());
    }
}
#[test]
fn symlink_payloads_are_never_accepted() {
    let mut z = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default();
    let e = entries(&manifest());
    for (name, data) in e.into_iter().take(2) {
        z.start_file(name, options).unwrap();
        z.write_all(&data).unwrap();
    }
    z.add_symlink("data/game/hello.txt", "outside", options)
        .unwrap();
    for (name, data) in payload().into_iter().take(2) {
        z.start_file(format!("data/{name}"), options).unwrap();
        z.write_all(&data).unwrap();
    }
    assert!(read_bytes(&z.finish().unwrap().into_inner()).is_err());
}
#[test]
fn library_export_uses_valid_copy_without_changing_catalog_health() {
    let d = tempfile::tempdir().unwrap();
    let db = Database::open(&d.path().join("catalog.sqlite3")).unwrap();
    let a = d.path().join("a.bin");
    let b = d.path().join("b.bin");
    fs::write(&a, b"original").unwrap();
    fs::write(&b, b"original").unwrap();
    let req = crate::contracts::StartImportRequest {
        request_id: uuid::Uuid::new_v4().to_string(),
        selection_token: uuid::Uuid::new_v4().to_string(),
    };
    let task = library::create_task(&db, &req).unwrap();
    library::run_import(&db, &task.task_id, vec![a.clone(), b]).unwrap();
    fs::write(a, b"corrupt").unwrap();
    let (m, s) = from_catalog(&db, "1").unwrap();
    let path = d.path().join("library.g4");
    create(&path, &m, &s).unwrap();
    assert_eq!(
        read(&path).unwrap().manifest.files[0].sha256,
        m.files[0].sha256
    );
    assert_eq!(crate::verification::summary(&db, "1").unwrap().verified, 2);
}

#[test]
fn changed_source_between_selection_and_copy_never_publishes_partial_package() {
    let d = tempfile::tempdir().unwrap();
    let m = manifest();
    let paths = sources(d.path());
    let out = d.path().join("changed.g4");
    let result = create_observed(&out, &m, &paths, &mut |path| {
        if path == paths[2][0] {
            fs::write(path, b"Changed package fixture\n").unwrap();
        }
    });
    assert!(matches!(
        result.unwrap_err().code,
        ErrorCode::PackageIntegrityFailed
    ));
    assert!(!out.exists());
}

#[test]
fn multi_chunk_binary_payload_roundtrips_with_a_bounded_stream_buffer() {
    let d = tempfile::tempdir().unwrap();
    let bytes = vec![0x5au8; 2 * 1024 * 1024 + 17];
    let source = d.path().join("large.bin");
    fs::write(&source, &bytes).unwrap();
    let mut m = manifest();
    m.files = vec![crate::manifests::ManifestFile {
        path: "large.bin".into(),
        size_bytes: bytes.len().to_string(),
        sha256: format!("{:x}", Sha256::digest(&bytes)),
    }];
    let out = d.path().join("large.g4");
    create(&out, &m, &[Arc::new(vec![source])]).unwrap();
    assert_eq!(read(&out).unwrap().total_bytes, bytes.len() as u64);
}

#[test]
fn relative_package_paths_are_readable_without_changing_process_directory() {
    let fixture = Path::new("../../../packages/g4-format/fixtures/v1-minimal.g4");
    assert_eq!(read(fixture).unwrap().total_bytes, 40);
}

#[test]
fn commented_archives_are_rejected_to_keep_footer_preflight_unambiguous() {
    let mut bytes =
        include_bytes!("../../../../../packages/g4-format/fixtures/v1-minimal.g4").to_vec();
    let end = bytes.len() - 22;
    bytes[end + 20..end + 22].copy_from_slice(&4u16.to_le_bytes());
    bytes.extend_from_slice(b"note");
    assert!(read_bytes(&bytes).is_err());
}
