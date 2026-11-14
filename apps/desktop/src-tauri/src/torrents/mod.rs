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
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use uuid::Uuid;
mod magnets;
mod persistence;
use persistence::{Owned, Recipe, Saved, Store};
mod storage;
pub mod trackers;
pub use magnets::normalize_magnet;
use storage::Handles;
const MAX_TOTAL: u64 = 1024 * 1024 * 1024 * 1024;
const MAX_FILES: usize = 1024;
// Governor's burst capacity must hold at least one standard 16 KiB piece request.
fn validate_limits(limits: BandwidthLimits) -> Result<()> {
    if [limits.download_bps, limits.upload_bps]
        .into_iter()
        .any(|n| n != 0 && n < 16 * 1024)
    {
        return Err(library::invalid(ErrorField::Request));
    }
    Ok(())
}
fn engine_limits(limits: BandwidthLimits) -> librqbit::limits::LimitsConfig {
    librqbit::limits::LimitsConfig {
        download_bps: std::num::NonZeroU32::new(limits.download_bps),
        upload_bps: std::num::NonZeroU32::new(limits.upload_bps),
    }
}
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
    sample: Mutex<RateSample>,
    base_received: u64,
    base_uploaded: u64,
}
struct RateSample {
    at: Instant,
    received: u64,
    uploaded: u64,
    down: u64,
    up: u64,
}
impl Default for RateSample {
    fn default() -> Self {
        Self {
            at: Instant::now(),
            received: 0,
            uploaded: 0,
            down: 0,
            up: 0,
        }
    }
}
impl Runtime {
    fn observe(&self, snapshot: &mut TransferSnapshot) {
        let stats = self.handle.stats();
        snapshot.verified_bytes = stats
            .progress_bytes
            .min(snapshot.total_bytes.parse().unwrap_or(0))
            .to_string();
        snapshot.uploaded_bytes = stats
            .uploaded_bytes
            .saturating_add(self.base_uploaded)
            .max(snapshot.uploaded_bytes.parse().unwrap_or(0))
            .to_string();
        snapshot.is_seeding &= stats.finished && stats.live.is_some();
        snapshot.telemetry = stats.live.map(|live| {
            let received = live.snapshot.fetched_bytes;
            snapshot.received_bytes = received
                .saturating_add(self.base_received)
                .max(snapshot.received_bytes.parse().unwrap_or(0))
                .to_string();
            let mut rate = self.sample.lock().unwrap_or_else(|e| e.into_inner());
            let elapsed = rate.at.elapsed();
            if elapsed >= Duration::from_millis(250) {
                let bps = |now: u64, before: u64| {
                    ((u128::from(now.saturating_sub(before)) * 1_000_000_000) / elapsed.as_nanos())
                        .min(u128::from(u64::MAX)) as u64
                };
                rate.down = bps(received, rate.received);
                rate.up = bps(stats.uploaded_bytes, rate.uploaded);
                rate.received = received;
                rate.uploaded = stats.uploaded_bytes;
                rate.at = Instant::now();
            }
            let peers = live.snapshot.peer_stats;
            let seeders = peers.live_seeders.min(peers.live);
            TransferTelemetry {
                connected_peers: peers.live,
                seeders,
                leechers: peers.live.saturating_sub(seeders),
                download_bps: rate.down.to_string(),
                upload_bps: rate.up.to_string(),
            }
        });
    }
}
struct Transfer {
    finish_gate: tokio::sync::Mutex<()>,
    snapshot: Mutex<TransferSnapshot>,
    runtime: Mutex<Option<Runtime>>,
    saved: Mutex<Saved>,
    store: Option<Arc<Store>>,
    generation: AtomicU64,
    operation: Mutex<Option<tokio_util::sync::CancellationToken>>,
}
impl Transfer {
    fn new(saved: Saved, snapshot: TransferSnapshot, store: Option<Arc<Store>>) -> Arc<Self> {
        Arc::new(Self {
            finish_gate: tokio::sync::Mutex::new(()),
            snapshot: Mutex::new(snapshot),
            runtime: Mutex::new(None),
            saved: Mutex::new(saved),
            store,
            generation: AtomicU64::new(0),
            operation: Mutex::new(None),
        })
    }
    async fn persist(&self, full: bool) -> Result<()> {
        let Some(store) = self.store.clone() else {
            return Ok(());
        };
        let snapshot = self.snapshot();
        let saved = full.then(|| self.saved.lock().unwrap_or_else(|e| e.into_inner()).clone());
        tokio::task::spawn_blocking(move || match saved {
            Some(v) => store.save(&v, &snapshot),
            None => store.snapshot(&snapshot),
        })
        .await
        .map_err(|_| failed())?
    }
    fn cancel_operation(&self) {
        if let Some(token) = self
            .operation
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
        {
            token.cancel();
        }
    }
    fn snapshot(&self) -> TransferSnapshot {
        let runtime = self.runtime.lock().unwrap_or_else(|e| e.into_inner());
        let mut snapshot = self.snapshot.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(rt) = runtime.as_ref() {
            rt.observe(&mut snapshot);
        }
        snapshot.clone()
    }
    async fn continue_seeding(&self, generation: u64) -> bool {
        let _finish = self.finish_gate.lock().await;
        if self.generation.load(Ordering::SeqCst) != generation {
            return true;
        }
        let io = {
            let runtime = self.runtime.lock().unwrap_or_else(|e| e.into_inner());
            let Some(rt) = runtime.as_ref() else {
                return true;
            };
            if self
                .snapshot
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .is_seeding
            {
                return true;
            }
            rt.io.clone()
        };
        if !tokio::task::spawn_blocking(move || io.seal())
            .await
            .is_ok_and(|r| r.is_ok())
        {
            return false;
        }
        let mut s = self.snapshot.lock().unwrap_or_else(|e| e.into_inner());
        s.is_seeding = true;
        s.verified_bytes = s.total_bytes.clone();
        true
    }
    async fn finish(&self, status: TransferStatus) -> Result<()> {
        self.cancel_operation();
        self.finish_inner(status, None).await
    }
    async fn finish_inner(&self, mut status: TransferStatus, expected: Option<u64>) -> Result<()> {
        let _finish = self.finish_gate.lock().await;
        if expected.is_some_and(|g| g != self.generation.load(Ordering::SeqCst)) {
            return Ok(());
        }
        if status == TransferStatus::Paused
            && matches!(
                self.snapshot
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .status,
                TransferStatus::Completed | TransferStatus::Failed | TransferStatus::Stopped
            )
        {
            return Ok(());
        }
        self.generation.fetch_add(1, Ordering::SeqCst);
        let runtime = {
            let mut rt = self.runtime.lock().unwrap_or_else(|e| e.into_inner());
            let taken = rt.take();
            let mut s = self.snapshot.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(ref r) = taken {
                r.observe(&mut s);
                s.status = TransferStatus::Stopping;
            }
            s.telemetry = None;
            s.is_seeding = false;
            taken
        };
        if let Some(rt) = runtime {
            rt.session.stop().await;
            let io = rt.io.clone();
            let complete = status == TransferStatus::Completed;
            if !tokio::task::spawn_blocking(move || {
                if complete {
                    io.flush()
                } else {
                    io.flush_partial()
                }
            })
            .await
            .is_ok_and(|v| v.is_ok())
            {
                status = TransferStatus::Failed;
            }
            drop(rt);
        }
        {
            let mut s = self.snapshot.lock().unwrap_or_else(|e| e.into_inner());
            if s.status != TransferStatus::Completed {
                s.status = status;
            }
            s.listen_port = None;
            s.error = if s.status == TransferStatus::Failed {
                Some(failed())
            } else {
                None
            };
        }
        self.persist(false).await
    }
}
struct StartSession {
    limits: BandwidthLimits,
    raw: Vec<u8>,
    meta: Layout,
    handles: Handles,
    mode: TransferMode,
    request: String,
    fingerprint: String,
    port: u16,
    dht: bool,
    trackers: Vec<String>,
    seed_after_download: bool,
    initial_peer: Option<SocketAddr>,
    output: Option<String>,
    root: PathBuf,
    owner: Option<Owned>,
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
    store: Option<Arc<Store>>,
    unavailable: bool,
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
            store: None,
            unavailable: false,
        }
    }
}
impl Transfers {
    pub fn open(path: &Path) -> Result<Self> {
        let (store, records) = Store::open(path)?;
        let manager = Self {
            store: Some(store.clone()),
            ..Default::default()
        };
        for (saved, snapshot) in records {
            manager.requests.lock().map_err(|_| failed())?.insert(
                saved.request.clone(),
                Accepted {
                    fingerprint: saved.fingerprint.clone(),
                    id: snapshot.id.clone(),
                },
            );
            manager.entries.lock().map_err(|_| failed())?.insert(
                snapshot.id.clone(),
                Transfer::new(saved, snapshot, Some(store.clone())),
            );
        }
        Ok(manager)
    }
    pub fn unavailable() -> Self {
        Self {
            unavailable: true,
            ..Default::default()
        }
    }
    pub fn ensure_ready(&self) -> Result<()> {
        if self.unavailable || self.closing.load(Ordering::SeqCst) {
            Err(err(ErrorCode::TransferUnavailable))
        } else {
            Ok(())
        }
    }
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
        self.ensure_ready()?;
        let entries = self.entries.lock().map_err(|_| failed())?;
        if entries.len() >= 64
            || entries
                .values()
                .filter(|t| {
                    matches!(
                        t.snapshot().status,
                        TransferStatus::Active
                            | TransferStatus::Stopping
                            | TransferStatus::Resuming
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
            limits,
            raw,
            meta,
            handles,
            mode,
            request,
            fingerprint,
            port,
            dht,
            trackers,
            seed_after_download,
            initial_peer,
            output,
            root,
            owner,
        } = input;
        let snapshot = TransferSnapshot {
            limits,
            id: Uuid::new_v4().to_string(),
            name: meta.name,
            info_hash: meta.hash,
            mode,
            status: TransferStatus::Resuming,
            total_bytes: total(&inspect(&raw)?).to_string(),
            verified_bytes: "0".into(),
            uploaded_bytes: "0".into(),
            received_bytes: "0".into(),
            seed_after_download,
            is_seeding: false,
            telemetry: None,
            listen_port: None,
            dht,
            tracker_count: trackers.len() as u32,
            output_name: output,
            error: None,
        };
        let saved = Saved {
            version: 1,
            raw,
            request: request.clone(),
            fingerprint: fingerprint.clone(),
            recipe: Recipe {
                limits,
                root,
                owner,
                mode,
                port,
                dht,
                trackers,
                seed_after_download,
                initial_peer,
            },
        };
        let entry = Transfer::new(saved, snapshot.clone(), self.store.clone());
        entry.persist(true).await?;
        self.entries
            .lock()
            .map_err(|_| failed())?
            .insert(snapshot.id.clone(), entry.clone());
        self.requests.lock().map_err(|_| failed())?.insert(
            request,
            Accepted {
                fingerprint,
                id: snapshot.id.clone(),
            },
        );
        self.run(entry, Some(handles)).await
    }
    async fn run(
        self: &Arc<Self>,
        entry: Arc<Transfer>,
        prepared: Option<Handles>,
    ) -> Result<TransferSnapshot> {
        let cancel = self.cancel.child_token();
        *entry.operation.lock().map_err(|_| failed())? = Some(cancel.clone());
        let _finish = entry.finish_gate.lock().await;
        let generation = entry.generation.fetch_add(1, Ordering::SeqCst) + 1;
        {
            let mut s = entry.snapshot.lock().map_err(|_| failed())?;
            s.status = TransferStatus::Resuming;
            s.error = None;
            s.telemetry = None;
            s.is_seeding = false;
            s.listen_port = None;
            s.verified_bytes = "0".into();
        }
        let result = async {
            self.ensure_ready()?;
            entry.persist(false).await?;
            let saved = entry.saved.lock().map_err(|_| failed())?.clone();
            let handles = match prepared {
                Some(v) => v,
                None => {
                    let saved = saved.clone();
                    tokio::task::spawn_blocking(move || {
                        let meta = inspect(&saved.raw)?;
                        match saved.recipe.mode {
                            TransferMode::Seed => Handles::seed(&saved.recipe.root, &meta)
                                .map_err(|_| err(ErrorCode::SourceUnavailable)),
                            TransferMode::Download => {
                                persistence::owned(
                                    &saved.recipe.root,
                                    saved.recipe.owner.as_ref().ok_or_else(failed)?,
                                    &meta.hash,
                                )?;
                                Handles::resume(&saved.recipe.root, &meta)
                                    .map_err(|_| err(ErrorCode::SourceUnavailable))
                            }
                        }
                    })
                    .await
                    .map_err(|_| failed())??
                }
            };
            if cancel.is_cancelled() {
                return Err(err(ErrorCode::TransferUnavailable));
            }
            self.activate(&entry, &saved, handles, &cancel).await
        }
        .await;
        *entry.operation.lock().map_err(|_| failed())? = None;
        match result {
            Ok(runtime) => {
                {
                    let mut slot = entry.runtime.lock().map_err(|_| failed())?;
                    let mut snapshot = entry.snapshot.lock().map_err(|_| failed())?;
                    snapshot.status = TransferStatus::Active;
                    snapshot.listen_port = runtime.session.listen_addr().map(|p| p.port());
                    snapshot.is_seeding = snapshot.mode == TransferMode::Seed;
                    runtime.observe(&mut snapshot);
                    *slot = Some(runtime);
                }
                if let Err(e) = entry.persist(false).await {
                    drop(_finish);
                    let _ = entry.finish(TransferStatus::Failed).await;
                    return Err(e);
                }
                Self::monitor(entry.clone(), generation);
                Ok(entry.snapshot())
            }
            Err(e) => {
                {
                    let mut s = entry.snapshot.lock().map_err(|_| failed())?;
                    s.status = if cancel.is_cancelled() {
                        TransferStatus::Paused
                    } else {
                        TransferStatus::Failed
                    };
                    s.error = if s.status == TransferStatus::Failed {
                        Some(e.clone())
                    } else {
                        None
                    };
                }
                entry.persist(false).await?;
                if cancel.is_cancelled() {
                    Ok(entry.snapshot())
                } else {
                    Err(e)
                }
            }
        }
    }
    async fn activate(
        self: &Arc<Self>,
        entry: &Transfer,
        saved: &Saved,
        handles: Handles,
        cancel: &tokio_util::sync::CancellationToken,
    ) -> Result<Runtime> {
        let r = &saved.recipe;
        let session_token = cancel.child_token();
        let session = Session::new_with_opts(
            PathBuf::new(),
            SessionOptions {
                ratelimits: engine_limits(r.limits),
                cancellation_token: Some(session_token.clone()),
                dht: self.dht_config(r.dht),
                disable_trackers: r.trackers.is_empty(),
                disable_local_service_discovery: true,
                persistence: None,
                fastresume: false,
                ipv4_only: true,
                listen: Some(ListenerOptions {
                    listen_addr: SocketAddr::V4(SocketAddrV4::new(self.listen_ip, r.port)),
                    ipv4_only: true,
                    announce_port: ((r.dht || !r.trackers.is_empty())
                        && self.listen_ip.is_loopback()
                        && r.port != 0)
                        .then_some(r.port),
                    enable_upnp_port_forwarding: false,
                    ..Default::default()
                }),
                peer_limit: Some(8),
                ..Default::default()
            },
        )
        .await
        .map_err(|_| {
            session_token.cancel();
            failed()
        })?;
        let result = async {
            let handle = session
                .add_torrent(
                    AddTorrent::from_bytes(ever::route(&saved.raw, &r.trackers)?),
                    Some(AddTorrentOptions {
                        paused: true,
                        overwrite: true,
                        disable_trackers: r.trackers.is_empty(),
                        storage_factory: Some(handles.clone().boxed()),
                        initial_peers: r.initial_peer.map(|p| vec![p]),
                        peer_limit: Some(8),
                        ..Default::default()
                    }),
                )
                .await
                .map_err(|_| failed())?
                .into_handle()
                .ok_or_else(failed)?;
            tokio::select! {
                _=cancel.cancelled()=>return Err(err(ErrorCode::TransferUnavailable)),
                initialized=handle.wait_until_initialized()=>initialized.map_err(|_|failed())?,
            }
            if r.mode == TransferMode::Seed && !handle.stats().finished {
                return Err(err(ErrorCode::SourceChanged));
            }
            {
                let mut stored = entry.saved.lock().map_err(|_| failed())?;
                stored.recipe.port = session.listen_addr().ok_or_else(failed)?.port();
            }
            entry.persist(true).await?;
            self.ensure_ready()?;
            if cancel.is_cancelled() {
                return Err(err(ErrorCode::TransferUnavailable));
            }
            session.unpause(&handle).await.map_err(|_| failed())?;
            Ok(handle)
        }
        .await;
        match result {
            Ok(handle) => {
                let snapshot = entry.snapshot.lock().map_err(|_| failed())?;
                Ok(Runtime {
                    session,
                    handle,
                    io: handles,
                    sample: Mutex::new(RateSample::default()),
                    base_received: snapshot.received_bytes.parse().map_err(|_| failed())?,
                    base_uploaded: snapshot.uploaded_bytes.parse().map_err(|_| failed())?,
                })
            }
            Err(e) => {
                session.stop().await;
                Err(e)
            }
        }
    }
    fn monitor(entry: Arc<Transfer>, generation: u64) {
        tokio::spawn(async move {
            let mut checkpoint = Instant::now();
            loop {
                tokio::time::sleep(Duration::from_millis(250)).await;
                if entry.generation.load(Ordering::SeqCst) != generation {
                    return;
                }
                let state = {
                    let rt = entry.runtime.lock().unwrap_or_else(|e| e.into_inner());
                    rt.as_ref().map(|r| {
                        let s = r.handle.stats();
                        (
                            s.finished,
                            s.error.is_some()
                                || matches!(s.state, librqbit::TorrentStatsState::Error),
                        )
                    })
                };
                match state {
                    None => return,
                    Some((_, true)) => {
                        let _ = entry
                            .finish_inner(TransferStatus::Failed, Some(generation))
                            .await;
                        return;
                    }
                    Some((true, _)) => {
                        let snapshot = entry.snapshot();
                        if snapshot.mode == TransferMode::Download {
                            if snapshot.seed_after_download {
                                if !entry.continue_seeding(generation).await {
                                    let _ = entry
                                        .finish_inner(TransferStatus::Failed, Some(generation))
                                        .await;
                                    return;
                                }
                            } else {
                                let _ = entry
                                    .finish_inner(TransferStatus::Completed, Some(generation))
                                    .await;
                                return;
                            }
                        }
                    }
                    _ => {}
                }
                if checkpoint.elapsed() >= Duration::from_secs(1) {
                    let guard = entry.finish_gate.lock().await;
                    if entry.generation.load(Ordering::SeqCst) != generation {
                        return;
                    }
                    let result = entry.persist(false).await;
                    drop(guard);
                    if result.is_err() {
                        let _ = entry
                            .finish_inner(TransferStatus::Failed, Some(generation))
                            .await;
                        return;
                    }
                    checkpoint = Instant::now();
                }
            }
        });
    }
    pub async fn seed(
        self: &Arc<Self>,
        db: Arc<Database>,
        imports: Arc<ImportState>,
        request: StartSeedRequest,
    ) -> Result<TransferSnapshot> {
        let _gate = self.gate.lock().await;
        validate_limits(request.limits)?;
        let trackers = trackers::normalize(&request.trackers)?;
        let fingerprint = format!(
            "seed:{}:{}:{}:{trackers:?}",
            request.item_id, request.listen_port, request.dht
        );
        let fingerprint = if request.limits == BandwidthLimits::default() {
            fingerprint
        } else {
            format!(
                "{fingerprint}:limits:{}:{}",
                request.limits.download_bps, request.limits.upload_bps
            )
        };
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
            Ok((created.torrent, meta, storage, PathBuf::from(root)))
        })
        .await
        .map_err(|_| failed())??;
        self.start(StartSession {
            limits: request.limits,
            raw: prepared.0,
            meta: prepared.1,
            handles: prepared.2,
            mode: TransferMode::Seed,
            request: request.request_id,
            fingerprint,
            port: request.listen_port,
            dht: request.dht,
            trackers,
            seed_after_download: false,
            initial_peer: None,
            output: None,
            root: prepared.3,
            owner: None,
        })
        .await
    }
    pub async fn download(
        self: &Arc<Self>,
        request: StartDownloadRequest,
        parent: PathBuf,
    ) -> Result<TransferSnapshot> {
        let _gate = self.gate.lock().await;
        validate_limits(request.limits)?;
        let trackers = trackers::normalize(&request.trackers)?;
        let addr = magnets::initial_peer(&request.peer, request.dht || !trackers.is_empty())?;
        let fingerprint = format!(
            "download:{}:{addr:?}:{}:{trackers:?}:{}",
            request.token, request.dht, request.seed_after_download
        );
        let fingerprint = if request.limits == BandwidthLimits::default() {
            fingerprint
        } else {
            format!(
                "{fingerprint}:limits:{}:{}",
                request.limits.download_bps, request.limits.upload_bps
            )
        };
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
            let owner = persistence::own(&root, &meta.hash)?;
            let handles = Handles::download(&root, &meta).map_err(|_| failed())?;
            Ok((handles, root, owner))
        })
        .await
        .map_err(|_| failed())??;
        let result = self
            .start(StartSession {
                limits: request.limits,
                raw: pending.raw,
                meta: pending.meta,
                handles: storage.0,
                mode: TransferMode::Download,
                request: request.request_id,
                fingerprint,
                port: 0,
                dht: request.dht,
                trackers,
                seed_after_download: request.seed_after_download,
                initial_peer: addr,
                output: Some(output_name),
                root: storage.1,
                owner: Some(storage.2),
            })
            .await?;
        self.pending
            .lock()
            .map_err(|_| failed())?
            .remove(&request.token);
        Ok(result)
    }
    pub async fn set_limits(&self, request: TransferLimitsRequest) -> Result<TransferSnapshot> {
        validate_limits(request.limits)?;
        let _gate = self.gate.lock().await;
        let entry = self.entry(&request.transfer_id)?;
        let _finish = entry.finish_gate.lock().await;
        if matches!(
            entry.snapshot().status,
            TransferStatus::Resuming | TransferStatus::Stopping
        ) {
            return Err(err(ErrorCode::TransferUnavailable));
        }
        let old = entry.saved.lock().map_err(|_| failed())?.recipe.limits;
        entry.saved.lock().map_err(|_| failed())?.recipe.limits = request.limits;
        entry.snapshot.lock().map_err(|_| failed())?.limits = request.limits;
        if let Err(e) = entry.persist(true).await {
            entry.saved.lock().map_err(|_| failed())?.recipe.limits = old;
            entry.snapshot.lock().map_err(|_| failed())?.limits = old;
            return Err(e);
        }
        if let Some(rt) = entry.runtime.lock().map_err(|_| failed())?.as_ref() {
            let config = engine_limits(request.limits);
            rt.session.ratelimits.set_download_bps(config.download_bps);
            rt.session.ratelimits.set_upload_bps(config.upload_bps);
        }
        Ok(entry.snapshot())
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
        transfer.finish(TransferStatus::Stopped).await?;
        Ok(transfer.snapshot())
    }
    fn entry(&self, id: &str) -> Result<Arc<Transfer>> {
        valid_uuid(id)?;
        self.ensure_ready()?;
        self.entries
            .lock()
            .map_err(|_| failed())?
            .get(id)
            .cloned()
            .ok_or_else(|| err(ErrorCode::NotFound))
    }
    pub async fn pause(&self, id: &str) -> Result<TransferSnapshot> {
        let entry = self.entry(id)?;
        entry.finish(TransferStatus::Paused).await?;
        Ok(entry.snapshot())
    }
    pub async fn resume(self: &Arc<Self>, id: &str) -> Result<TransferSnapshot> {
        let _gate = self.gate.lock().await;
        let entry = self.entry(id)?;
        if entry.snapshot().status == TransferStatus::Active {
            return Ok(entry.snapshot());
        }
        if entry.snapshot().status == TransferStatus::Completed {
            return Err(library::invalid(ErrorField::Request));
        }
        if self
            .entries
            .lock()
            .map_err(|_| failed())?
            .values()
            .filter(|e| {
                matches!(
                    e.snapshot().status,
                    TransferStatus::Active | TransferStatus::Resuming | TransferStatus::Stopping
                )
            })
            .count()
            >= 4
        {
            return Err(limit());
        }
        self.run(entry, None).await
    }
    pub async fn forget(&self, id: &str) -> Result<TransferForgetResult> {
        let _gate = self.gate.lock().await;
        let entry = self.entry(id)?;
        let _finish = entry.finish_gate.lock().await;
        if matches!(
            entry.snapshot().status,
            TransferStatus::Active | TransferStatus::Resuming | TransferStatus::Stopping
        ) {
            return Err(library::invalid(ErrorField::Request));
        }
        if let Some(store) = self.store.clone() {
            let id = id.to_owned();
            tokio::task::spawn_blocking(move || store.delete(&id))
                .await
                .map_err(|_| failed())??;
        }
        self.entries.lock().map_err(|_| failed())?.remove(id);
        self.requests
            .lock()
            .map_err(|_| failed())?
            .retain(|_, a| a.id != id);
        Ok(TransferForgetResult { forgotten: true })
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
            let _ = e.finish(TransferStatus::Paused).await;
        }
    }
}
#[cfg(test)]
mod tests;
