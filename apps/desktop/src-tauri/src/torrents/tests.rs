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
