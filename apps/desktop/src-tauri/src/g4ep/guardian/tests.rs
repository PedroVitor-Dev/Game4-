use super::*;
use std::io::Cursor;
fn ctx() -> OperationContext {
    OperationContext {
        cancellation: tokio_util::sync::CancellationToken::new(),
        deadline: Instant::now() + Duration::from_secs(30),
    }
}
fn fixture() -> (
    tempfile::TempDir,
    super::super::manifest::G4epEverManifest,
    Vec<u8>,
    FragmentDescriptor,
) {
    let d = tempfile::tempdir().unwrap();
    let input = d.path().join("source.bin");
    let bytes = vec![7; 65536];
    fs::write(&input, &bytes).unwrap();
    let m = local::encode_file(
        &input,
        &d.path().join("encoded"),
        CodingProfile::new(8, 4, 65536).unwrap(),
        &ctx(),
        &mut |_| {},
    )
    .unwrap();
    let doc = super::super::manifest::create(&m, "authorized.bin", vec![]).unwrap();
    (d, doc, bytes, m.manifest().fragments[0].clone())
}
fn enable(n: &GuardianNode, quota: u64, up: u32, down: u32) {
    n.configure(GuardianConfig {
        active: true,
        quota_bytes: quota.to_string(),
        upload_bps: up,
        download_bps: down,
    })
    .unwrap()
}
#[test]
fn opt_in_permissions_integrity_atomic_write_and_restart() {
    let (d, doc, bytes, f) = fixture();
    let root = d.path().join("node");
    let n = GuardianNode::open(&root).unwrap();
    assert!(matches!(
        n.commit_fragment(&f, &mut Cursor::new(&bytes), &ctx()),
        Err(Error::GuardianDisabled)
    ));
    enable(&n, 65536, 0, 0);
    assert!(matches!(
        n.commit_fragment(&f, &mut Cursor::new(&bytes), &ctx()),
        Err(Error::GuardianNotApproved)
    ));
    n.approve(&doc, 0, 0, false).unwrap();
    for bad in [vec![0; 65536], vec![7; 65535], vec![7; 65537]] {
        assert!(
            n.commit_fragment(&f, &mut Cursor::new(bad), &ctx())
                .is_err()
        );
        assert_eq!(n.snapshot(0).unwrap().used_bytes, "0");
        assert_eq!(fs::read_dir(n.root.join("staging")).unwrap().count(), 0);
    }
    n.commit_fragment(&f, &mut Cursor::new(&bytes), &ctx())
        .unwrap();
    assert!(matches!(
        n.read_fragment(&f, &ctx()),
        Err(Error::GuardianNotApproved)
    ));
    n.approve(&doc, 0, 0, true).unwrap();
    let mut output = Vec::new();
    n.read_fragment(&f, &ctx())
        .unwrap()
        .read_to_end(&mut output)
        .unwrap();
    assert_eq!(output, bytes);
    // Already stored does not bypass checking newly supplied bytes.
    assert!(
        n.commit_fragment(&f, &mut Cursor::new(vec![0; 65536]), &ctx())
            .is_err()
    );
    n.commit_fragment(&f, &mut Cursor::new(&bytes), &ctx())
        .unwrap();
    assert_eq!(n.snapshot(0).unwrap().fragment_count, 1);
    let node = n.node_id();
    drop(n);
    let n = GuardianNode::open(&root).unwrap();
    assert_eq!(n.node_id(), node);
    assert!(!n.snapshot(0).unwrap().config.active);
    assert_eq!(n.snapshot(0).unwrap().used_bytes, "65536");
    assert_eq!(n.snapshot(0).unwrap().health, "unchecked");
    n.verify_all(&ctx()).unwrap();
    assert_eq!(n.snapshot(0).unwrap().entries[0].health, "verified");
}
#[test]
fn quotas_deduplication_and_explicit_deletion() {
    let (d, doc, bytes, f) = fixture();
    let n = GuardianNode::open(&d.path().join("node")).unwrap();
    n.approve(&doc, 0, 0, true).unwrap();
    enable(&n, 65535, 0, 0);
    assert!(matches!(
        n.commit_fragment(&f, &mut Cursor::new(&bytes), &ctx()),
        Err(Error::QuotaExceeded)
    ));
    enable(&n, 65536, 0, 0);
    n.commit_fragment(&f, &mut Cursor::new(&bytes), &ctx())
        .unwrap();
    assert!(
        n.configure(GuardianConfig {
            active: true,
            quota_bytes: "1".into(),
            upload_bps: 0,
            download_bps: 0
        })
        .is_err()
    );
    let id = n.snapshot(0).unwrap().entries[0].id;
    n.revoke(id, false).unwrap();
    assert!(matches!(
        n.read_fragment(&f, &ctx()),
        Err(Error::GuardianNotApproved)
    ));
    assert_eq!(n.snapshot(0).unwrap().used_bytes, "65536");
    n.approve(&doc, 0, 0, true).unwrap();
    let id = n.snapshot(0).unwrap().entries[0].id;
    n.revoke(id, true).unwrap();
    assert_eq!(n.snapshot(0).unwrap().used_bytes, "0");
}
#[test]
fn corrupted_missing_and_orphan_bytes_are_never_silently_trusted() {
    let (d, doc, bytes, f) = fixture();
    let root = d.path().join("node");
    let n = GuardianNode::open(&root).unwrap();
    n.approve(&doc, 0, 0, true).unwrap();
    enable(&n, 131072, 0, 0);
    n.commit_fragment(&f, &mut Cursor::new(&bytes), &ctx())
        .unwrap();
    fs::write(n.blob(f.sha256), vec![0; 65536]).unwrap();
    assert!(matches!(
        n.read_fragment(&f, &ctx()),
        Err(Error::IntegrityMismatch)
    ));
    assert_eq!(n.snapshot(0).unwrap().health, "attention");
    fs::remove_file(n.blob(f.sha256)).unwrap();
    n.verify_all(&ctx()).unwrap();
    assert_eq!(n.snapshot(0).unwrap().entries[0].health, "missing");
    // Crash between atomic blob publication and inventory commit: recovered but untrusted.
    fs::write(n.blob(f.sha256), &bytes).unwrap();
    fs::write(n.root.join("staging/interrupted.part"), [1, 2]).unwrap();
    drop(n);
    let n = GuardianNode::open(&root).unwrap();
    assert_eq!(n.snapshot(0).unwrap().used_bytes, "65536");
    assert_eq!(n.snapshot(0).unwrap().entries[0].health, "unknown");
    assert_eq!(fs::read_dir(n.root.join("staging")).unwrap().count(), 0);
}
#[test]
fn locks_protect_the_store_and_live_readers() {
    let (d, doc, bytes, f) = fixture();
    let root = d.path().join("node");
    let n = GuardianNode::open(&root).unwrap();
    assert!(GuardianNode::open(&root).is_err());
    n.approve(&doc, 0, 0, true).unwrap();
    enable(&n, 65536, 0, 0);
    n.commit_fragment(&f, &mut Cursor::new(&bytes), &ctx())
        .unwrap();
    let reader = n.read_fragment(&f, &ctx()).unwrap();
    drop(n);
    assert!(GuardianNode::open(&root).is_err());
    drop(reader);
    assert!(GuardianNode::open(&root).is_ok());
}
#[test]
fn disabled_and_revoked_streams_stop_and_cancelled_writes_leave_no_partial_blob() {
    let (d, doc, bytes, f) = fixture();
    let n = GuardianNode::open(&d.path().join("node")).unwrap();
    n.approve(&doc, 0, 0, true).unwrap();
    enable(&n, 65536, 0, 0);
    let c = ctx();
    c.cancellation.cancel();
    assert!(matches!(
        n.commit_fragment(&f, &mut Cursor::new(&bytes), &c),
        Err(Error::Cancelled)
    ));
    n.commit_fragment(&f, &mut Cursor::new(&bytes), &ctx())
        .unwrap();
    let mut reader = n.read_fragment(&f, &ctx()).unwrap();
    n.configure(GuardianConfig::default()).unwrap();
    assert!(reader.read(&mut [0; 1]).is_err());
    drop(reader);
    enable(&n, 65536, 0, 0);
    let mut reader = n.read_fragment(&f, &ctx()).unwrap();
    let id = n.snapshot(0).unwrap().entries[0].id;
    n.revoke(id, false).unwrap();
    assert!(reader.read(&mut [0; 1]).is_err());
}
#[test]
fn shared_upload_and_download_limits_are_enforced() {
    let (d, doc, bytes, f) = fixture();
    let n = GuardianNode::open(&d.path().join("node")).unwrap();
    n.approve(&doc, 0, 0, true).unwrap();
    enable(&n, 65536, 131072, 131072);
    let start = Instant::now();
    n.commit_fragment(&f, &mut Cursor::new(&bytes), &ctx())
        .unwrap();
    assert!(start.elapsed() >= Duration::from_millis(450));
    let start = Instant::now();
    let mut output = Vec::new();
    n.read_fragment(&f, &ctx())
        .unwrap()
        .read_to_end(&mut output)
        .unwrap();
    assert!(start.elapsed() >= Duration::from_millis(450));
    assert_eq!(output, bytes);
}
#[test]
fn concurrent_writes_cannot_overcommit_quota() {
    let (d, doc, bytes, f) = fixture();
    let n = Arc::new(GuardianNode::open(&d.path().join("node")).unwrap());
    n.approve(&doc, 0, 0, true).unwrap();
    n.approve(&doc, 0, 1, true).unwrap();
    enable(&n, 65536, 0, 0);
    let g = super::super::manifest::validate(&doc)
        .unwrap()
        .manifest()
        .fragments[1]
        .clone();
    let tasks = [(f, bytes), (g, vec![0; 65536])].map(|(f, b)| {
        let n = Arc::clone(&n);
        std::thread::spawn(move || n.commit_fragment(&f, &mut Cursor::new(b), &ctx()).is_ok())
    });
    let successes = tasks
        .into_iter()
        .map(|t| u32::from(t.join().unwrap()))
        .sum::<u32>();
    assert_eq!(successes, 1);
    assert_eq!(n.snapshot(0).unwrap().used_bytes, "65536");
}
#[test]
fn unowned_and_relative_directories_are_rejected() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("Game4-Guardian-v1");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("unrelated.txt"), "keep").unwrap();
    assert!(GuardianNode::open(d.path()).is_err());
    assert_eq!(
        fs::read_to_string(root.join("unrelated.txt")).unwrap(),
        "keep"
    );
    assert!(GuardianNode::open(Path::new("relative")).is_err());
}

