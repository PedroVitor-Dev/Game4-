use super::{manifest::*, types::*};
use crate::{contracts::ErrorCode, ever};
use ed25519_dalek::SigningKey;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;
fn example(signed: bool) -> G4epEverManifest {
    parse(if signed {
        include_bytes!("../../../../../packages/g4e-format/examples/g4ep-fragments-v2-signed.g4e")
            .as_slice()
    } else {
        include_bytes!("../../../../../packages/g4e-format/examples/g4ep-fragments-v2.g4e")
            .as_slice()
    })
    .unwrap()
}
fn context() -> OperationContext {
    OperationContext {
        cancellation: CancellationToken::new(),
        deadline: Instant::now() + Duration::from_secs(60),
    }
}
#[test]
fn shared_v2_examples_are_canonical_valid_and_bound_to_exact_reconstruction_map() {
    for signed in [false, true] {
        let doc = example(signed);
        let bytes = serialize(&doc).unwrap();
        assert_eq!(serialize(&parse(&bytes).unwrap()).unwrap(), bytes);
        let model = validate(&doc).unwrap();
        assert_eq!(model.manifest().fragments.len(), 12);
        assert_eq!(model.manifest().object.bytes, 524288);
        assert_eq!(
            doc.authentication.payload_sha256,
            "0da7e6664cdffc1d9b9cc000853764edd9197184bdae2a60f8f83375c05cbf0b"
        );
        assert!(matches!(
            ever::parse_document(&bytes).unwrap(),
            ever::EverDocument::G4ep(_)
        ));
        assert!(ever::parse(&bytes).is_err());
    }
    for bytes in [
        include_bytes!("../../../../../packages/g4e-format/examples/original-files.g4e").as_slice(),
        include_bytes!("../../../../../packages/g4e-format/examples/package.g4e").as_slice(),
    ] {
        assert!(matches!(
            ever::parse_document(bytes).unwrap(),
            ever::EverDocument::V1(_)
        ));
    }
}
#[test]
fn shared_v2_invalid_corpus_rejects_resigned_bad_geometry_and_untrusted_references() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../packages/g4e-format/g4ep-invalid-cases.json"
    ))
    .unwrap();
    for row in corpus.as_array().unwrap() {
        assert!(
            ever::parse_document(&serde_json::to_vec(&row["document"]).unwrap()).is_err(),
            "{}",
            row["name"]
        );
    }
}
#[test]
fn signatures_match_node_vectors_and_require_the_expected_origin_key() {
    let signed = example(true);
    let signer = &signed.authentication.signatures[0].key_id;
    require_origin(&signed, signer).unwrap();
    assert!(require_origin(&example(false), signer).is_err());
    assert!(require_origin(&signed, &format!("urn:sha256:{}", "0".repeat(64))).is_err());
    let seed: [u8; 32] = [
        0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec, 0x2c,
        0xc4, 0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03, 0x1c, 0xae,
        0x7f, 0x60,
    ];
    let mut doc = example(false);
    sign(&mut doc, &SigningKey::from_bytes(&seed)).unwrap();
    assert_eq!(serialize(&doc).unwrap(), serialize(&signed).unwrap());
    let mut changed = doc.clone();
    changed.manifest.collection.title = "tampered".into();
    assert!(matches!(
        validate(&changed).unwrap_err().code,
        ErrorCode::G4eIntegrityFailed
    ));
}
#[test]
fn strict_json_and_descriptor_injection_are_rejected_without_any_io() {
    let bytes = serialize(&example(false)).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    for text in [
        text.replace(
            "\"schemaVersion\":2",
            "\"schemaVersion\":2,\"schemaVer\\u0073ion\":2",
        ),
        text.clone() + "{}",
        "\u{feff}".to_string() + &text,
    ] {
        assert!(parse(text.as_bytes()).is_err());
    }
    let mut doc = example(false);
    doc.manifest.files[0].path = "C:/private/rom.iso".into();
    doc.authentication.payload_sha256 = payload_hash(&doc).unwrap();
    assert!(validate(&doc).is_err());
}
#[test]
fn encoded_output_can_export_and_verify_without_original_path_or_payload() {
    let d = tempfile::tempdir().unwrap();
    let input = d.path().join("private-original.bin");
    let output = d.path().join("encoded");
    let source = vec![7u8; 700001];
    std::fs::write(&input, &source).unwrap();
    let profile = CodingProfile::new(8, 4, 65536).unwrap();
    let m = super::local::encode_file(&input, &output, profile, &context(), &mut |_| {}).unwrap();
    let doc = create(&m, "preserved.bin", vec![]).unwrap();
    assert_eq!(doc.g4ep.groups.len(), 2);
    let path = d.path().join("preserved.g4e");
    write(&path, &doc).unwrap();
    assert!(write(&path, &doc).is_err());
    let read = read(&path).unwrap();
    let model = validate(&read).unwrap();
    for f in &model.manifest().fragments {
        super::local::verify_fragment(&output, f, &context()).unwrap();
    }
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(!text.contains("private-original"));
    assert!(!text.contains(&d.path().to_string_lossy().to_string()));
    assert!(!text.contains("romBytes"));
    assert_eq!(std::fs::read(&input).unwrap(), source);
    let summary = crate::file_associations::inspect_ever(&path).unwrap();
    assert_eq!(summary.schema_version, 2);
    assert!(summary.magnets.is_empty());
    assert!(!summary.content_verified);
    assert!(!summary.g4ep.unwrap().publisher_trusted);
}
#[test]
fn canonicalization_survives_object_key_reordering_but_not_group_reordering() {
    let doc = example(true);
    let mut v: serde_json::Value = serde_json::from_slice(&serialize(&doc).unwrap()).unwrap();
    v["authentication"]["payloadSha256"] = serde_json::json!("0".repeat(64));
    assert!(parse(&serde_json::to_vec(&v).unwrap()).is_err());
    let valid = example(false);
    let m = validate(&valid).unwrap();
    let mut m = m.manifest().clone();
    m.object.bytes = 524289;
    for f in m.fragments.clone() {
        let mut next = f;
        next.coordinate.stripe = 1;
        m.fragments.push(next);
    }
    let fresh = create(
        &ValidatedManifest::new(m).unwrap(),
        "two-groups.bin",
        vec![],
    )
    .unwrap();
    let mut changed = fresh.clone();
    changed.g4ep.groups.reverse();
    changed.authentication.payload_sha256 = payload_hash(&changed).unwrap();
    assert!(validate(&changed).is_err());
}
