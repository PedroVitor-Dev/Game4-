use super::*;
use librqbit::storage::TorrentStorage;
use sha2::{Digest, Sha256};
use std::fs;
fn setup() -> (
    tempfile::TempDir,
    Arc<Database>,
    Arc<ImportState>,
    PathBuf,
    Vec<u8>,
) {
    let d = tempfile::tempdir().unwrap();
    let db = Arc::new(Database::open(&d.path().join("catalog.sqlite3")).unwrap());
    let imports = Arc::new(ImportState::default());
    let root = d.path().join("originals");
    fs::create_dir_all(root.join("nested")).unwrap();
    let bytes = vec![0x5au8; 2 * 1024 * 1024 + 17];
    fs::write(root.join("a.bin"), &bytes[..1024 * 1024 - 3]).unwrap();
    fs::write(root.join("nested/β.bin"), &bytes[1024 * 1024 - 3..]).unwrap();
    fs::write(root.join("z-empty.bin"), b"").unwrap();
    let req = StartImportRequest {
        request_id: Uuid::new_v4().to_string(),
        selection_token: Uuid::new_v4().to_string(),
    };
    let task = library::create_task(&db, &req).unwrap();
    library::run_import(&db, &task.task_id, vec![root.clone()]).unwrap();
    (d, db, imports, root, bytes)
}
fn manager() -> Arc<Transfers> {
    Arc::new(Transfers {
        listen_ip: Ipv4Addr::LOCALHOST,
        ..Default::default()
    })
}
fn seed_request() -> StartSeedRequest {
    StartSeedRequest {
        request_id: Uuid::new_v4().to_string(),
        item_id: "1".into(),
        listen_port: 0,
        dht: false,
        trackers: vec![],
    }
}
#[test]
fn imported_torrent_is_offline_bounded_and_source_storage_rejects_all_writes() {
    let (d, db, _, root, _) = setup();
    let created = ever::from_catalog(&db, "1").unwrap();
    let m = manager();
    let preview = m.import(created.torrent.clone()).unwrap();
    assert_eq!(preview.files.len(), 3);
    assert!(m.list().items.is_empty());
    let meta = inspect(&created.torrent).unwrap();
    let handles = Handles::seed(&root, &meta).unwrap();
    assert!(handles.pwrite_all(0, 0, b"bad").is_err());
    assert!(handles.ensure_file_length(0, 1).is_err());
    assert!(handles.remove_file(0, Path::new("a.bin")).is_err());
    assert!(handles.remove_directory_if_empty(&root).is_err());
    assert_eq!(fs::read(root.join("a.bin")).unwrap()[0], 0x5a);
    assert!(m.import(b"not a torrent".to_vec()).is_err());
    let path = d.path().join("export.torrent");
    write_new(&path, &created.torrent).unwrap();
    assert!(write_new(&path, &created.torrent).is_err());
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn real_loopback_transfer_preserves_sources_and_completes_with_matching_sha256() {
    let (d, db, imports, root, original) = setup();
    let seed = manager();
    let download = manager();
    let request = seed_request();
    let seed_snapshot = seed
        .seed(Arc::clone(&db), Arc::clone(&imports), request.clone())
        .await
        .unwrap();
    assert_eq!(seed_snapshot.mode, TransferMode::Seed);
    assert_eq!(seed_snapshot.verified_bytes, seed_snapshot.total_bytes);
    let replay = seed
        .seed(Arc::clone(&db), Arc::clone(&imports), request.clone())
        .await
        .unwrap();
    assert_eq!(replay.id, seed_snapshot.id);
    let created = ever::from_catalog(&db, "1").unwrap();
    let preview = download.import(created.torrent).unwrap();
    let parent = d.path().join("received");
    fs::create_dir(&parent).unwrap();
    let request = StartDownloadRequest {
        request_id: Uuid::new_v4().to_string(),
        token: preview.token,
        dht: false,
        trackers: vec![],
        peer: format!("127.0.0.1:{}", seed_snapshot.listen_port.unwrap()),
    };
    let started = download
        .download(request.clone(), parent.clone())
        .await
        .unwrap();
    let again = download.download(request, parent.clone()).await.unwrap();
    assert_eq!(again.id, started.id);
    let complete = tokio::time::timeout(Duration::from_secs(35), async {
        loop {
            let item = download
                .list()
                .items
                .into_iter()
                .find(|v| v.id == started.id)
                .unwrap();
            if item.status == TransferStatus::Completed {
                return item;
            }
            assert_ne!(item.status, TransferStatus::Failed, "download failed");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(complete.verified_bytes, original.len().to_string());
    let output = parent.join(complete.output_name.unwrap());
    let received = [
        fs::read(output.join("a.bin")).unwrap(),
        fs::read(output.join("nested/β.bin")).unwrap(),
    ]
    .concat();
    assert_eq!(Sha256::digest(&received), Sha256::digest(&original));
    assert!(output.join("z-empty.bin").exists());
    assert_eq!(fs::metadata(output.join("z-empty.bin")).unwrap().len(), 0);
    assert_eq!(fs::read_dir(&parent).unwrap().count(), 1);
    assert_eq!(
        [
            fs::read(root.join("a.bin")).unwrap(),
            fs::read(root.join("nested/β.bin")).unwrap()
        ]
        .concat(),
        original
    );
    seed.stop(&seed_snapshot.id).await.unwrap();
    download.shutdown().await;
    seed.shutdown().await;
    // Stopping releases read-only handles, allowing the owner to edit again.
    fs::OpenOptions::new()
        .write(true)
        .open(root.join("a.bin"))
        .unwrap();
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn invalid_requests_and_cancelled_shutdown_never_start_new_network_work() {
    let (d, db, imports, _, _) = setup();
    let m = manager();
    for addr in [
        "0.0.0.0:4",
        "127.0.0.1:0",
        "example.invalid:4",
        "[::1]:4",
        "255.255.255.255:4",
    ] {
        assert!(peer(addr).is_err());
    }
    let req = StartDownloadRequest {
        request_id: Uuid::new_v4().to_string(),
        token: Uuid::new_v4().to_string(),
        dht: false,
        trackers: vec![],
        peer: "127.0.0.1:4444".into(),
    };
    assert!(m.download(req, d.path().into()).await.is_err());
    assert!(m.list().items.is_empty());
    m.abort();
    assert!(m.seed(db, imports, seed_request()).await.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stopping_partial_download_preserves_files_and_rejects_conflicting_replay() {
    let (d, db, _, _, _) = setup();
    let m = manager();
    let created = ever::from_catalog(&db, "1").unwrap();
    let preview = m.import(created.torrent).unwrap();
    // Hold a local port without accepting BitTorrent, so the transfer remains incomplete.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let request = StartDownloadRequest {
        request_id: Uuid::new_v4().to_string(),
        token: preview.token,
        dht: false,
        trackers: vec![],
        peer: listener.local_addr().unwrap().to_string(),
    };
    let started = m.download(request.clone(), d.path().into()).await.unwrap();
    let mut conflicting = request;
    conflicting.token = Uuid::new_v4().to_string();
    assert!(matches!(
        m.download(conflicting, d.path().into())
            .await
            .unwrap_err()
            .code,
        ErrorCode::InvalidArgument
    ));
    let stopped = m.stop(&started.id).await.unwrap();
    assert_eq!(stopped.status, TransferStatus::Stopped);
    assert!(stopped.listen_port.is_none());
    let root = d.path().join(stopped.output_name.unwrap());
    assert!(root.join("a.bin").exists());
    assert_eq!(fs::metadata(root.join("a.bin")).unwrap().len(), 0);
    fs::OpenOptions::new()
        .write(true)
        .open(root.join("a.bin"))
        .unwrap();
    assert_eq!(
        m.stop(&started.id).await.unwrap().status,
        TransferStatus::Stopped
    );
    m.shutdown().await;
}

#[tokio::test]
async fn expired_preview_fails_before_creating_output() {
    let (d, db, _, _, _) = setup();
    let m = manager();
    let preview = m
        .import(ever::from_catalog(&db, "1").unwrap().torrent)
        .unwrap();
    m.pending
        .lock()
        .unwrap()
        .get_mut(&preview.token)
        .unwrap()
        .created = Instant::now() - Duration::from_secs(301);
    let before = fs::read_dir(d.path()).unwrap().count();
    assert!(
        m.download(
            StartDownloadRequest {
                request_id: Uuid::new_v4().to_string(),
                token: preview.token,
                dht: false,
                trackers: vec![],
                peer: "127.0.0.1:42420".into()
            },
            d.path().into()
        )
        .await
        .is_err()
    );
    assert_eq!(fs::read_dir(d.path()).unwrap().count(), before);
    assert!(m.list().items.is_empty());
}

#[test]
fn magnet_parser_normalizes_hashes_and_rejects_ambiguous_or_remote_sources() {
    let h = "0000000000000000000000000000000000000000";
    assert_eq!(normalize_magnet("magnet:?xt=urn:btih:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA&dn=ignored&tr=https%3A%2F%2Ftracker.invalid").unwrap().info_hash, h);
    assert_eq!(
        normalize_magnet("magnet:?xt=urn:btih:ABCDEF0123456789ABCDEF0123456789ABCDEF01")
            .unwrap()
            .info_hash,
        "abcdef0123456789abcdef0123456789abcdef01"
    );
    for input in [
        "https://example.invalid/file.torrent",
        "magnet:?dn=nohash",
        "magnet:?xt=urn:btmh:1220abcd",
        "magnet:?xt=urn:btih:no",
        "magnet:?xt=urn:btih:0000000000000000000000000000000000000000&xt=urn:btih:0000000000000000000000000000000000000000",
        "magnet:?xt=urn:btih:0000000000000000000000000000000000000000&xs=http://127.0.0.1/",
        "magnet:?xt=urn:btih:0000000000000000000000000000000000000000&x.pe=127.0.0.1:1",
    ] {
        assert!(normalize_magnet(input).is_err(), "accepted {input}");
    }
    assert_eq!(
        normalize_magnet("magnet:?xt=urn:btih:AERUKZ4JVPG66AJDIVTYTK6N54ASGRLH")
            .unwrap()
            .info_hash,
        "0123456789abcdef0123456789abcdef01234567"
    );
    assert!(normalize_magnet(&"x".repeat(16385)).is_err());
    assert!(magnets::initial_peer("", false).is_err());
    assert!(magnets::initial_peer("", true).unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn real_magnet_metadata_resolution_is_preview_only_and_cancellable() {
    let (d, db, imports, _, _) = setup();
    let seed = manager();
    let started = seed
        .seed(db.clone(), imports, seed_request())
        .await
        .unwrap();
    let m = manager();
    let before = fs::read_dir(d.path()).unwrap().count();
    let result = m
        .resolve_magnet(ResolveMagnetRequest {
            request_id: Uuid::new_v4().to_string(),
            magnet: format!("magnet:?xt=urn:btih:{}", started.info_hash),
            peer: format!("127.0.0.1:{}", started.listen_port.unwrap()),
            dht: false,
            trackers: vec![],
        })
        .await
        .unwrap();
    assert!(!result.cancelled);
    let preview = result.preview.unwrap();
    assert_eq!(preview.info_hash, started.info_hash);
    assert_eq!(preview.files.len(), 3);
    assert_eq!(fs::read_dir(d.path()).unwrap().count(), before);
    assert!(m.list().items.is_empty());
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let request_id = Uuid::new_v4().to_string();
    let lookup = tokio::spawn({
        let m = m.clone();
        let request_id = request_id.clone();
        async move {
            m.resolve_magnet(ResolveMagnetRequest {
                request_id,
                magnet: format!("magnet:?xt=urn:btih:{}", started.info_hash),
                peer: listener.local_addr().unwrap().to_string(),
                dht: false,
                trackers: vec![],
            })
            .await
        }
    });
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if m.resolution.lock().unwrap().is_some() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(
        !m.cancel_magnet(CancelMagnetRequest {
            request_id: Uuid::new_v4().to_string()
        })
        .unwrap()
        .cancelled
    );
    assert!(
        m.cancel_magnet(CancelMagnetRequest { request_id })
            .unwrap()
            .cancelled
    );
    assert!(lookup.await.unwrap().unwrap().cancelled);
    assert!(m.resolution.lock().unwrap().is_none());
    m.shutdown().await;
    seed.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn isolated_dht_discovers_seed_and_resolves_magnet_without_an_initial_peer() {
    use librqbit::dht::{DhtBuilder, DhtConfig, Id20};
    let (d, db, imports, _, original) = setup();
    let created = ever::from_catalog(&db, "1").unwrap();
    let hash = inspect(&created.torrent).unwrap().hash;
    // A real isolated UDP DHT node close to this hash stores the real seed announcement.
    let port_a = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let port_b = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let addr_a = port_a.local_addr().unwrap();
    let addr_b = port_b.local_addr().unwrap();
    drop((port_a, port_b));
    let hash_id = hash.parse::<Id20>().unwrap();
    let neighbor_id = Id20::new([0x42; 20]);
    // Public bootstraps already know other nodes. Reproduce that condition
    // with two reciprocal routing entries, without any Internet bootstrap.
    let template_a = DhtBuilder::with_config(DhtConfig {
        peer_id: Some(hash_id),
        bootstrap_addrs: Some(vec![addr_b.to_string()]),
        listen_addr: Some("127.0.0.1:0".parse().unwrap()),
        ..Default::default()
    })
    .await
    .unwrap();
    let table_a = template_a.with_routing_tables(|v4, _| {
        let mut table = v4.clone();
        let _ = table.add_node(neighbor_id, addr_b);
        table
    });
    template_a.cancellation_token().cancel();
    let template_b = DhtBuilder::with_config(DhtConfig {
        peer_id: Some(neighbor_id),
        bootstrap_addrs: Some(vec![addr_a.to_string()]),
        listen_addr: Some("127.0.0.1:0".parse().unwrap()),
        ..Default::default()
    })
    .await
    .unwrap();
    let table_b = template_b.with_routing_tables(|v4, _| {
        let mut table = v4.clone();
        let _ = table.add_node(hash_id, addr_a);
        table
    });
    template_b.cancellation_token().cancel();
    let bootstrap = DhtBuilder::with_config(DhtConfig {
        peer_id: Some(hash_id),
        routing_table: Some(table_a),
        bootstrap_addrs: Some(vec![addr_b.to_string()]),
        listen_addr: Some(addr_a),
        ..Default::default()
    })
    .await
    .unwrap();
    let neighbor = DhtBuilder::with_config(DhtConfig {
        peer_id: Some(neighbor_id),
        routing_table: Some(table_b),
        bootstrap_addrs: Some(vec![addr_a.to_string()]),
        listen_addr: Some(addr_b),
        ..Default::default()
    })
    .await
    .unwrap();
    let bootstrap_addr = bootstrap.listen_addr().to_string();
    let make = || {
        Arc::new(Transfers {
            listen_ip: Ipv4Addr::LOCALHOST,
            dht_bootstrap: Some(vec![bootstrap_addr.clone()]),
            ..Default::default()
        })
    };
    let seed = make();
    let download = make();
    let mut req = seed_request();
    req.dht = true;
    let port_reservation = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    req.listen_port = port_reservation.local_addr().unwrap().port();
    drop(port_reservation);
    let seeded = seed.seed(db, imports, req).await.unwrap();
    assert!(seeded.dht);
    // Observe the actual BEP 5 announcement before starting the receiver.
    // Otherwise its first lookup can precede the seed announcement and the
    // upstream 60-second node requery interval races our metadata deadline.
    let probe = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let mut query = b"d1:ad2:id20:".to_vec();
    query.extend_from_slice(&[0x71; 20]);
    query.extend_from_slice(b"9:info_hash20:");
    query.extend_from_slice(&hash_id.0);
    query.extend_from_slice(b"e1:q9:get_peers1:t2:aa1:y1:qe");
    let port = seeded.listen_port.unwrap().to_be_bytes();
    let compact = [127, 0, 0, 1, port[0], port[1]];
    tokio::time::timeout(Duration::from_secs(25), async {
        let mut response = [0u8; 4096];
        loop {
            probe.send_to(&query, addr_a).await.unwrap();
            if let Ok(Ok((n, _))) =
                tokio::time::timeout(Duration::from_millis(200), probe.recv_from(&mut response))
                    .await
                && response[..n].windows(6).any(|b| b == compact)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("seed must announce its actual TCP port through DHT");
    let preview = download
        .resolve_magnet(ResolveMagnetRequest {
            request_id: Uuid::new_v4().to_string(),
            magnet: format!("magnet:?xt=urn:btih:{hash}"),
            peer: String::new(),
            dht: true,
            trackers: vec![],
        })
        .await
        .unwrap()
        .preview
        .unwrap();
    assert_eq!(preview.info_hash, hash);
    let transfer = download
        .download(
            StartDownloadRequest {
                request_id: Uuid::new_v4().to_string(),
                token: preview.token,
                peer: String::new(),
                dht: true,
                trackers: vec![],
            },
            d.path().into(),
        )
        .await
        .unwrap();
    let completed = tokio::time::timeout(Duration::from_secs(45), async {
        loop {
            let job = download
                .list()
                .items
                .into_iter()
                .find(|v| v.id == transfer.id)
                .unwrap();
            if job.status == TransferStatus::Completed {
                break job;
            }
            assert_ne!(job.status, TransferStatus::Failed);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap();
    let root = d.path().join(completed.output_name.unwrap());
    let received = [
        fs::read(root.join("a.bin")).unwrap(),
        fs::read(root.join("nested/β.bin")).unwrap(),
    ]
    .concat();
    assert_eq!(Sha256::digest(&received), Sha256::digest(&original));
    download.shutdown().await;
    seed.shutdown().await;
    bootstrap.cancellation_token().cancel();
    neighbor.cancellation_token().cancel();
}

#[test]
fn tracker_policy_and_reference_rewriting_preserve_info_identity() {
    let (_dir, db, _, _, _) = setup();
    let raw = ever::from_catalog(&db, "1").unwrap().torrent;
    let urls = trackers::normalize(&[
        "https://TRACKER.example:443/announce?key=secret".into(),
        "udp://tracker.example:6969/announce".into(),
    ])
    .unwrap();
    assert_eq!(urls[0], "https://tracker.example/announce?key=secret");
    let routed = ever::route(&raw, &urls).unwrap();
    assert_eq!(inspect(&routed).unwrap().hash, inspect(&raw).unwrap().hash);
    assert_eq!(inspect(&routed).unwrap().trackers, urls);
    assert_eq!(
        inspect(&ever::route(&routed, &[]).unwrap())
            .unwrap()
            .trackers
            .len(),
        0
    );
    let link = trackers::magnet(&inspect(&raw).unwrap().hash, &urls);
    assert_eq!(
        url::Url::parse(&link)
            .unwrap()
            .query_pairs()
            .filter(|(k, _)| k == "tr")
            .count(),
        2
    );
    assert_eq!(
        normalize_magnet(&link).unwrap().info_hash,
        inspect(&raw).unwrap().hash
    );
    for bad in [
        "file:///C:/test",
        "ws://tracker.example/",
        "https://user:password@tracker.example/",
        "http://tracker.example/#fragment",
        "udp://tracker.example/announce",
        "udp://tracker.example:80/announce?passkey=unsupported",
        "http://tracker.example:0/",
        "http://tracker.example/a b",
    ] {
        assert!(
            trackers::normalize(&[bad.into()]).is_err(),
            "accepted {bad}"
        );
    }
    assert!(trackers::normalize(&vec!["http://example.invalid/".into(); 17]).is_err());
    assert_eq!(
        trackers::normalize(&[urls[0].clone(), urls[0].clone()])
            .unwrap()
            .len(),
        1
    );
}

struct MockTracker {
    url: String,
    ports: Arc<Mutex<Vec<u16>>>,
    calls: Arc<std::sync::atomic::AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for MockTracker {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn mock_tracker(udp: bool) -> MockTracker {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let ports = Arc::new(Mutex::new(Vec::<u16>::new()));
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let records = ports.clone();
    let count = calls.clone();
    if udp {
        let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let url = format!("udp://{}/announce", socket.local_addr().unwrap());
        let task = tokio::spawn(async move {
            let mut buf = [0u8; 4096];
            loop {
                let (n, addr) = socket.recv_from(&mut buf).await.unwrap();
                if n < 16 {
                    continue;
                }
                let action = u32::from_be_bytes(buf[8..12].try_into().unwrap());
                let mut response = action.to_be_bytes().to_vec();
                response.extend_from_slice(&buf[12..16]);
                if action == 0 {
                    response.extend_from_slice(&42u64.to_be_bytes());
                } else if action == 1 && n >= 98 {
                    count.fetch_add(1, Ordering::SeqCst);
                    let left = u64::from_be_bytes(buf[64..72].try_into().unwrap());
                    let port = u16::from_be_bytes(buf[96..98].try_into().unwrap());
                    if left == 0 && !records.lock().unwrap().contains(&port) {
                        records.lock().unwrap().push(port);
                    }
                    response.extend_from_slice(&1u32.to_be_bytes());
                    response.extend_from_slice(&1u32.to_be_bytes());
                    response.extend_from_slice(&1u32.to_be_bytes());
                    for p in records.lock().unwrap().iter() {
                        response.extend_from_slice(&[127, 0, 0, 1]);
                        response.extend_from_slice(&p.to_be_bytes());
                    }
                } else {
                    continue;
                }
                socket.send_to(&response, addr).await.unwrap();
            }
        });
        MockTracker {
            url,
            ports,
            calls,
            task,
        }
    } else {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/announce", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = vec![0u8; 8192];
                let mut n = 0;
                while !request[..n].windows(4).any(|v| v == b"\r\n\r\n") {
                    let read = socket.read(&mut request[n..]).await.unwrap();
                    if read == 0 {
                        break;
                    }
                    n += read;
                }
                let text = String::from_utf8_lossy(&request[..n]);
                let Some(target) = text.split_whitespace().nth(1) else {
                    continue;
                };
                let u = url::Url::parse(&format!("http://tracker.local{target}")).unwrap();
                let fields: HashMap<_, _> = u
                    .query_pairs()
                    .map(|(k, v)| (k.into_owned(), v.into_owned()))
                    .collect();
                count.fetch_add(1, Ordering::SeqCst);
                let port: u16 = fields["port"].parse().unwrap();
                if fields["left"] == "0" && !records.lock().unwrap().contains(&port) {
                    records.lock().unwrap().push(port);
                }
                let mut compact = Vec::new();
                for p in records.lock().unwrap().iter() {
                    compact.extend_from_slice(&[127, 0, 0, 1]);
                    compact.extend_from_slice(&p.to_be_bytes());
                }
                let mut body = format!("d8:intervali1e5:peers{}:", compact.len()).into_bytes();
                body.extend_from_slice(&compact);
                body.push(b'e');
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                socket.write_all(header.as_bytes()).await.unwrap();
                socket.write_all(&body).await.unwrap();
            }
        });
        MockTracker {
            url,
            ports,
            calls,
            task,
        }
    }
}
async fn tracker_roundtrip(udp: bool) {
    let (d, db, imports, root, original) = setup();
    let tracker = mock_tracker(udp).await;
    let unselected = mock_tracker(false).await;
    let seed = manager();
    let download = manager();
    let reservation = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = reservation.local_addr().unwrap().port();
    drop(reservation);
    let mut req = seed_request();
    req.listen_port = port;
    req.trackers = vec![tracker.url.clone()];
    let seeded = seed.seed(db.clone(), imports, req).await.unwrap();
    assert_eq!(seeded.tracker_count, 1);
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            if tracker.ports.lock().unwrap().contains(&port) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("tracker must receive seeding announce");
    let metadata = download
        .resolve_magnet(ResolveMagnetRequest {
            request_id: Uuid::new_v4().to_string(),
            magnet: trackers::magnet(&seeded.info_hash, std::slice::from_ref(&tracker.url)),
            peer: String::new(),
            dht: false,
            trackers: vec![tracker.url.clone()],
        })
        .await
        .unwrap()
        .preview
        .unwrap();
    assert_eq!(metadata.trackers, vec![tracker.url.clone()]);
    assert!(download.list().items.is_empty());
    // Imported routing suggestions do not override the explicitly selected list.
    let raw = ever::from_catalog(&db, "1").unwrap().torrent;
    let preview = download
        .import(ever::route(&raw, std::slice::from_ref(&unselected.url)).unwrap())
        .unwrap();
    assert_eq!(preview.trackers, vec![unselected.url.clone()]);
    let job = download
        .download(
            StartDownloadRequest {
                request_id: Uuid::new_v4().to_string(),
                token: preview.token,
                peer: String::new(),
                dht: false,
                trackers: vec![tracker.url.clone()],
            },
            d.path().into(),
        )
        .await
        .unwrap();
    let done = tokio::time::timeout(Duration::from_secs(35), async {
        loop {
            let row = download
                .list()
                .items
                .into_iter()
                .find(|t| t.id == job.id)
                .unwrap();
            if row.status == TransferStatus::Completed {
                break row;
            }
            assert_ne!(row.status, TransferStatus::Failed);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap();
    let output = d.path().join(done.output_name.unwrap());
    let received = [
        fs::read(output.join("a.bin")).unwrap(),
        fs::read(output.join("nested/β.bin")).unwrap(),
    ]
    .concat();
    assert_eq!(Sha256::digest(&received), Sha256::digest(&original));
    assert_eq!(
        fs::read(root.join("a.bin")).unwrap(),
        original[..1024 * 1024 - 3]
    );
    assert_eq!(unselected.calls.load(Ordering::SeqCst), 0);
    assert!(tracker.calls.load(Ordering::SeqCst) >= 3);
    download.shutdown().await;
    seed.shutdown().await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn http_tracker_discovers_metadata_and_payload_without_dht_or_manual_peer() {
    tracker_roundtrip(false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn udp_tracker_discovers_metadata_and_payload_without_dht_or_manual_peer() {
    tracker_roundtrip(true).await;
}
