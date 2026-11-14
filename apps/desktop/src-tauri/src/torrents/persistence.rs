//! Private transfer state, separate from the preservation catalog. No sockets on load.
use super::*;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Owned {
    pub token: String,
    pub created: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Recipe {
    #[serde(default)]
    pub limits: BandwidthLimits,
    pub root: PathBuf,
    pub owner: Option<Owned>,
    pub mode: TransferMode,
    pub port: u16,
    pub dht: bool,
    pub trackers: Vec<String>,
    pub seed_after_download: bool,
    pub initial_peer: Option<SocketAddr>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Saved {
    pub version: u32,
    pub request: String,
    pub fingerprint: String,
    pub recipe: Recipe,
    #[serde(skip)]
    pub raw: Vec<u8>,
}
pub(super) struct Store {
    connection: Mutex<Connection>,
}
fn db_err(_: impl std::fmt::Debug) -> IpcError {
    err(ErrorCode::DatabaseUnavailable)
}
type OpenedStore = (Arc<Store>, Vec<(Saved, TransferSnapshot)>);
impl Store {
    pub fn open(path: &Path) -> Result<OpenedStore> {
        let parent = path.parent().ok_or_else(failed)?;
        std::fs::create_dir_all(parent).map_err(db_err)?;
        library::reject_ancestors(parent)?;
        if path.exists() {
            library::reject_ancestors(path)?;
        }
        let mut c = Connection::open(path).map_err(db_err)?;
        c.busy_timeout(Duration::from_secs(5)).map_err(db_err)?;
        c.pragma_update(None, "foreign_keys", true)
            .map_err(db_err)?;
        c.pragma_update(None, "synchronous", "FULL")
            .map_err(db_err)?;
        let v: i64 = c
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(db_err)?;
        if v == 0 {
            let tx = c.transaction().map_err(db_err)?;
            tx.execute_batch("CREATE TABLE transfers(id TEXT PRIMARY KEY, saved TEXT NOT NULL CHECK(length(saved)<=65536), metainfo BLOB NOT NULL CHECK(length(metainfo)<=8388608), snapshot TEXT NOT NULL CHECK(length(snapshot)<=65536)); PRAGMA user_version=1;").map_err(db_err)?;
            tx.commit().map_err(db_err)?;
        } else if v != 1 {
            return Err(err(ErrorCode::TransferUnavailable));
        }
        let budget: (i64, i64) = c
            .query_row(
                "SELECT count(*),coalesce(sum(length(metainfo)),0) FROM transfers",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(db_err)?;
        if budget.0 > 64 || budget.1 > 64 * 1024 * 1024 {
            return Err(limit());
        }
        // Keep frequent snapshots out of the large metainfo row. Also accept
        // early v1 stores that embedded the initial snapshot in that row.
        let snapshots:i64=c.query_row("SELECT count(*) FROM sqlite_master WHERE type='table' AND name='transfer_snapshots'",[],|r|r.get(0)).map_err(db_err)?;
        if snapshots == 0 {
            let tx = c.transaction().map_err(db_err)?;
            tx.execute_batch("CREATE TABLE transfer_snapshots(id TEXT PRIMARY KEY REFERENCES transfers(id) ON DELETE CASCADE, snapshot TEXT NOT NULL CHECK(length(snapshot)<=65536)); INSERT INTO transfer_snapshots SELECT id,snapshot FROM transfers;").map_err(db_err)?;
            tx.commit().map_err(db_err)?;
        }
        let mut records: Vec<(Saved, TransferSnapshot)> = Vec::new();
        {
            let mut query = c
                .prepare("SELECT t.id,t.saved,t.metainfo,s.snapshot FROM transfers t LEFT JOIN transfer_snapshots s ON t.id=s.id ORDER BY t.id")
                .map_err(db_err)?;
            let rows = query
                .query_map([], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, Vec<u8>>(2)?,
                        r.get::<_, String>(3)?,
                    ))
                })
                .map_err(db_err)?;
            for row in rows {
                let (id, json, raw, snapshot) = row.map_err(db_err)?;
                if json.len() > 65536 || snapshot.len() > 65536 || raw.len() > ever::MAX_METAINFO {
                    return Err(limit());
                }
                let mut saved: Saved = serde_json::from_str(&json).map_err(db_err)?;
                saved.raw = raw;
                let mut s: TransferSnapshot = serde_json::from_str(&snapshot).map_err(db_err)?;
                valid_uuid(&id)?;
                valid_uuid(&saved.request)?;
                let meta = inspect(&saved.raw)?;
                validate_limits(saved.recipe.limits)?;
                if saved.recipe.limits != s.limits
                    || saved.version != 1
                    || s.id != id
                    || s.mode != saved.recipe.mode
                    || s.info_hash != meta.hash
                    || s.total_bytes != total(&meta).to_string()
                    || !saved.recipe.root.is_absolute()
                    || saved.recipe.root.to_string_lossy().len() > 32768
                    || saved.recipe.seed_after_download != s.seed_after_download
                    || trackers::normalize(&saved.recipe.trackers)? != saved.recipe.trackers
                {
                    return Err(failed());
                }
                if s.verified_bytes.parse::<u64>().map_err(db_err)? > total(&meta) {
                    return Err(failed());
                }
                s.received_bytes.parse::<u64>().map_err(db_err)?;
                s.uploaded_bytes.parse::<u64>().map_err(db_err)?;
                if let Some(p) = saved.recipe.initial_peer {
                    peer(&p.to_string())?;
                }
                match (&saved.recipe.owner, s.mode) {
                    (Some(owner), TransferMode::Download) => {
                        valid_uuid(&owner.token)?;
                        if saved.recipe.root.file_name().and_then(|v| v.to_str())
                            != s.output_name.as_deref()
                        {
                            return Err(failed());
                        }
                    }
                    (None, TransferMode::Seed) => {}
                    _ => return Err(failed()),
                }
                if (s.status == TransferStatus::Failed) != s.error.is_some()
                    || (s.status == TransferStatus::Completed
                        && (s.mode != TransferMode::Download || s.verified_bytes != s.total_bytes))
                    || (s.mode == TransferMode::Seed && s.seed_after_download)
                    || s.dht != saved.recipe.dht
                    || s.tracker_count != saved.recipe.trackers.len() as u32
                {
                    return Err(failed());
                }
                if matches!(
                    s.status,
                    TransferStatus::Active
                        | TransferStatus::Resuming
                        | TransferStatus::Stopping
                        | TransferStatus::Paused
                ) {
                    s.status = TransferStatus::Paused;
                    s.error = None;
                }
                s.telemetry = None;
                s.is_seeding = false;
                s.listen_port = None;
                if records.iter().any(|(old, _)| old.request == saved.request) {
                    return Err(failed());
                }
                records.push((saved, s));
            }
        }
        let tx = c.transaction().map_err(db_err)?;
        for (_, s) in &records {
            tx.execute(
                "UPDATE transfer_snapshots SET snapshot=?2 WHERE id=?1",
                params![s.id, serde_json::to_string(s).map_err(db_err)?],
            )
            .map_err(db_err)?;
        }
        tx.commit().map_err(db_err)?;
        Ok((
            Arc::new(Self {
                connection: Mutex::new(c),
            }),
            records,
        ))
    }
    pub fn save(&self, saved: &Saved, snapshot: &TransferSnapshot) -> Result<()> {
        let json = serde_json::to_string(saved).map_err(db_err)?;
        let snapshot_json = serde_json::to_string(snapshot).map_err(db_err)?;
        if json.len() > 65536 || snapshot_json.len() > 65536 || saved.raw.len() > ever::MAX_METAINFO
        {
            return Err(limit());
        }
        let mut c = self.connection.lock().map_err(db_err)?;
        let tx = c.transaction().map_err(db_err)?;
        tx.execute("INSERT INTO transfers VALUES (?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET saved=excluded.saved,metainfo=excluded.metainfo,snapshot=excluded.snapshot",params![snapshot.id,json,saved.raw,snapshot_json]).map_err(db_err)?;
        tx.execute("INSERT INTO transfer_snapshots VALUES (?1,?2) ON CONFLICT(id) DO UPDATE SET snapshot=excluded.snapshot",params![snapshot.id,snapshot_json]).map_err(db_err)?;
        let budget: (i64, i64) = tx
            .query_row(
                "SELECT count(*),coalesce(sum(length(metainfo)),0) FROM transfers",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(db_err)?;
        if budget.0 > 64 || budget.1 > 64 * 1024 * 1024 {
            return Err(limit());
        }
        tx.commit().map_err(db_err)
    }
    #[cfg(test)]
    pub(super) fn connection_for_test(&self, read_only: bool) {
        self.connection
            .lock()
            .unwrap()
            .pragma_update(None, "query_only", read_only)
            .unwrap();
    }
    pub fn delete(&self, id: &str) -> Result<()> {
        self.connection
            .lock()
            .map_err(db_err)?
            .execute("DELETE FROM transfers WHERE id=?1", [id])
            .map_err(db_err)?;
        Ok(())
    }
    pub fn snapshot(&self, s: &TransferSnapshot) -> Result<()> {
        let changed = self
            .connection
            .lock()
            .map_err(db_err)?
            .execute(
                "UPDATE transfer_snapshots SET snapshot=?2 WHERE id=?1",
                params![s.id, serde_json::to_string(s).map_err(db_err)?],
            )
            .map_err(db_err)?;
        if changed != 1 {
            return Err(err(ErrorCode::DatabaseUnavailable));
        }
        Ok(())
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Marker {
    version: u32,
    token: String,
    hash: String,
}
fn marker(root: &Path) -> Result<PathBuf> {
    let name = root
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(failed)?;
    let parent = root.parent().ok_or_else(failed)?;
    Ok(parent.join(format!(".{name}.owner.json")))
}
fn created(root: &Path) -> Result<String> {
    Ok(std::fs::metadata(root)
        .map_err(db_err)?
        .created()
        .map_err(db_err)?
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(db_err)?
        .as_nanos()
        .to_string())
}
pub(super) fn own(root: &Path, hash: &str) -> Result<Owned> {
    let owner = Owned {
        token: Uuid::new_v4().to_string(),
        created: created(root)?,
    };
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(marker(root)?)
        .map_err(db_err)?;
    serde_json::to_writer(
        &mut file,
        &Marker {
            version: 1,
            token: owner.token.clone(),
            hash: hash.into(),
        },
    )
    .map_err(db_err)?;
    file.sync_all().map_err(db_err)?;
    Ok(owner)
}
pub(super) fn owned(root: &Path, owner: &Owned, hash: &str) -> Result<()> {
    library::reject_ancestors(root)?;
    if !root.is_dir() || created(root)? != owner.created {
        return Err(err(ErrorCode::SourceChanged));
    }
    let file = library::open_source(&marker(root)?)?;
    let mut bytes = Vec::new();
    file.take(4097).read_to_end(&mut bytes).map_err(db_err)?;
    if bytes.len() > 4096 {
        return Err(failed());
    }
    let m: Marker = serde_json::from_slice(&bytes).map_err(db_err)?;
    if m.version != 1 || m.token != owner.token || m.hash != hash {
        return Err(err(ErrorCode::SourceChanged));
    }
    Ok(())
}
