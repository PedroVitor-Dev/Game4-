//! Opt-in local Guardian storage adapter. No listener or peer discovery is started.
use super::{
    local,
    ports::{FragmentReader, StorageNode},
    types::*,
};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use ts_rs::TS;

const MAX_FILES: usize = 100_000;
const MAX_QUOTA: u64 = 1 << 40;
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GuardianConfig {
    pub active: bool,
    pub quota_bytes: String,
    pub upload_bps: u32,
    pub download_bps: u32,
}
impl Default for GuardianConfig {
    fn default() -> Self {
        Self {
            active: false,
            quota_bytes: (10_u64 << 30).to_string(),
            upload_bps: 0,
            download_bps: 0,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GuardianEntry {
    #[ts(type = "number")]
    pub id: i64,
    pub content_id: String,
    pub name: String,
    pub group: String,
    pub index: u8,
    pub sha256: String,
    pub size_bytes: u32,
    pub sharing: bool,
    pub health: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GuardianSnapshot {
    pub node_id: String,
    pub directory: String,
    pub config: GuardianConfig,
    pub used_bytes: String,
    pub fragment_count: u32,
    pub health: String,
    pub entries: Vec<GuardianEntry>,
    pub offset: u32,
    pub total: u32,
    pub network_started: bool,
}
struct Inner {
    db: Connection,
}
struct Gate {
    active: AtomicBool,
    upload: Mutex<Instant>,
    download: Mutex<Instant>,
}
pub struct GuardianNode {
    root: PathBuf,
    node: NodeId,
    inner: Arc<Mutex<Inner>>,
    gate: Arc<Gate>,
    _lock: Arc<File>,
}
fn invalid() -> Error {
    Error::InvalidManifest
}
fn storage() -> Error {
    Error::GuardianUnavailable
}
fn guard(path: &Path) -> Result<()> {
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(_) => return crate::library::reject_ancestors(ancestor).map_err(|_| storage()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Err(storage())
}
fn open(path: &Path) -> Result<File> {
    crate::library::open_source(path).map_err(|_| storage())
}
fn sql<T>(v: rusqlite::Result<T>) -> Result<T> {
    v.map_err(|_| storage())
}
fn decimal(value: &str) -> Result<u64> {
    value
        .parse::<u64>()
        .ok()
        .filter(|v| v.to_string() == value)
        .ok_or_else(invalid)
}
fn digest(value: &str) -> Result<Sha256Digest> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid());
    }
    let mut out = [0; 32];
    for (i, v) in out.iter_mut().enumerate() {
        *v = u8::from_str_radix(&value[i * 2..i * 2 + 2], 16).map_err(|_| invalid())?;
    }
    Ok(Sha256Digest(out))
}
fn config(db: &Connection) -> Result<GuardianConfig> {
    let raw: String = sql(db.query_row("SELECT config FROM node WHERE id=1", [], |r| r.get(0)))?;
    serde_json::from_str(&raw).map_err(|_| storage())
}
fn quota(c: &GuardianConfig) -> Result<u64> {
    let n = decimal(&c.quota_bytes)?;
    if n > MAX_QUOTA || c.upload_bps > 1_000_000_000 || c.download_bps > 1_000_000_000 {
        return Err(invalid());
    }
    Ok(n)
}
fn allowed(db: &Connection, f: &FragmentDescriptor, sharing: bool) -> Result<bool> {
    sql(db.query_row("SELECT EXISTS(SELECT 1 FROM approvals WHERE hash=?1 AND bytes=?2 AND group_idx=?3 AND shard=?4 AND (?5=0 OR sharing=1))", params![local::hex(f.sha256),f.bytes,f.coordinate.stripe as i64,f.coordinate.shard,sharing],|r|r.get(0)))
}
fn check_active(gate: &Gate, ctx: &OperationContext) -> Result<()> {
    ctx.checkpoint()?;
    if !gate.active.load(Ordering::Acquire) {
        return Err(Error::GuardianDisabled);
    }
    Ok(())
}
fn pace(gate: &Gate, upload: bool, bytes: usize, bps: u32, ctx: &OperationContext) -> Result<()> {
    check_active(gate, ctx)?;
    if bps == 0 {
        return Ok(());
    }
    let clock = if upload { &gate.upload } else { &gate.download };
    let target = {
        let mut next = clock.lock().map_err(|_| storage())?;
        *next =
            (*next).max(Instant::now()) + Duration::from_secs_f64(bytes as f64 / f64::from(bps));
        *next
    };
    while Instant::now() < target {
        check_active(gate, ctx)?;
        std::thread::sleep(
            target
                .saturating_duration_since(Instant::now())
                .min(Duration::from_millis(20)),
        );
    }
    check_active(gate, ctx)
}
fn verify(file: &mut File, expected: &FragmentDescriptor, ctx: &OperationContext) -> Result<()> {
    let mut h = Sha256::new();
    let mut left = u64::from(expected.bytes);
    let mut buf = [0; 65536];
    while left > 0 {
        ctx.checkpoint()?;
        let n = file.read(&mut buf[..left.min(65536) as usize])?;
        if n == 0 {
            return Err(Error::IntegrityMismatch);
        }
        h.update(&buf[..n]);
        left -= n as u64;
    }
    if file.read(&mut [0])? != 0 || h.finalize().as_slice() != expected.sha256.0 {
        return Err(Error::IntegrityMismatch);
    }
    Ok(())
}
impl GuardianNode {
    /// The selected directory hosts a dedicated child; existing unrelated files are never adopted.
    pub fn open(directory: &Path) -> Result<Self> {
        if !directory.is_absolute() {
            return Err(invalid());
        }
        guard(directory)?;
        fs::create_dir_all(directory)?;
        guard(directory)?;
        let root = directory.join("Game4-Guardian-v1");
        if !root.exists() {
            fs::create_dir(&root)?;
        }
        guard(&root)?;
        let marker = root.join("owner.json");
        let node = if marker.exists() {
            let mut f = open(&marker)?;
            let mut s = String::new();
            Read::by_ref(&mut f).take(4097).read_to_string(&mut s)?;
            let v: serde_json::Value = serde_json::from_str(&s).map_err(|_| storage())?;
            if v["format"] != "game4-guardian-store" || v["version"] != 1 {
                return Err(storage());
            }
            digest(v["nodeId"].as_str().ok_or_else(storage)?)?
        } else {
            if fs::read_dir(&root)?.next().is_some() {
                return Err(storage());
            }
            let id = Sha256Digest(Sha256::digest(uuid::Uuid::new_v4().as_bytes()).into());
            let mut f = File::options().write(true).create_new(true).open(&marker)?;
            f.write_all(serde_json::to_string(&serde_json::json!({"format":"game4-guardian-store","version":1,"nodeId":local::hex(id)})).map_err(|_| storage())?.as_bytes())?;
            f.sync_all()?;
            id
        };
        let lock_path = root.join("node.lock");
        guard(&lock_path)?;
        let lock = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)?;
        lock.try_lock().map_err(|_| storage())?;
        for name in ["fragments", "staging"] {
            let p = root.join(name);
            guard(&p)?;
            fs::create_dir_all(&p)?;
            guard(&p)?;
        }
        let db_path = root.join("state.sqlite3");
        for suffix in ["", "-wal", "-shm", "-journal"] {
            guard(&root.join(format!("state.sqlite3{suffix}")))?;
        }
        let db = sql(Connection::open(&db_path))?;
        sql(db.execute_batch("PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL; CREATE TABLE IF NOT EXISTS node(id INTEGER PRIMARY KEY CHECK(id=1),config TEXT NOT NULL); CREATE TABLE IF NOT EXISTS approvals(id INTEGER PRIMARY KEY,content TEXT NOT NULL,name TEXT NOT NULL,group_idx INTEGER NOT NULL,shard INTEGER NOT NULL,hash TEXT NOT NULL,bytes INTEGER NOT NULL,sharing INTEGER NOT NULL,UNIQUE(content,group_idx,shard)); CREATE TABLE IF NOT EXISTS blobs(hash TEXT PRIMARY KEY,bytes INTEGER NOT NULL,health TEXT NOT NULL);"))?;
        sql(db.execute(
            "INSERT OR IGNORE INTO node VALUES(1,?1)",
            [serde_json::to_string(&GuardianConfig::default()).map_err(|_| storage())?],
        ))?;
        let c = config(&db)?;
        quota(&c)?;
        // No sharing resumes on app restart. A user must opt in again each session.
        let mut paused = c;
        paused.active = false;
        sql(db.execute(
            "UPDATE node SET config=?1",
            [serde_json::to_string(&paused).map_err(|_| storage())?],
        ))?;
        let result = Self {
            root,
            node: NodeId(node.0),
            inner: Arc::new(Mutex::new(Inner { db })),
            gate: Arc::new(Gate {
                active: AtomicBool::new(false),
                upload: Mutex::new(Instant::now()),
                download: Mutex::new(Instant::now()),
            }),
            _lock: Arc::new(lock),
        };
        result.reconcile()?;
        Ok(result)
    }
    fn blob(&self, hash: Sha256Digest) -> PathBuf {
        self.root
            .join("fragments")
            .join(format!("{}.g4frag", local::hex(hash)))
    }
    fn used(&self) -> Result<(u64, u32)> {
        let mut used = 0u64;
        let mut count = 0;
        for e in fs::read_dir(self.root.join("fragments"))? {
            let p = e?.path();
            guard(&p)?;
            let m = fs::symlink_metadata(&p)?;
            if !m.is_file() || count >= MAX_FILES {
                return Err(storage());
            }
            used = used.checked_add(m.len()).ok_or_else(storage)?;
            count += 1;
        }
        Ok((used, count as u32))
    }
    fn reconcile(&self) -> Result<()> {
        let inner = self.inner.lock().map_err(|_| storage())?;
        for e in fs::read_dir(self.root.join("staging"))?.take(MAX_FILES + 1) {
            let p = e?.path();
            guard(&p)?;
            if !p.is_file() || p.extension().and_then(|v| v.to_str()) != Some("part") {
                return Err(storage());
            }
            fs::remove_file(p)?;
        }
        sql(inner.db.execute("UPDATE blobs SET health='missing'", []))?;
        self.used()?;
        for e in fs::read_dir(self.root.join("fragments"))? {
            let p = e?.path();
            let name = p.file_stem().and_then(|v| v.to_str()).ok_or_else(storage)?;
            let hash = digest(name)?;
            if p.extension().and_then(|v| v.to_str()) != Some("g4frag") {
                return Err(storage());
            }
            let bytes = open(&p)?.metadata()?.len();
            if !(65536..=4194304).contains(&bytes) {
                return Err(storage());
            }
            // Restart invalidates previous integrity observations. Reads rehash before exposing bytes.
            sql(inner.db.execute("INSERT INTO blobs VALUES(?1,?2,'unknown') ON CONFLICT(hash) DO UPDATE SET bytes=excluded.bytes,health='unknown'",params![local::hex(hash),bytes as i64]))?;
        }
        Ok(())
    }
    pub fn configure(&self, c: GuardianConfig) -> Result<()> {
        let limit = quota(&c)?;
        if !c.active {
            self.gate.active.store(false, Ordering::Release);
        }
        let inner = self.inner.lock().map_err(|_| storage())?;
        if self.used()?.0 > limit {
            return Err(Error::QuotaExceeded);
        }
        sql(inner.db.execute(
            "UPDATE node SET config=?1",
            [serde_json::to_string(&c).map_err(|_| storage())?],
        ))?;
        *self.gate.upload.lock().map_err(|_| storage())? = Instant::now();
        *self.gate.download.lock().map_err(|_| storage())? = Instant::now();
        self.gate.active.store(c.active, Ordering::Release);
        Ok(())
    }
    pub fn approve(
        &self,
        doc: &super::manifest::G4epEverManifest,
        group: u64,
        index: u8,
        sharing: bool,
    ) -> Result<()> {
        let manifest = super::manifest::validate(doc).map_err(|_| invalid())?;
        let f = manifest
            .manifest()
            .fragments
            .iter()
            .find(|f| {
                f.coordinate
                    == FragmentCoordinate {
                        stripe: group,
                        shard: index,
                    }
            })
            .ok_or_else(invalid)?;
        let inner = self.inner.lock().map_err(|_| storage())?;
        let count: u32 = sql(inner
            .db
            .query_row("SELECT count(*) FROM approvals", [], |r| r.get(0)))?;
        if count >= MAX_FILES as u32 {
            return Err(Error::MetadataLimit);
        }
        sql(inner.db.execute("INSERT INTO approvals(content,name,group_idx,shard,hash,bytes,sharing) VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(content,group_idx,shard) DO UPDATE SET sharing=excluded.sharing,hash=excluded.hash,bytes=excluded.bytes,name=excluded.name",params![doc.content_id,doc.manifest.files[0].path,group as i64,index,local::hex(f.sha256),f.bytes,sharing]))?;
        Ok(())
    }
    pub fn import_file(
        &self,
        f: &FragmentDescriptor,
        path: &Path,
        ctx: &OperationContext,
    ) -> Result<StorageReceipt> {
        let mut file = open(path)?;
        self.commit_fragment(f, &mut file, ctx)
    }
    pub fn export_file(
        &self,
        f: &FragmentDescriptor,
        path: &Path,
        ctx: &OperationContext,
    ) -> Result<()> {
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()?.join(path)
        };
        guard(&path)?;
        let mut source = self.read_fragment(f, ctx)?;
        let mut temp = tempfile::NamedTempFile::new_in(path.parent().ok_or_else(storage)?)?;
        std::io::copy(&mut source, &mut temp)?;
        temp.as_file().sync_all()?;
        temp.persist_noclobber(path)
            .map_err(|e| Error::Io(e.error))?;
        Ok(())
    }
    pub fn entry(&self, id: i64) -> Result<FragmentDescriptor> {
        let inner = self.inner.lock().map_err(|_| storage())?;
        let (group, index, hash, bytes): (i64, u8, String, u32) = sql(inner.db.query_row(
            "SELECT group_idx,shard,hash,bytes FROM approvals WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        ))?;
        Ok(FragmentDescriptor {
            coordinate: FragmentCoordinate {
                stripe: group as u64,
                shard: index,
            },
            bytes,
            sha256: digest(&hash)?,
        })
    }
    pub fn revoke(&self, id: i64, delete: bool) -> Result<()> {
        let mut inner = self.inner.lock().map_err(|_| storage())?;
        let hash: String =
            sql(inner
                .db
                .query_row("SELECT hash FROM approvals WHERE id=?1", [id], |r| r.get(0)))?;
        let tx = sql(inner.db.transaction())?;
        sql(tx.execute("DELETE FROM approvals WHERE id=?1", [id]))?;
        let remaining: u32 = sql(tx.query_row(
            "SELECT count(*) FROM approvals WHERE hash=?1",
            [&hash],
            |r| r.get(0),
        ))?;
        if delete && remaining == 0 {
            let p = self.blob(digest(&hash)?);
            guard(&p)?;
            match fs::remove_file(p) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
            sql(tx.execute("DELETE FROM blobs WHERE hash=?1", [hash]))?;
        }
        sql(tx.commit())?;
        Ok(())
    }
    /// Explicit owner action: remove bytes without any remaining admission permission.
    pub fn prune(&self) -> Result<()> {
        let inner = self.inner.lock().map_err(|_| storage())?;
        for e in fs::read_dir(self.root.join("fragments"))? {
            let path = e?.path();
            guard(&path)?;
            if path.extension().and_then(|s| s.to_str()) != Some("g4frag") {
                return Err(storage());
            }
            let hash = path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(storage)?;
            digest(hash)?;
            let retained: bool = sql(inner.db.query_row(
                "SELECT EXISTS(SELECT 1 FROM approvals WHERE hash=?1)",
                [hash],
                |r| r.get(0),
            ))?;
            if !retained {
                fs::remove_file(&path)?;
                sql(inner.db.execute("DELETE FROM blobs WHERE hash=?1", [hash]))?;
            }
        }
        Ok(())
    }
    pub fn snapshot(&self, offset: u32) -> Result<GuardianSnapshot> {
        let inner = self.inner.lock().map_err(|_| storage())?;
        let (used, count) = self.used()?;
        let total: u32 = sql(inner
            .db
            .query_row("SELECT count(*) FROM approvals", [], |r| r.get(0)))?;
        let mut q=sql(inner.db.prepare("SELECT a.id,a.content,a.name,a.group_idx,a.shard,a.hash,a.bytes,a.sharing,coalesce(b.health,'missing') FROM approvals a LEFT JOIN blobs b ON b.hash=a.hash ORDER BY a.id LIMIT 100 OFFSET ?1"))?;
        let entries = sql(q.query_map([offset], |r| {
            Ok(GuardianEntry {
                id: r.get(0)?,
                content_id: r.get(1)?,
                name: r.get(2)?,
                group: r.get::<_, i64>(3)?.to_string(),
                index: r.get(4)?,
                sha256: r.get(5)?,
                size_bytes: r.get(6)?,
                sharing: r.get(7)?,
                health: r.get(8)?,
            })
        }))?
        .collect::<rusqlite::Result<Vec<_>>>();
        let bad: u32 = sql(inner.db.query_row(
            "SELECT count(*) FROM blobs WHERE health IN ('corrupt','missing')",
            [],
            |r| r.get(0),
        ))?;
        Ok(GuardianSnapshot {
            node_id: local::hex(Sha256Digest(self.node.0)),
            directory: self.root.display().to_string(),
            config: {
                let mut c = config(&inner.db)?;
                c.active = self.gate.active.load(Ordering::Acquire);
                c
            },
            used_bytes: used.to_string(),
            fragment_count: count,
            health: if bad > 0 {
                "attention"
            } else if sql(inner.db.query_row(
                "SELECT EXISTS(SELECT 1 FROM blobs WHERE health='unknown')",
                [],
                |r| r.get::<_, bool>(0),
            ))? {
                "unchecked"
            } else {
                "ready"
            }
            .into(),
            entries: sql(entries)?,
            offset,
            total,
            network_started: false,
        })
    }
    pub fn verify_all(&self, ctx: &OperationContext) -> Result<()> {
        let inner = self.inner.lock().map_err(|_| storage())?;
        let records = {
            let mut q = sql(inner
                .db
                .prepare("SELECT hash,bytes FROM blobs LIMIT 100001"))?;
            sql(q.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?))))?
                .collect::<rusqlite::Result<Vec<_>>>()
        };
        let records = sql(records)?;
        if records.len() > MAX_FILES {
            return Err(Error::MetadataLimit);
        }
        for (hash, bytes) in records {
            ctx.checkpoint()?;
            let f = FragmentDescriptor {
                coordinate: FragmentCoordinate {
                    stripe: 0,
                    shard: 0,
                },
                bytes,
                sha256: digest(&hash)?,
            };
            let state =
                match open(&self.blob(f.sha256)).and_then(|mut file| verify(&mut file, &f, ctx)) {
                    Ok(()) => "verified",
                    Err(Error::Cancelled) => return Err(Error::Cancelled),
                    Err(Error::Deadline) => return Err(Error::Deadline),
                    Err(Error::IntegrityMismatch) => "corrupt",
                    Err(_) => "missing",
                };
            sql(inner.db.execute(
                "UPDATE blobs SET health=?1 WHERE hash=?2",
                params![state, hash],
            ))?;
        }
        Ok(())
    }
}
impl StorageNode for GuardianNode {
    fn node_id(&self) -> NodeId {
        self.node
    }
    fn capacity(&self, ctx: &OperationContext) -> Result<StorageCapacity> {
        ctx.checkpoint()?;
        let inner = self.inner.lock().map_err(|_| storage())?;
        Ok(StorageCapacity {
            quota_bytes: quota(&config(&inner.db)?)?,
            used_bytes: self.used()?.0,
            reserved_bytes: 0,
        })
    }
    fn commit_fragment(
        &self,
        f: &FragmentDescriptor,
        input: &mut dyn Read,
        ctx: &OperationContext,
    ) -> Result<StorageReceipt> {
        check_active(&self.gate, ctx)?;
        let inner = self.inner.lock().map_err(|_| storage())?;
        if !allowed(&inner.db, f, false)? {
            return Err(Error::GuardianNotApproved);
        }
        let c = config(&inner.db)?;
        let dest = self.blob(f.sha256);
        guard(&dest)?;
        let exists = dest.exists();
        if exists {
            let mut existing = open(&dest)?;
            verify(&mut existing, f, ctx)?;
        }
        let (used, count) = self.used()?;
        if count as usize >= MAX_FILES
            || used
                .checked_add(if exists { 0 } else { u64::from(f.bytes) })
                .is_none_or(|n| n > quota(&c).unwrap_or(0))
        {
            return Err(Error::QuotaExceeded);
        }
        let mut stage = tempfile::Builder::new()
            .suffix(".part")
            .tempfile_in(self.root.join("staging"))?;
        let mut h = Sha256::new();
        let mut left = f.bytes as usize;
        let mut buf = [0; 65536];
        while left > 0 {
            check_active(&self.gate, ctx)?;
            let n = input.read(&mut buf[..left.min(65536)])?;
            if n == 0 {
                return Err(Error::IntegrityMismatch);
            }
            pace(&self.gate, false, n, c.download_bps, ctx)?;
            if !exists {
                stage.write_all(&buf[..n])?;
            }
            h.update(&buf[..n]);
            left -= n;
        }
        if input.read(&mut [0])? != 0 || h.finalize().as_slice() != f.sha256.0 {
            return Err(Error::IntegrityMismatch);
        }
        check_active(&self.gate, ctx)?;
        stage.as_file().sync_all()?;
        if !exists {
            stage
                .persist_noclobber(&dest)
                .map_err(|e| Error::Io(e.error))?;
        }
        sql(inner.db.execute("INSERT INTO blobs VALUES(?1,?2,'verified') ON CONFLICT(hash) DO UPDATE SET health='verified'",params![local::hex(f.sha256),f.bytes]))?;
        Ok(StorageReceipt {
            node: self.node,
            sha256: f.sha256,
            bytes: f.bytes,
            observed_at: Instant::now(),
        })
    }
    fn read_fragment(
        &self,
        f: &FragmentDescriptor,
        ctx: &OperationContext,
    ) -> Result<FragmentReader> {
        check_active(&self.gate, ctx)?;
        let inner = self.inner.lock().map_err(|_| storage())?;
        if !allowed(&inner.db, f, true)? {
            return Err(Error::GuardianNotApproved);
        }
        let mut file = open(&self.blob(f.sha256))?;
        if let Err(e) = verify(&mut file, f, ctx) {
            if matches!(e, Error::IntegrityMismatch) {
                sql(inner.db.execute(
                    "UPDATE blobs SET health='corrupt' WHERE hash=?1",
                    [local::hex(f.sha256)],
                ))?;
            }
            return Err(e);
        }
        use std::io::{Seek, SeekFrom};
        file.seek(SeekFrom::Start(0))?;
        sql(inner.db.execute(
            "UPDATE blobs SET health='verified' WHERE hash=?1",
            [local::hex(f.sha256)],
        ))?;
        drop(inner);
        Ok(Box::new(GuardedReader {
            file,
            fragment: f.clone(),
            gate: Arc::clone(&self.gate),
            inner: Arc::clone(&self.inner),
            ctx: ctx.clone(),
            left: f.bytes as usize,
            _lock: Arc::clone(&self._lock),
        }))
    }
    fn inspect(
        &self,
        requested: &[FragmentDescriptor],
        ctx: &OperationContext,
    ) -> Result<Vec<FragmentObservation>> {
        if requested.len() > 100 {
            return Err(Error::MetadataLimit);
        }
        let inner = self.inner.lock().map_err(|_| storage())?;
        requested
            .iter()
            .map(|f| {
                ctx.checkpoint()?;
                if !allowed(&inner.db, f, false)? {
                    return Err(Error::GuardianNotApproved);
                }
                let health = match open(&self.blob(f.sha256))
                    .and_then(|mut file| verify(&mut file, f, ctx))
                {
                    Ok(()) => FragmentHealth::Verified,
                    Err(Error::IntegrityMismatch) => FragmentHealth::Corrupt,
                    Err(Error::Cancelled) => return Err(Error::Cancelled),
                    Err(Error::Deadline) => return Err(Error::Deadline),
                    Err(_) => FragmentHealth::Missing,
                };
                Ok(FragmentObservation {
                    node: self.node,
                    fragment: f.clone(),
                    health,
                    observed_at: Instant::now(),
                })
            })
            .collect()
    }
}
struct GuardedReader {
    file: File,
    fragment: FragmentDescriptor,
    gate: Arc<Gate>,
    inner: Arc<Mutex<Inner>>,
    ctx: OperationContext,
    left: usize,
    _lock: Arc<File>,
}
impl Read for GuardedReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut run = || -> Result<usize> {
            check_active(&self.gate, &self.ctx)?;
            let inner = self.inner.lock().map_err(|_| storage())?;
            if !allowed(&inner.db, &self.fragment, true)? {
                return Err(Error::GuardianNotApproved);
            }
            let bps = config(&inner.db)?.upload_bps;
            drop(inner);
            let max = buf.len().min(65536).min(self.left);
            pace(&self.gate, true, max, bps, &self.ctx)?;
            let inner = self.inner.lock().map_err(|_| storage())?;
            check_active(&self.gate, &self.ctx)?;
            if !allowed(&inner.db, &self.fragment, true)? {
                return Err(Error::GuardianNotApproved);
            }
            let n = self.file.read(&mut buf[..max])?;
            self.left -= n;
            Ok(n)
        };
        run().map_err(std::io::Error::other)
    }
}

#[cfg(test)]
mod tests;
