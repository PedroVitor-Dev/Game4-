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
        seed_after_download: false,
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
    assert_eq!(fs::read_dir(&parent).unwrap().count(), 2); // payload directory + ownership sidecar
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
        seed_after_download: false,
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
        seed_after_download: false,
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
                seed_after_download: false,
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
                seed_after_download: false,
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
                seed_after_download: false,
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn upload_peer_observations_and_download_to_seeding_handoff_work_end_to_end() {
    let (d, db, imports, _, original) = setup();
    let source = manager();
    let receiver = manager();
    let next = manager();
    let raw = ever::from_catalog(&db, "1").unwrap().torrent;
    let seeded = source.seed(db, imports, seed_request()).await.unwrap();
    {
        let entries = source.entries.lock().unwrap();
        let runtime = entries[&seeded.id].runtime.lock().unwrap();
        runtime
            .as_ref()
            .unwrap()
            .session
            .ratelimits
            .set_upload_bps(std::num::NonZeroU32::new(262144));
    }
    let preview = receiver.import(raw.clone()).unwrap();
    let request = StartDownloadRequest {
        request_id: Uuid::new_v4().to_string(),
        token: preview.token,
        peer: format!("127.0.0.1:{}", seeded.listen_port.unwrap()),
        dht: false,
        trackers: vec![],
        seed_after_download: true,
    };
    let started = receiver
        .download(request.clone(), d.path().into())
        .await
        .unwrap();
    let mut seeder_seen = false;
    let mut leecher_seen = false;
    let mut down_speed_seen = false;
    let mut up_speed_seen = false;
    let complete = tokio::time::timeout(Duration::from_secs(35), async {
        loop {
            let received = receiver
                .list()
                .items
                .into_iter()
                .find(|v| v.id == started.id)
                .unwrap();
            let sending = source
                .list()
                .items
                .into_iter()
                .find(|v| v.id == seeded.id)
                .unwrap();
            if let Some(t) = &received.telemetry {
                seeder_seen |= t.seeders > 0;
                down_speed_seen |= t.download_bps.parse::<u64>().unwrap() > 0;
                assert_eq!(t.seeders + t.leechers, t.connected_peers);
            }
            if let Some(t) = &sending.telemetry {
                leecher_seen |= t.leechers > 0;
                up_speed_seen |= t.upload_bps.parse::<u64>().unwrap() > 0;
            }
            assert_ne!(received.status, TransferStatus::Failed);
            if received.is_seeding {
                break received;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
    assert!(
        seeder_seen && leecher_seen && down_speed_seen && up_speed_seen,
        "real peer classes and transfer rates must be observed"
    );
    assert_eq!(complete.status, TransferStatus::Active);
    assert_eq!(complete.verified_bytes, original.len().to_string());
    assert!(complete.received_bytes.parse::<u64>().unwrap() >= original.len() as u64);
    {
        let entries = receiver.entries.lock().unwrap();
        let runtime = entries[&started.id].runtime.lock().unwrap();
        assert!(
            runtime
                .as_ref()
                .unwrap()
                .io
                .pwrite_all(0, 0, b"bad")
                .is_err(),
            "completed download storage must be sealed"
        );
    }
    let mut conflicting = request;
    conflicting.seed_after_download = false;
    assert!(
        receiver
            .download(conflicting, d.path().into())
            .await
            .is_err()
    );
    source.stop(&seeded.id).await.unwrap();
    // The original source is now offline. A third session must download solely
    // from the completed download that explicitly stayed online as a seed.
    let preview = next.import(raw).unwrap();
    let next_job = next
        .download(
            StartDownloadRequest {
                request_id: Uuid::new_v4().to_string(),
                token: preview.token,
                peer: format!("127.0.0.1:{}", complete.listen_port.unwrap()),
                dht: false,
                trackers: vec![],
                seed_after_download: false,
            },
            d.path().into(),
        )
        .await
        .unwrap();
    let next_done = tokio::time::timeout(Duration::from_secs(25), async {
        loop {
            let row = next
                .list()
                .items
                .into_iter()
                .find(|v| v.id == next_job.id)
                .unwrap();
            if row.status == TransferStatus::Completed {
                break row;
            }
            assert_ne!(row.status, TransferStatus::Failed);
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
    let root = d.path().join(next_done.output_name.unwrap());
    let received = [
        fs::read(root.join("a.bin")).unwrap(),
        fs::read(root.join("nested/β.bin")).unwrap(),
    ]
    .concat();
    assert_eq!(Sha256::digest(&received), Sha256::digest(&original));
    let uploading = receiver
        .list()
        .items
        .into_iter()
        .find(|v| v.id == started.id)
        .unwrap();
    assert!(uploading.uploaded_bytes.parse::<u64>().unwrap() > 0);
    let stopped = receiver.stop(&started.id).await.unwrap();
    assert!(!stopped.is_seeding);
    assert!(stopped.telemetry.is_none());
    assert!(stopped.listen_port.is_none());
    assert_eq!(stopped.uploaded_bytes, uploading.uploaded_bytes);
    assert!(next_done.telemetry.is_none());
    receiver.shutdown().await;
    next.shutdown().await;
    source.shutdown().await;
}

fn persistent(path: &Path) -> Arc<Transfers> {
    let mut m = Transfers::open(path).unwrap();
    m.listen_ip = Ipv4Addr::LOCALHOST;
    Arc::new(m)
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn persisted_partial_download_reopens_offline_rechecks_and_repairs_without_a_new_copy() {
    use std::io::{Seek, SeekFrom, Write};
    let (d, db, imports, _, original) = setup();
    let source = manager();
    let raw = ever::from_catalog(&db, "1").unwrap().torrent;
    let seeded = source.seed(db, imports, seed_request()).await.unwrap();
    {
        let entries = source.entries.lock().unwrap();
        let rt = entries[&seeded.id].runtime.lock().unwrap();
        rt.as_ref()
            .unwrap()
            .session
            .ratelimits
            .set_upload_bps(std::num::NonZeroU32::new(262144));
    }
    let state = d.path().join("private/state.sqlite3");
    let receive = persistent(&state);
    let preview = receive.import(raw).unwrap();
    let parent = d.path().join("received");
    fs::create_dir(&parent).unwrap();
    let request = StartDownloadRequest {
        request_id: Uuid::new_v4().to_string(),
        token: preview.token,
        peer: format!("127.0.0.1:{}", seeded.listen_port.unwrap()),
        dht: false,
        trackers: vec![],
        seed_after_download: false,
    };
    let started = receive
        .download(request.clone(), parent.clone())
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            let row = receive.list().items[0].clone();
            if row.verified_bytes.parse::<u64>().unwrap() >= 1048576 {
                break;
            }
            assert_ne!(row.status, TransferStatus::Failed);
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
    let paused = receive.pause(&started.id).await.unwrap();
    assert_eq!(paused.status, TransferStatus::Paused);
    assert!(paused.telemetry.is_none());
    assert!(paused.listen_port.is_none());
    assert!(paused.verified_bytes.parse::<u64>().unwrap() < original.len() as u64);
    let root = parent.join(paused.output_name.clone().unwrap());
    let mut file = fs::OpenOptions::new()
        .write(true)
        .open(root.join("a.bin"))
        .unwrap();
    file.seek(SeekFrom::Start(0)).unwrap();
    file.write_all(&[0xee]).unwrap();
    file.sync_all().unwrap();
    drop(file);
    receive.shutdown().await;
    drop(receive);
    let restored = persistent(&state);
    let row = restored.list().items[0].clone();
    assert_eq!(row.id, started.id);
    assert_eq!(row.status, TransferStatus::Paused);
    assert!(row.telemetry.is_none());
    assert!(
        restored.entries.lock().unwrap()[&row.id]
            .runtime
            .lock()
            .unwrap()
            .is_none()
    );
    assert_eq!(
        restored.download(request, parent.clone()).await.unwrap().id,
        row.id,
        "request replay survives restart"
    );
    source.entries.lock().unwrap()[&seeded.id]
        .runtime
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .session
        .ratelimits
        .set_upload_bps(None);
    restored.resume(&row.id).await.unwrap();
    let complete = tokio::time::timeout(Duration::from_secs(25), async {
        loop {
            let s = restored.list().items[0].clone();
            if s.status == TransferStatus::Completed {
                break s;
            }
            assert_ne!(s.status, TransferStatus::Failed);
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
    let received = [
        fs::read(root.join("a.bin")).unwrap(),
        fs::read(root.join("nested/β.bin")).unwrap(),
    ]
    .concat();
    assert_eq!(Sha256::digest(&received), Sha256::digest(&original));
    assert!(
        complete.received_bytes.parse::<u64>().unwrap()
            >= paused.received_bytes.parse::<u64>().unwrap() + 1048576
    );
    assert_eq!(
        fs::read_dir(&parent)
            .unwrap()
            .filter(|e| e.as_ref().unwrap().path().is_dir())
            .count(),
        1
    );
    restored.forget(&row.id).await.unwrap();
    assert!(restored.list().items.is_empty());
    assert!(root.join("a.bin").exists());
    assert!(persistent(&state).list().items.is_empty());
    source.shutdown().await;
    restored.shutdown().await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn persisted_seed_recovers_paused_refuses_changed_original_and_keeps_listening_port() {
    let (d, db, imports, root, original) = setup();
    let path = d.path().join("private/state.sqlite3");
    let m = persistent(&path);
    let seeded = m.seed(db, imports, seed_request()).await.unwrap();
    let port = seeded.listen_port;
    assert!(m.forget(&seeded.id).await.is_err());
    m.pause(&seeded.id).await.unwrap();
    m.shutdown().await;
    drop(m);
    let original_file = root.join("a.bin");
    let mut changed = fs::read(&original_file).unwrap();
    changed[0] = 0x33;
    fs::write(&original_file, &changed).unwrap();
    let restored = persistent(&path);
    assert_eq!(restored.list().items[0].status, TransferStatus::Paused);
    assert!(matches!(
        restored.resume(&seeded.id).await.unwrap_err().code,
        ErrorCode::SourceChanged
    ));
    assert_eq!(fs::read(&original_file).unwrap(), changed);
    assert!(
        restored.entries.lock().unwrap()[&seeded.id]
            .runtime
            .lock()
            .unwrap()
            .is_none()
    );
    fs::write(&original_file, &original[..1024 * 1024 - 3]).unwrap();
    let (a, b) = tokio::join!(restored.resume(&seeded.id), restored.resume(&seeded.id));
    assert_eq!(a.unwrap().listen_port, port);
    assert_eq!(b.unwrap().listen_port, port);
    assert!(restored.list().items[0].is_seeding);
    restored.shutdown().await;
    assert_eq!(
        persistent(&path).list().items[0].status,
        TransferStatus::Paused
    );
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn unsafe_partial_ownership_oversized_files_and_cancelled_resume_never_open_payload_sessions()
{
    let (d, db, _, _, _) = setup();
    let path = d.path().join("private/state.sqlite3");
    let m = persistent(&path);
    let preview = m
        .import(ever::from_catalog(&db, "1").unwrap().torrent)
        .unwrap();
    let guard = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let job = m
        .download(
            StartDownloadRequest {
                request_id: Uuid::new_v4().to_string(),
                token: preview.token,
                peer: guard.local_addr().unwrap().to_string(),
                dht: false,
                trackers: vec![],
                seed_after_download: false,
            },
            d.path().into(),
        )
        .await
        .unwrap();
    m.pause(&job.id).await.unwrap();
    let root = d.path().join(job.output_name.unwrap());
    let marker = root.parent().unwrap().join(format!(
        ".{}.owner.json",
        root.file_name().unwrap().to_str().unwrap()
    ));
    let correct = fs::read(&marker).unwrap();
    fs::write(&marker, b"{}").unwrap();
    assert!(m.resume(&job.id).await.is_err());
    assert_eq!(fs::metadata(root.join("a.bin")).unwrap().len(), 0);
    fs::write(&marker, correct).unwrap();
    let oversized = vec![0x44; 1024 * 1024];
    fs::write(root.join("a.bin"), &oversized).unwrap();
    assert!(m.resume(&job.id).await.is_err());
    assert_eq!(fs::read(root.join("a.bin")).unwrap(), oversized);
    fs::write(root.join("a.bin"), b"").unwrap();
    let entry = m.entries.lock().unwrap()[&job.id].clone();
    let held = entry.finish_gate.lock().await;
    let resuming = tokio::spawn({
        let m = m.clone();
        let id = job.id.clone();
        async move { m.resume(&id).await }
    });
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if entry.operation.lock().unwrap().is_some() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let pausing = tokio::spawn({
        let m = m.clone();
        let id = job.id.clone();
        async move { m.pause(&id).await }
    });
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if entry
                .operation
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .is_cancelled()
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    drop(held);
    assert_eq!(
        resuming.await.unwrap().unwrap().status,
        TransferStatus::Paused
    );
    assert_eq!(
        pausing.await.unwrap().unwrap().status,
        TransferStatus::Paused
    );
    assert!(entry.runtime.lock().unwrap().is_none());
    m.shutdown().await;
}
#[test]
fn unsupported_or_corrupt_transfer_store_is_not_reset_and_does_not_affect_catalog() {
    let (d, db, _, _, _) = setup();
    let path = d.path().join("private/state.sqlite3");
    let m = persistent(&path);
    drop(m);
    let c = rusqlite::Connection::open(&path).unwrap();
    c.pragma_update(None, "user_version", 99).unwrap();
    assert!(Transfers::open(&path).is_err());
    assert_eq!(
        c.pragma_query_value::<i64, _>(None, "user_version", |r| r.get(0))
            .unwrap(),
        99
    );
    c.pragma_update(None, "user_version", 1).unwrap();
    c.execute(
        "INSERT INTO transfers VALUES (?1,'not-json',?2,'{}')",
        rusqlite::params![Uuid::new_v4().to_string(), b"bad metainfo".to_vec()],
    )
    .unwrap();
    assert!(Transfers::open(&path).is_err());
    assert_eq!(
        c.query_row::<i64, _, _>("SELECT count(*) FROM transfers", [], |r| r.get(0))
            .unwrap(),
        1
    );
    assert_eq!(db.info().unwrap().item_count, 1);
    assert!(Transfers::unavailable().ensure_ready().is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn checkpoint_write_failure_stops_sharing_without_resetting_saved_intent() {
    let (d, db, imports, _, _) = setup();
    let path = d.path().join("private/state.sqlite3");
    let m = persistent(&path);
    let seed = m.seed(db, imports, seed_request()).await.unwrap();
    m.store.as_ref().unwrap().connection_for_test(true);
    tokio::time::timeout(Duration::from_secs(6), async {
        loop {
            let row = m.list().items[0].clone();
            if row.status == TransferStatus::Failed {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
    assert!(
        m.entries.lock().unwrap()[&seed.id]
            .runtime
            .lock()
            .unwrap()
            .is_none()
    );
    m.store.as_ref().unwrap().connection_for_test(false);
    let reopened = persistent(&path);
    assert_eq!(reopened.list().items[0].status, TransferStatus::Paused);
    reopened.resume(&seed.id).await.unwrap();
    reopened.shutdown().await;
    m.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn early_v1_inline_snapshots_upgrade_without_losing_identity_or_starting_network() {
    let (d, db, imports, _, _) = setup();
    let path = d.path().join("private/state.sqlite3");
    let m = persistent(&path);
    let seed = m.seed(db, imports, seed_request()).await.unwrap();
    let paused = m.pause(&seed.id).await.unwrap();
    m.shutdown().await;
    drop(m);
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute(
        "UPDATE transfers SET snapshot=?1",
        [serde_json::to_string(&paused).unwrap()],
    )
    .unwrap();
    c.execute_batch("DROP TABLE transfer_snapshots").unwrap();
    drop(c);
    let restored = persistent(&path);
    let row = restored.list().items[0].clone();
    assert_eq!(row.id, seed.id);
    assert_eq!(row.status, TransferStatus::Paused);
    assert!(row.listen_port.is_none());
    assert!(
        restored.entries.lock().unwrap()[&seed.id]
            .runtime
            .lock()
            .unwrap()
            .is_none()
    );
    restored.shutdown().await;
}
