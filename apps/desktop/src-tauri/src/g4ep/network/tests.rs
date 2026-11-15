use super::super::{guardian::GuardianConfig, manifest};
use super::*;
use std::io::Read;
fn ctx() -> OperationContext {
    OperationContext {
        cancellation: CancellationToken::new(),
        deadline: Instant::now() + Duration::from_secs(15),
    }
}
fn ports() -> (String, String) {
    let a = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let b = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    (
        a.local_addr().unwrap().to_string(),
        b.local_addr().unwrap().to_string(),
    )
}
struct Pair {
    _dir: tempfile::TempDir,
    a: Arc<ManualTransport>,
    b: Arc<ManualTransport>,
    f: FragmentDescriptor,
    bytes: Vec<u8>,
}
fn pair() -> Pair {
    let d = tempfile::tempdir().unwrap();
    let bytes = vec![9; 262144];
    let input = d.path().join("input.bin");
    fs::write(&input, &bytes).unwrap();
    let m = local::encode_file(
        &input,
        &d.path().join("encoded"),
        CodingProfile::new(8, 4, 262144).unwrap(),
        &ctx(),
        &mut |_| {},
    )
    .unwrap();
    let doc = manifest::create(&m, "authorized.bin", vec![]).unwrap();
    let f = m.manifest().fragments[0].clone();
    let a = Arc::new(GuardianNode::open(&d.path().join("a")).unwrap());
    let b = Arc::new(GuardianNode::open(&d.path().join("b")).unwrap());
    for n in [&a, &b] {
        n.approve(&doc, 0, 0, true).unwrap();
        n.configure(GuardianConfig {
            active: true,
            quota_bytes: "524288".into(),
            upload_bps: 0,
            download_bps: 0,
        })
        .unwrap();
    }
    let ia = Identity::load(a.node_id(), &d.path().join("keys-a")).unwrap();
    let ib = Identity::load(b.node_id(), &d.path().join("keys-b")).unwrap();
    let (aa, bb) = ports();
    let pa = ia.profile(&aa).unwrap();
    let pb = ib.profile(&bb).unwrap();
    let ca = ManualConfig {
        bind: aa,
        peers: vec![pb],
        timeout_seconds: 3,
        ..Default::default()
    };
    let cb = ManualConfig {
        bind: bb,
        peers: vec![pa],
        timeout_seconds: 3,
        ..Default::default()
    };
    Pair {
        _dir: d,
        a: ManualTransport::new(a, ia, ca).unwrap(),
        b: ManualTransport::new(b, ib, cb).unwrap(),
        f,
        bytes,
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mutual_tls_discovery_advertisement_upload_and_download_are_real() {
    let p = pair();
    let a = p.a.start().await.unwrap();
    let b = p.b.start().await.unwrap();
    let peer = p.a.peers()[0];
    assert_eq!(p.a.probe(&peer, &ctx()).await.unwrap(), 1);
    assert_eq!(p.a.discover(&p.a.network, &ctx()).await.unwrap().len(), 1);
    assert_eq!(
        p.a.availability(&peer, std::slice::from_ref(&p.f), &ctx())
            .await
            .unwrap()
            .0,
        vec![false]
    );
    p.a.node
        .commit_fragment(&p.f, &mut std::io::Cursor::new(&p.bytes), &ctx())
        .unwrap();
    p.a.push(&peer, &p.f, &ctx()).await.unwrap();
    assert_eq!(
        p.a.availability(&peer, std::slice::from_ref(&p.f), &ctx())
            .await
            .unwrap()
            .0,
        vec![true]
    );
    let id = p.a.node.snapshot(0).unwrap().entries[0].id;
    p.a.node.revoke(id, true).unwrap();
    // Re-admit the exact known map without retaining the complete source at either node.
    let m = local::read_manifest(&p._dir.path().join("encoded"), &ctx()).unwrap();
    let doc = manifest::create(&m, "authorized.bin", vec![]).unwrap();
    p.a.node.approve(&doc, 0, 0, true).unwrap();
    p.a.fetch(&peer, &p.f, &ctx()).await.unwrap();
    let mut actual = Vec::new();
    p.a.node
        .read_fragment(&p.f, &ctx())
        .unwrap()
        .read_to_end(&mut actual)
        .unwrap();
    assert_eq!(actual, p.bytes);
    a.stopped().await;
    b.stopped().await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn wrong_peer_certificate_and_public_routes_fail_closed() {
    let p = pair();
    let b = p.b.start().await.unwrap();
    let mut bad = p.a.config.clone();
    bad.peers[0].certificate_der = hex(&p.a.identity.certificate);
    bad.peers[0].certificate_sha256 = p.a.identity.fingerprint();
    let wrong =
        ManualTransport::new(Arc::clone(&p.a.node), Arc::clone(&p.a.identity), bad).unwrap();
    assert!(matches!(
        wrong.probe(&wrong.peers()[0], &ctx()).await,
        Err(Error::PeerAuthentication)
    ));
    for address in [
        "0.0.0.0:42430",
        "8.8.8.8:443",
        "example.com:42430",
        "127.0.0.1:0",
    ] {
        assert!(private_address(address).is_err());
    }
    let mut bad = p.a.config.clone();
    bad.peers[0].node_id = p.a.config.peers[0].node_id.clone();
    bad.peers.push(bad.peers[0].clone());
    assert!(bad.validate(p.a.node.node_id()).is_err());
    b.stopped().await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn disabled_unapproved_and_quota_rejections_leave_no_partial_bytes() {
    let p = pair();
    let b = p.b.start().await.unwrap();
    p.a.node
        .commit_fragment(&p.f, &mut std::io::Cursor::new(&p.bytes), &ctx())
        .unwrap();
    p.b.node
        .configure(GuardianConfig {
            active: true,
            quota_bytes: "1".into(),
            upload_bps: 0,
            download_bps: 0,
        })
        .unwrap();
    assert!(matches!(
        p.a.push(&p.a.peers()[0], &p.f, &ctx()).await,
        Err(Error::QuotaExceeded)
    ));
    let id = p.b.node.snapshot(0).unwrap().entries[0].id;
    p.b.node.revoke(id, false).unwrap();
    assert!(matches!(
        p.a.push(&p.a.peers()[0], &p.f, &ctx()).await,
        Err(Error::GuardianNotApproved)
    ));
    assert_eq!(p.b.node.snapshot(0).unwrap().used_bytes, "0");
    assert_eq!(
        fs::read_dir(p.b.node.storage_root().join("staging"))
            .unwrap()
            .count(),
        0
    );
    b.stopped().await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn connection_retry_recovers_after_an_initial_refusal() {
    let p = pair();
    let server = tokio::sync::Mutex::new(None::<RunningNode>);
    let first = std::sync::atomic::AtomicBool::new(true);
    let transport = &p.a;
    let service = &p.b;
    let peer = &p.a.peers()[0];
    let server_ref = &server;
    let first_ref = &first;
    let (_, attempts) =
        p.a.retry(&ctx(), |attempt| async move {
            if first_ref.swap(false, std::sync::atomic::Ordering::SeqCst) {
                let error = transport
                    .connect(peer, &attempt)
                    .await
                    .expect_err("server is not listening yet");
                *server_ref.lock().await = Some(service.start().await?);
                return Err(error);
            }
            let mut stream = transport.connect(peer, &attempt).await?;
            send(
                &mut stream,
                &Request {
                    version: 1,
                    node_id: hex(&transport.identity.node.0),
                    operation: Operation::Probe,
                },
            )
            .await?;
            let response = receive(&mut stream).await?;
            transport.check_response(&response, peer)
        })
        .await
        .unwrap();
    assert_eq!(attempts, 2);
    server.into_inner().unwrap().stopped().await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn slots_timeouts_and_cancellation_bound_slow_connections() {
    let mut p = pair();
    let mut cfg = p.b.config.clone();
    cfg.max_connections = 1;
    p.b = ManualTransport::new(Arc::clone(&p.b.node), Arc::clone(&p.b.identity), cfg).unwrap();
    let server = p.b.start().await.unwrap();
    let socket = TcpStream::connect(p.b.network.bind()).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(p.b.connections(), 1);
    assert!(p.a.probe(&p.a.peers()[0], &ctx()).await.is_err());
    assert_eq!(p.b.connections(), 1);
    drop(socket);
    let cancel = ctx();
    cancel.cancellation.cancel();
    assert!(matches!(
        p.a.probe(&p.a.peers()[0], &cancel).await,
        Err(Error::Cancelled)
    ));
    let c = ctx();
    let start = Instant::now();
    assert!(matches!(
        bounded(&c, 2, async { std::future::pending::<Result<()>>().await }).await,
        Err(Error::NetworkTimeout)
    ));
    assert!(start.elapsed() < Duration::from_secs(3));
    server.stopped().await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn network_limits_and_stop_interrupt_a_live_fragment() {
    let p = pair();
    let server = p.b.start().await.unwrap();
    p.a.node
        .commit_fragment(&p.f, &mut std::io::Cursor::new(&p.bytes), &ctx())
        .unwrap();
    p.a.node
        .configure(GuardianConfig {
            active: true,
            quota_bytes: "524288".into(),
            upload_bps: 262144,
            download_bps: 0,
        })
        .unwrap();
    let start = Instant::now();
    p.a.push(&p.a.peers()[0], &p.f, &ctx()).await.unwrap();
    assert!(start.elapsed() >= Duration::from_millis(900));
    p.a.node
        .configure(GuardianConfig {
            active: true,
            quota_bytes: "524288".into(),
            upload_bps: 1,
            download_bps: 0,
        })
        .unwrap();
    let client = Arc::clone(&p.a);
    let f = p.f.clone();
    let task = tokio::spawn(async move { client.push(&client.peers()[0], &f, &ctx()).await });
    tokio::time::sleep(Duration::from_millis(100)).await;
    p.a.cancel.cancel();
    assert!(matches!(task.await.unwrap(), Err(Error::Cancelled)));
    server.stopped().await;
}
#[test]
fn identity_profile_and_settings_roundtrip_expose_no_private_key() {
    let p = pair();
    let profile = p.a.identity.profile(&p.a.config.bind).unwrap();
    let public = serde_json::to_string(&profile).unwrap();
    assert!(!public.contains("privateKey"));
    let file = p._dir.path().join("profile.json");
    write_profile(&file, &profile).unwrap();
    assert!(write_profile(&file, &profile).is_err());
    assert_eq!(read_profile(&file).unwrap().node_id, profile.node_id);
    save_config(&p.a.node, &p.a.config).unwrap();
    assert_eq!(read_config(&p.a.node).unwrap().peers.len(), 1);
    let again = Identity::load(p.a.node.node_id(), &p._dir.path().join("keys-a")).unwrap();
    assert_eq!(again.fingerprint(), p.a.identity.fingerprint());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn authenticated_peer_cannot_publish_corrupt_or_truncated_bytes() {
    for truncate in [false, true] {
        let p = pair();
        let listener = TcpListener::bind(p.b.network.bind()).await.unwrap();
        let acceptor = TlsAcceptor::from(Arc::new(p.b.tls_server().unwrap()));
        let b = Arc::clone(&p.b);
        let size = p.f.bytes as usize;
        let fake = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut stream = acceptor.accept(socket).await.unwrap();
            let _: Request = receive(&mut stream).await.unwrap();
            send(&mut stream, &b.response("ok", vec![])).await.unwrap();
            let bytes = vec![0; if truncate { size / 2 } else { size }];
            stream.write_all(&bytes).await.unwrap();
            stream.flush().await.unwrap();
        });
        assert!(p.a.fetch(&p.a.peers()[0], &p.f, &ctx()).await.is_err());
        fake.await.unwrap();
        assert_eq!(p.a.node.snapshot(0).unwrap().used_bytes, "0");
        assert_eq!(
            fs::read_dir(p.a.node.storage_root().join("staging"))
                .unwrap()
                .count(),
            0
        );
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn transfer_ports_verify_before_output_and_reuse_directional_limits() {
    let p = pair();
    let server = p.b.start().await.unwrap();
    let peer = p.a.peers()[0];
    let mut source = std::io::Cursor::new(p.bytes.clone());
    p.a.upload(&p.a.network, &peer, &p.f, &mut source, &ctx())
        .await
        .unwrap();
    let mut output = std::io::Cursor::new(Vec::new());
    p.a.download(&p.a.network, &peer, &p.f, &mut output, &ctx())
        .await
        .unwrap();
    assert_eq!(output.into_inner(), p.bytes);
    let mut source = std::io::Cursor::new(vec![0; p.f.bytes as usize]);
    assert!(matches!(
        p.a.upload(&p.a.network, &peer, &p.f, &mut source, &ctx())
            .await,
        Err(Error::IntegrityMismatch)
    ));
    server.stopped().await;
}
