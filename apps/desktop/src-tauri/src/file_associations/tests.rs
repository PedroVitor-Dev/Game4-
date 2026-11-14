use super::*;
use std::fs;
#[test]
fn launch_arguments_preserve_unicode_spaces_and_separate_extensions() {
    let d = tempfile::tempdir().unwrap();
    let inbox = FileInbox::default();
    inbox.enqueue(
        [
            "Coleção β Game.G4",
            "manifesto.g4e",
            "ignored.exe",
            "--flag",
            "https://example.test/file.g4",
        ]
        .into_iter()
        .map(OsString::from),
        d.path(),
    );
    let first = inbox.next().unwrap();
    assert_eq!(first.file_name, "Coleção β Game.G4");
    assert_eq!(first.kind, AssociatedFileKind::G4);
    assert!(inbox.read(&Uuid::new_v4().to_string()).is_err());
    assert_eq!(inbox.next().unwrap().token, first.token);
    assert!(inbox.read(&first.token).is_err());
    assert!(inbox.read(&first.token).is_err());
    assert_eq!(inbox.next().unwrap().kind, AssociatedFileKind::G4e);
    assert_eq!(inbox.pending.lock().unwrap().len(), 1);
}
#[test]
fn inbox_is_bounded_and_duplicate_paths_do_not_duplicate_pending_work() {
    let d = tempfile::tempdir().unwrap();
    let inbox = FileInbox::default();
    inbox.enqueue(
        ["same.g4", "same.g4"].into_iter().map(OsString::from),
        d.path(),
    );
    assert_eq!(inbox.pending.lock().unwrap().len(), 1);
    inbox.enqueue(
        (0..50).map(|i| OsString::from(format!("file-{i}.g4e"))),
        d.path(),
    );
    assert_eq!(inbox.pending.lock().unwrap().len(), 16);
    let empty = FileInbox::default();
    empty.enqueue([OsString::from("relative.g4")], Path::new("not-absolute"));
    assert!(empty.next().is_none());
}
#[test]
fn inspected_associations_match_picker_results_without_modifying_or_extracting() {
    let d = tempfile::tempdir().unwrap();
    let g4 = d.path().join("Preservação ∞.g4");
    let g4e = d.path().join("Original files.g4e");
    let raw = include_bytes!("../../../../../packages/g4-format/fixtures/v1-minimal.g4");
    let manifest = include_bytes!("../../../../../packages/g4e-format/examples/original-files.g4e");
    fs::write(&g4, raw).unwrap();
    fs::write(&g4e, manifest).unwrap();
    let inbox = FileInbox::default();
    inbox.enqueue(
        [g4.clone().into_os_string(), g4e.clone().into_os_string()],
        d.path(),
    );
    let token = inbox.next().unwrap().token;
    let package = inbox.read(&token).unwrap();
    assert!(package.manifest.is_none());
    assert_eq!(package.kind, AssociatedFileKind::G4);
    assert_eq!(
        serde_json::to_value(package.package.unwrap()).unwrap(),
        serde_json::to_value(inspect_package(&g4).unwrap()).unwrap()
    );
    let result = inbox.read(&inbox.next().unwrap().token).unwrap();
    assert!(result.package.is_none());
    let inspected = result.manifest.unwrap();
    assert!(!inspected.content_verified);
    assert!(!inspected.magnets.is_empty());
    assert!(inbox.next().is_none());
    assert_eq!(fs::read(&g4).unwrap(), raw);
    assert_eq!(fs::read(&g4e).unwrap(), manifest);
    assert_eq!(fs::read_dir(d.path()).unwrap().count(), 2);
}
#[test]
fn corrupt_payload_is_rejected_and_does_not_block_the_next_association() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("broken.g4");
    fs::write(&path, b"not a ZIP package").unwrap();
    let inbox = FileInbox::default();
    inbox.enqueue(
        [path.into_os_string(), OsString::from("missing.g4e")],
        d.path(),
    );
    assert!(inbox.read(&inbox.next().unwrap().token).is_err());
    assert_eq!(inbox.next().unwrap().file_name, "missing.g4e");
}
#[test]
fn bundle_registers_only_supported_preservation_extensions() {
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
    let associations = config["bundle"]["fileAssociations"].as_array().unwrap();
    assert_eq!(associations.len(), 2);
    assert_eq!(associations[0]["ext"], serde_json::json!(["g4"]));
    assert_eq!(associations[1]["ext"], serde_json::json!(["g4e"]));
    for entry in associations {
        assert_eq!(entry["role"], "Viewer");
    }
}
