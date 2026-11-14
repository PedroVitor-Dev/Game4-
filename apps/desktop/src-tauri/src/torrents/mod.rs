//! Real BitTorrent sessions, started only by explicit seed/download commands.
use crate::{
    contracts::*,
    database::Database,
    ever::{self, Layout},
    library::{self, ImportState},
};
use librqbit::{
    AddTorrent, AddTorrentOptions, ListenerOptions, ManagedTorrent, Session, SessionOptions,
    storage::StorageFactoryExt,
};
use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use uuid::Uuid;
mod magnets;
mod storage;
pub mod trackers;
pub use magnets::normalize_magnet;
use storage::Handles;
const MAX_TOTAL: u64 = 1024 * 1024 * 1024 * 1024;
const MAX_FILES: usize = 1024;
type Result<T> = std::result::Result<T, IpcError>;
fn err(c: ErrorCode) -> IpcError {
    IpcError::new(c)
}
fn failed() -> IpcError {
    err(ErrorCode::TransferFailed)
}
fn limit() -> IpcError {
    err(ErrorCode::TransferLimitExceeded)
}
fn valid_uuid(s: &str) -> Result<()> {
    if !Uuid::parse_str(s).is_ok_and(|v| {
        v.get_version_num() == 4 && v.get_variant() == uuid::Variant::RFC4122 && v.to_string() == s
    }) {
        return Err(library::invalid(ErrorField::Request));
    }
    Ok(())
}
pub fn peer(s: &str) -> Result<SocketAddr> {
    let SocketAddr::V4(addr) = s
        .trim()
        .parse::<SocketAddr>()
        .map_err(|_| library::invalid(ErrorField::Request))?
    else {
        return Err(library::invalid(ErrorField::Request));
    };
    if addr.port() == 0
        || addr.ip().is_unspecified()
        || addr.ip().is_multicast()
        || addr.ip().is_broadcast()
    {
        return Err(library::invalid(ErrorField::Request));
    }
    Ok(SocketAddr::V4(addr))
}
pub(crate) fn inspect(raw: &[u8]) -> Result<Layout> {
    let mut layout = ever::inspect(raw).map_err(|e| {
        if matches!(e.code, ErrorCode::G4eLimitExceeded) {
            limit()
        } else {
            err(ErrorCode::TorrentInvalid)
        }
    })?;
    let total = layout
        .files
        .iter()
        .try_fold(0u64, |a, (_, n)| a.checked_add(*n))
        .ok_or_else(limit)?;
    if layout.files.len() > MAX_FILES || total > MAX_TOTAL {
        return Err(limit());
    }
    if total == 0 {
        return Err(err(ErrorCode::TorrentInvalid));
    }
    layout.trackers = trackers::normalize(&layout.trackers)?;
    librqbit::torrent_from_bytes(raw).map_err(|_| err(ErrorCode::TorrentInvalid))?;
    Ok(layout)
}
fn total(meta: &Layout) -> u64 {
    meta.files.iter().map(|(_, n)| *n).sum()
}
pub fn read_file(path: &Path) -> Result<Vec<u8>> {
    let f = library::open_source(path)?;
    let mut bytes = Vec::new();
    f.take((ever::MAX_METAINFO + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| err(ErrorCode::TorrentInvalid))?;
    inspect(&bytes)?;
    Ok(bytes)
}
pub fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    inspect(bytes)?;
    if !path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("torrent"))
    {
        return Err(err(ErrorCode::InvalidArgument));
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|_| failed())?;
    temp.write_all(bytes).map_err(|_| failed())?;
    temp.as_file().sync_all().map_err(|_| failed())?;
    temp.persist_noclobber(path).map_err(|_| failed())?;
    Ok(())
}
#[derive(Clone)]
struct Pending {
    raw: Vec<u8>,
    meta: Layout,
    created: Instant,
}
struct Runtime {
    session: Arc<Session>,
    handle: Arc<ManagedTorrent>,
    io: Handles,
}
struct Transfer {
    finish_gate: tokio::sync::Mutex<()>,
    snapshot: Mutex<TransferSnapshot>,
    runtime: Mutex<Option<Runtime>>,
}
impl Transfer {
    fn snapshot(&self) -> TransferSnapshot {
        let runtime = self.runtime.lock().unwrap_or_else(|e| e.into_inner());
        let mut snapshot = self.snapshot.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(rt) = runtime.as_ref() {
            let stats = rt.handle.stats();
            snapshot.verified_bytes = stats
                .progress_bytes
                .min(snapshot.total_bytes.parse().unwrap_or(0))
                .to_string();
            snapshot.uploaded_bytes = stats.uploaded_bytes.to_string();
        }
        snapshot.clone()
    }
    async fn finish(&self, mut status: TransferStatus) {
        let _finish = self.finish_gate.lock().await;
        let runtime = {
            let mut runtime = self.runtime.lock().unwrap_or_else(|e| e.into_inner());
            let rt = runtime.take();
            if let Some(ref rt) = rt {
                let stats = rt.handle.stats();
                let mut s = self.snapshot.lock().unwrap_or_else(|e| e.into_inner());
                s.verified_bytes = stats
                    .progress_bytes
                    .min(s.total_bytes.parse().unwrap_or(0))
                    .to_string();
                s.uploaded_bytes = stats.uploaded_bytes.to_string();
                s.status = TransferStatus::Stopping;
            }
            rt
        };
        if let Some(rt) = runtime {
            if status == TransferStatus::Completed {
                let io = rt.io.clone();
                if !tokio::task::spawn_blocking(move || io.flush())
                    .await
                    .is_ok_and(|r| r.is_ok())
                {
                    status = TransferStatus::Failed;
                }
            }
            rt.session.stop().await;
            drop(rt);
            let mut s = self.snapshot.lock().unwrap_or_else(|e| e.into_inner());
            s.status = status;
            s.listen_port = None;
            if status == TransferStatus::Failed {
                s.error = Some(failed());
            }
        }
    }
}
struct StartSession {
    raw: Vec<u8>,
    meta: Layout,
    handles: Handles,
    mode: TransferMode,
    request: String,
    fingerprint: String,
    port: u16,
    dht: bool,
    trackers: Vec<String>,
    initial_peer: Option<SocketAddr>,
    output: Option<String>,
}
struct Accepted {
    fingerprint: String,
    id: String,
}
pub struct Transfers {
    gate: tokio::sync::Mutex<()>,
    pending: Mutex<HashMap<String, Pending>>,
    entries: Mutex<HashMap<String, Arc<Transfer>>>,
    requests: Mutex<HashMap<String, Accepted>>,
    closing: AtomicBool,
    listen_ip: Ipv4Addr,
    cancel: tokio_util::sync::CancellationToken,
    resolution: Mutex<Option<magnets::Resolution>>,
    dht_bootstrap: Option<Vec<String>>,
}
impl Default for Transfers {
    fn default() -> Self {
        Self {
            gate: tokio::sync::Mutex::new(()),
            pending: Mutex::new(HashMap::new()),
            entries: Mutex::new(HashMap::new()),
            requests: Mutex::new(HashMap::new()),
            closing: AtomicBool::new(false),
            listen_ip: Ipv4Addr::UNSPECIFIED,
            cancel: tokio_util::sync::CancellationToken::new(),
            resolution: Mutex::new(None),
            dht_bootstrap: None,
        }
    }
}
impl Transfers {
    fn dht_config(&self, enabled: bool) -> Option<librqbit::DhtSessionConfig> {
        enabled.then(|| librqbit::DhtSessionConfig {
            bootstrap_addrs: self.dht_bootstrap.clone(),
            port: Some(0),
            persistence: None,
        })
    }
    pub fn import(&self, raw: Vec<u8>) -> Result<TorrentPreview> {
        let meta = inspect(&raw)?;
        let mut pending = self.pending.lock().map_err(|_| failed())?;
        pending.retain(|_, p| p.created.elapsed() < Duration::from_secs(300));
        if pending.len() >= 16 {
            return Err(limit());
        }
        let token = Uuid::new_v4().to_string();
        let preview = TorrentPreview {
            token: token.clone(),
            name: meta.name.clone(),
            info_hash: meta.hash.clone(),
            trackers: meta.trackers.clone(),
            total_bytes: total(&meta).to_string(),
            files: meta
                .files
                .iter()
                .map(|(p, n)| TorrentFileEntry {
                    path: p.clone(),
                    size_bytes: n.to_string(),
                })
                .collect(),
        };
        pending.insert(
            token,
            Pending {
                raw,
                meta,
                created: Instant::now(),
            },
        );
        Ok(preview)
    }
    pub fn list(&self) -> TransferPage {
        let mut items: Vec<_> = self
            .entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .map(|t| t.snapshot())
            .collect();
        items.sort_by(|a, b| a.id.cmp(&b.id));
        TransferPage { items }
    }
    fn replay(&self, id: &str, fingerprint: &str) -> Result<Option<TransferSnapshot>> {
        valid_uuid(id)?;
        if let Some(a) = self.requests.lock().map_err(|_| failed())?.get(id) {
            if a.fingerprint != fingerprint {
                return Err(library::invalid(ErrorField::RequestId));
            }
            return self
                .entries
                .lock()
                .map_err(|_| failed())?
                .get(&a.id)
                .map(|t| Some(t.snapshot()))
                .ok_or_else(failed);
        }
        Ok(None)
    }
    fn available(&self) -> Result<()> {
        if self.closing.load(Ordering::SeqCst) {
            return Err(err(ErrorCode::TransferUnavailable));
        }
        let entries = self.entries.lock().map_err(|_| failed())?;
        if entries.len() >= 64
            || entries
                .values()
                .filter(|t| {
                    matches!(
                        t.snapshot().status,
                        TransferStatus::Active | TransferStatus::Stopping
                    )
                })
                .count()
                >= 4
        {
            return Err(limit());
        }
        Ok(())
    }
    async fn start(self: &Arc<Self>, input: StartSession) -> Result<TransferSnapshot> {
        let StartSession {
            raw,
            meta,
            handles,
            mode,
            request,
            fingerprint,
            port,
            dht,
            trackers,
            initial_peer,
            output,
        } = input;
        if self.closing.load(Ordering::SeqCst) {
            return Err(err(ErrorCode::TransferUnavailable));
        }
        let session = Session::new_with_opts(
            PathBuf::new(),
            SessionOptions {
                cancellation_token: Some(self.cancel.child_token()),
                dht: self.dht_config(dht),
                disable_trackers: trackers.is_empty(),
                disable_local_service_discovery: true,
                persistence: None,
                fastresume: false,
                ipv4_only: true,
                listen: Some(ListenerOptions {
                    listen_addr: SocketAddr::V4(SocketAddrV4::new(self.listen_ip, port)),
                    ipv4_only: true,
                    // The engine suppresses loopback announcements by default. Tests use
                    // an explicit bound port to exercise an isolated local DHT swarm.
                    announce_port: ((dht || !trackers.is_empty())
                        && self.listen_ip.is_loopback()
                        && port != 0)
                        .then_some(port),
                    enable_upnp_port_forwarding: false,
                    ..Default::default()
                }),
                peer_limit: Some(8),
                ..Default::default()
            },
        )
        .await
        .map_err(|_| failed())?;
        let result = async {
            let handle = session
                .add_torrent(
                    AddTorrent::from_bytes(ever::route(&raw, &trackers)?),
                    Some(AddTorrentOptions {
                        paused: true,
                        overwrite: false,
                        disable_trackers: trackers.is_empty(),
                        storage_factory: Some(handles.clone().boxed()),
                        initial_peers: initial_peer.map(|p| vec![p]),
                        peer_limit: Some(8),
                        ..Default::default()
                    }),
                )
                .await
                .map_err(|_| failed())?
                .into_handle()
                .ok_or_else(failed)?;
            tokio::time::timeout(Duration::from_secs(60), handle.wait_until_initialized())
                .await
                .map_err(|_| failed())?
                .map_err(|_| failed())?;
            if mode == TransferMode::Seed && !handle.stats().finished {
                return Err(err(ErrorCode::SourceChanged));
            }
            if self.closing.load(Ordering::SeqCst) {
                return Err(err(ErrorCode::TransferUnavailable));
            }
            session.unpause(&handle).await.map_err(|_| failed())?;
            Ok(handle)
        }
        .await;
        let handle = match result {
            Ok(h) => h,
            Err(e) => {
                session.stop().await;
                return Err(e);
            }
        };
        let total_bytes = total(&meta).to_string();
        let snapshot = TransferSnapshot {
            id: Uuid::new_v4().to_string(),
            name: meta.name,
            info_hash: meta.hash,
            mode,
            status: TransferStatus::Active,
            total_bytes,
            verified_bytes: handle.stats().progress_bytes.to_string(),
            uploaded_bytes: "0".into(),
            listen_port: session.listen_addr().map(|a| a.port()),
            dht,
            tracker_count: trackers.len() as u32,
            output_name: output,
            error: None,
        };
        let entry = Arc::new(Transfer {
            finish_gate: tokio::sync::Mutex::new(()),
            snapshot: Mutex::new(snapshot.clone()),
            runtime: Mutex::new(Some(Runtime {
                session,
                handle,
                io: handles,
            })),
        });
        self.entries
            .lock()
            .map_err(|_| failed())?
            .insert(snapshot.id.clone(), Arc::clone(&entry));
        self.requests.lock().map_err(|_| failed())?.insert(
            request,
            Accepted {
                fingerprint,
                id: snapshot.id.clone(),
            },
        );
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(250)).await;
                let state = {
                    let rt = entry.runtime.lock().unwrap_or_else(|e| e.into_inner());
                    rt.as_ref().map(|r| {
                        let stats = r.handle.stats();
                        (
                            stats.finished,
                            stats.error.is_some()
                                || matches!(stats.state, librqbit::TorrentStatsState::Error),
                        )
                    })
                };
                match state {
                    None => return,
                    Some((_, true)) => {
                        entry.finish(TransferStatus::Failed).await;
                        return;
                    }
                    Some((true, _)) if mode == TransferMode::Download => {
                        entry.finish(TransferStatus::Completed).await;
                        return;
                    }
                    _ => {}
                }
            }
        });
        Ok(snapshot)
    }
    pub async fn seed(
        self: &Arc<Self>,
        db: Arc<Database>,
        imports: Arc<ImportState>,
        request: StartSeedRequest,
    ) -> Result<TransferSnapshot> {
        let _gate = self.gate.lock().await;
        let trackers = trackers::normalize(&request.trackers)?;
        let fingerprint = format!(
            "seed:{}:{}:{}:{trackers:?}",
            request.item_id, request.listen_port, request.dht
        );
        if let Some(old) = self.replay(&request.request_id, &fingerprint)? {
            return Ok(old);
        }
        self.available()?;
        crate::manifests::item_id(&request.item_id)?;
        let prepared = tokio::task::spawn_blocking(move || {
            let _gate = imports
                .gate
                .try_lock()
                .map_err(|_| err(ErrorCode::LibraryBusy))?;
            let _worker = imports
                .worker
                .try_lock()
                .map_err(|_| err(ErrorCode::LibraryBusy))?;
            let active: bool = db
                .connection
                .lock()
                .map_err(|_| failed())?
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM tasks WHERE status IN ('queued','running'))",
                    [],
                    |r| r.get(0),
                )
                .map_err(|_| failed())?;
            if active {
                return Err(err(ErrorCode::LibraryBusy));
            }
            let created = ever::from_catalog(&db, &request.item_id)?;
            let meta = inspect(&created.torrent)?;
            let root: String = db
                .connection
                .lock()
                .map_err(|_| failed())?
                .query_row(
                    "SELECT source_key FROM import_sources WHERE item_id=?1",
                    [&request.item_id],
                    |r| r.get(0),
                )
                .map_err(|_| failed())?;
            let storage = Handles::seed(Path::new(&root), &meta)
                .map_err(|_| err(ErrorCode::SourceUnavailable))?;
            Ok((created.torrent, meta, storage))
        })
        .await
        .map_err(|_| failed())??;
        self.start(StartSession {
            raw: prepared.0,
            meta: prepared.1,
            handles: prepared.2,
            mode: TransferMode::Seed,
            request: request.request_id,
            fingerprint,
            port: request.listen_port,
            dht: request.dht,
            trackers,
            initial_peer: None,
            output: None,
        })
        .await
    }
    pub async fn download(
        self: &Arc<Self>,
        request: StartDownloadRequest,
        parent: PathBuf,
    ) -> Result<TransferSnapshot> {
        let _gate = self.gate.lock().await;
        let trackers = trackers::normalize(&request.trackers)?;
        let addr = magnets::initial_peer(&request.peer, request.dht || !trackers.is_empty())?;
        let fingerprint = format!(
            "download:{}:{addr:?}:{}:{trackers:?}",
            request.token, request.dht
        );
        if let Some(old) = self.replay(&request.request_id, &fingerprint)? {
            return Ok(old);
        }
        self.available()?;
        let pending = self
            .pending
            .lock()
            .map_err(|_| failed())?
            .get(&request.token)
            .filter(|p| p.created.elapsed() < Duration::from_secs(300))
            .cloned()
            .ok_or_else(|| library::invalid(ErrorField::SelectionToken))?;
        let name = format!("game4-download-{}", Uuid::new_v4());
        let output_name = name.clone();
        let meta = pending.meta.clone();
        let storage = tokio::task::spawn_blocking(move || {
            library::reject_ancestors(&parent)?;
            if !parent.is_dir() {
                return Err(library::invalid(ErrorField::Request));
            }
            let root = parent.join(name);
            std::fs::create_dir(&root).map_err(|_| failed())?;
            Handles::download(&root, &meta).map_err(|_| failed())
        })
        .await
        .map_err(|_| failed())??;
        let result = self
            .start(StartSession {
                raw: pending.raw,
                meta: pending.meta,
                handles: storage,
                mode: TransferMode::Download,
                request: request.request_id,
                fingerprint,
                port: 0,
                dht: request.dht,
                trackers,
                initial_peer: addr,
                output: Some(output_name),
            })
            .await?;
        self.pending
            .lock()
            .map_err(|_| failed())?
            .remove(&request.token);
        Ok(result)
    }
    pub async fn stop(&self, id: &str) -> Result<TransferSnapshot> {
        valid_uuid(id)?;
        let transfer = Arc::clone(
            self.entries
                .lock()
                .map_err(|_| failed())?
                .get(id)
                .ok_or_else(|| err(ErrorCode::NotFound))?,
        );
        transfer.finish(TransferStatus::Stopped).await;
        Ok(transfer.snapshot())
    }
    pub fn abort(&self) {
        self.closing.store(true, Ordering::SeqCst);
        self.cancel.cancel();
    }
    pub async fn shutdown(&self) {
        self.abort();
        self.closing.store(true, Ordering::SeqCst);
        let _gate = self.gate.lock().await;
        let entries: Vec<_> = self
            .entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .cloned()
            .collect();
        for e in entries {
            e.finish(TransferStatus::Stopped).await;
        }
    }
}
#[cfg(test)]
mod tests;