#[test]
fn disabling_interrupts_a_throttled_write_and_removes_its_stage() {
    let (d, doc, bytes, f) = fixture();
    let n = Arc::new(GuardianNode::open(&d.path().join("node")).unwrap());
    n.approve(&doc, 0, 0, true).unwrap();
    enable(&n, 65536, 0, 1);
    let worker = {
        let n = Arc::clone(&n);
        std::thread::spawn(move || n.commit_fragment(&f, &mut Cursor::new(bytes), &ctx()))
    };
    let start = Instant::now();
    while fs::read_dir(n.root.join("staging")).unwrap().count() == 0
        && start.elapsed() < Duration::from_secs(2)
    {
        std::thread::sleep(Duration::from_millis(5));
    }
    n.configure(GuardianConfig::default()).unwrap();
    assert!(matches!(
        worker.join().unwrap(),
        Err(Error::GuardianDisabled)
    ));
    assert!(start.elapsed() < Duration::from_secs(3));
    assert_eq!(n.snapshot(0).unwrap().used_bytes, "0");
    assert_eq!(fs::read_dir(n.root.join("staging")).unwrap().count(), 0);
}
#[test]
fn prune_only_removes_unapproved_bytes() {
    let (d, doc, bytes, f) = fixture();
    let n = GuardianNode::open(&d.path().join("node")).unwrap();
    n.approve(&doc, 0, 0, true).unwrap();
    enable(&n, 65536, 0, 0);
    n.commit_fragment(&f, &mut Cursor::new(bytes), &ctx())
        .unwrap();
    n.prune().unwrap();
    assert_eq!(n.snapshot(0).unwrap().used_bytes, "65536");
    let id = n.snapshot(0).unwrap().entries[0].id;
    n.revoke(id, false).unwrap();
    n.prune().unwrap();
    assert_eq!(n.snapshot(0).unwrap().used_bytes, "0");
}
