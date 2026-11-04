//! Local-only import. No file is copied, moved, executed, or shared.
use crate::{contracts::*, database::Database};
use rusqlite::{OptionalExtension, params};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File, Metadata},
    io::Read,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, Instant},
};
use uuid::Uuid;

type Result<T> = std::result::Result<T, IpcError>;
fn err(code: ErrorCode) -> IpcError {
    IpcError::new(code)
}
fn db_err(_: rusqlite::Error) -> IpcError {
    err(ErrorCode::DatabaseUnavailable)
}
fn io_err(_: std::io::Error) -> IpcError {
    err(ErrorCode::SourceUnavailable)
}
pub fn invalid(field: ErrorField) -> IpcError {
    IpcError {
        code: ErrorCode::InvalidArgument,
        retryable: false,
        field: Some(field),
    }
}

struct Selection {
    paths: Vec<PathBuf>,
    created: Instant,
}
#[derive(Default)]
pub struct ImportState {
    selections: Mutex<HashMap<String, Selection>>,
    pub gate: Mutex<()>,
    pub worker: Mutex<()>,
}
impl ImportState {
    pub(crate) fn invalidate_selections(&self) {
        self.selections
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }
    pub fn select(&self, paths: Vec<PathBuf>) -> Result<SourceSelection> {
        if paths.is_empty() {
            return Ok(SourceSelection {
                selection_token: None,
                display_names: vec![],
                cancelled: true,
            });
        }
        if paths.len() > 100 {
            return Err(err(ErrorCode::ImportLimitExceeded));
        }
        let display_names = paths
            .iter()
            .map(|p| {
                p.file_name()
                    .unwrap_or(p.as_os_str())
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        let token = Uuid::new_v4().to_string();
        let mut selections = self
            .selections
            .lock()
            .map_err(|_| err(ErrorCode::UnexpectedError))?;
        selections.retain(|_, s| s.created.elapsed() < Duration::from_secs(300));
        if selections.len() >= 16 {
            return Err(err(ErrorCode::ImportLimitExceeded));
        }
        selections.insert(
            token.clone(),
            Selection {
                paths,
                created: Instant::now(),
            },
        );
        Ok(SourceSelection {
            selection_token: Some(token),
            display_names,
            cancelled: false,
        })
    }
    pub fn take(&self, token: &str) -> Result<Vec<PathBuf>> {
        self.selections
            .lock()
            .map_err(|_| err(ErrorCode::UnexpectedError))?
            .remove(token)
            .filter(|s| s.created.elapsed() < Duration::from_secs(300))
            .map(|s| s.paths)
            .ok_or_else(|| invalid(ErrorField::SelectionToken))
    }
}

pub fn validate_request(request: &StartImportRequest) -> Result<()> {
    if !Uuid::parse_str(&request.request_id)
        .is_ok_and(|id| id.get_version_num() == 4 && id.to_string() == request.request_id)
    {
        return Err(invalid(ErrorField::RequestId));
    }
    if Uuid::parse_str(&request.selection_token).is_err() {
        return Err(invalid(ErrorField::SelectionToken));
    }
    Ok(())
}
pub fn existing(db: &Database, request: &StartImportRequest) -> Result<Option<TaskAccepted>> {
    let c = db
        .connection
        .lock()
        .map_err(|_| err(ErrorCode::DatabaseUnavailable))?;
    let row: Option<(i64, String, String)> = c
        .query_row(
            "SELECT id, status, selection_token FROM tasks WHERE request_id = ?1",
            [&request.request_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .map_err(db_err)?;
    row.map(|(id, status, token)| {
        if token != request.selection_token {
            return Err(invalid(ErrorField::RequestId));
        }
        Ok(TaskAccepted {
            task_id: id.to_string(),
            status: parse_enum(&status)?,
        })
    })
    .transpose()
}
fn parse_enum<T: serde::de::DeserializeOwned>(s: &str) -> Result<T> {
    serde_json::from_value(serde_json::Value::String(s.into()))
        .map_err(|_| err(ErrorCode::InvalidResponse))
}
pub fn create_task(db: &Database, request: &StartImportRequest) -> Result<TaskAccepted> {
    let c = db
        .connection
        .lock()
        .map_err(|_| err(ErrorCode::DatabaseUnavailable))?;
    c.execute("INSERT INTO tasks (kind, request_id, selection_token, updated_at) VALUES ('import', ?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))", params![request.request_id, request.selection_token]).map_err(db_err)?;
    Ok(TaskAccepted {
        task_id: c.last_insert_rowid().to_string(),
        status: TaskStatus::Queued,
    })
}
fn update(
    db: &Database,
    id: &str,
    status: &str,
    processed: usize,
    total: Option<usize>,
    error: Option<ErrorCode>,
) -> Result<()> {
    let code = error.map(|code| {
        serde_json::to_value(code)
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned()
    });
    db.connection.lock().map_err(|_| err(ErrorCode::DatabaseUnavailable))?.execute(
        "UPDATE tasks SET status=?2, processed_count=?3, total_count=?4, error_code=?5, revision=revision+1, updated_at=strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), started_at=coalesce(started_at,strftime('%Y-%m-%dT%H:%M:%fZ', 'now')), finished_at=CASE WHEN ?2 IN ('failed','succeeded') THEN strftime('%Y-%m-%dT%H:%M:%fZ', 'now') ELSE NULL END WHERE id=?1",
        params![id,status,processed as i64,total.map(|v| v as i64),code]).map_err(db_err)?;
    Ok(())
}
pub fn run_import(db: &Database, id: &str, roots: Vec<PathBuf>) -> Result<()> {
    run_import_observed(db, id, roots, &mut || {})
}

// The observer also permits deterministic concurrency tests without huge fixtures
// or timing-dependent sleeps. Production uses a no-op, optimized away by Rust.
fn run_import_observed(
    db: &Database,
    id: &str,
    roots: Vec<PathBuf>,
    after_chunk: &mut impl FnMut(),
) -> Result<()> {
    let mut processed = 0;
    let mut total = None;
    let result = (|| {
        update(db, id, "running", 0, None, None)?;
        let mut seen = HashSet::new();
        let mut batches = Vec::new();
        let mut count = 0;
        for root in roots {
            reject_ancestors(&root)?;
            let root = root.canonicalize().map_err(io_err)?;
            reject_ancestors(&root)?;
            if !seen.insert(root.clone()) {
                continue;
            }
            let mut entries = Vec::new();
            collect(&root, &root, &mut entries, &mut count, 0)?;
            if entries.is_empty() {
                return Err(err(ErrorCode::SourceUnavailable));
            }
            batches.push((root, entries));
        }
        total = Some(count);
        update(db, id, "running", 0, total, None)?;
        let mut imports = Vec::new();
        // Reuse one bounded allocation across every file in this import.
        let mut buffer = vec![0u8; 1024 * 1024];
        let mut last_progress = Instant::now();
        for (root, entries) in batches {
            let mut files = Vec::new();
            for path in entries {
                let (size, hash, signature) =
                    hash_file_with_buffer(&path, &mut buffer, after_chunk)?;
                let relative = if root.is_file() {
                    path.file_name().map(Path::new).unwrap().to_path_buf()
                } else {
                    path.strip_prefix(&root)
                        .map_err(|_| err(ErrorCode::SourceUnavailable))?
                        .to_path_buf()
                };
                files.push(ImportedFile {
                    path: text_path(&path)?,
                    relative: text_path(&relative)?.replace('\\', "/"),
                    size,
                    hash,
                    signature,
                });
                processed += 1;
                // Avoid an SQLite FULL-sync write for every tiny file. Final
                // progress is always stored atomically with catalog completion.
                if last_progress.elapsed() >= Duration::from_millis(200) {
                    update(db, id, "running", processed, total, None)?;
                    last_progress = Instant::now();
                }
            }
            imports.push((root, files));
        }
        commit_import(db, id, imports, processed)
    })();
    if let Err(ref error) = result {
        update(db, id, "failed", processed, total, Some(error.code))?;
    }
    result
}
fn text_path(path: &Path) -> Result<String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| err(ErrorCode::SourceUnavailable))
}
fn is_link(m: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        m.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        m.file_type().is_symlink()
    }
}
fn reject_link(path: &Path) -> Result<Metadata> {
    let m = fs::symlink_metadata(path).map_err(io_err)?;
    if is_link(&m) {
        return Err(err(ErrorCode::SourceUnavailable));
    }
    Ok(m)
}
fn reject_ancestors(path: &Path) -> Result<()> {
    for ancestor in path.ancestors() {
        reject_link(ancestor)?;
    }
    Ok(())
}
fn collect(
    root: &Path,
    path: &Path,
    files: &mut Vec<PathBuf>,
    count: &mut usize,
    depth: usize,
) -> Result<()> {
    if depth > 128 {
        return Err(err(ErrorCode::ImportLimitExceeded));
    }
    let m = reject_link(path)?;
    if m.is_file() {
        let canonical = path.canonicalize().map_err(io_err)?;
        if !canonical.starts_with(root) {
            return Err(err(ErrorCode::SourceUnavailable));
        }
        *count += 1;
        if *count > 100_000 {
            return Err(err(ErrorCode::ImportLimitExceeded));
        }
        files.push(canonical);
    } else if m.is_dir() {
        let mut children: Vec<_> = fs::read_dir(path)
            .map_err(io_err)?
            .map(|v| v.map(|e| e.path()).map_err(io_err))
            .collect::<Result<_>>()?;
        children.sort();
        for child in children {
            collect(root, &child, files, count, depth + 1)?;
        }
    } else {
        return Err(err(ErrorCode::SourceUnavailable));
    }
    Ok(())
}
struct ImportedFile {
    path: String,
    relative: String,
    size: i64,
    hash: String,
    signature: Option<crate::identification::Signature>,
}
#[cfg(test)]
fn hash_file(path: &Path) -> Result<(i64, String, Option<crate::identification::Signature>)> {
    hash_file_with_buffer(path, &mut vec![0u8; 1024 * 1024], &mut || {})
}

fn hash_file_with_buffer(
    path: &Path,
    buffer: &mut [u8],
    after_chunk: &mut impl FnMut(),
) -> Result<(i64, String, Option<crate::identification::Signature>)> {
    reject_ancestors(path)?;
    let mut options = File::options();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(1).custom_flags(0x00200000);
    }
    let mut file = options.open(path).map_err(io_err)?;
    let before = file.metadata().map_err(io_err)?;
    if is_link(&before) || !before.is_file() {
        return Err(err(ErrorCode::SourceUnavailable));
    }
    let size = i64::try_from(before.len()).map_err(|_| err(ErrorCode::ImportLimitExceeded))?;
    let mut hash = Sha256::new();
    let mut bytes = 0u64;
    let mut prefix = Vec::with_capacity((size as usize).min(crate::identification::PREFIX_BYTES));
    loop {
        let n = file.read(buffer).map_err(io_err)?;
        if n == 0 {
            break;
        }
        let keep = (crate::identification::PREFIX_BYTES - prefix.len()).min(n);
        prefix.extend_from_slice(&buffer[..keep]);
        hash.update(&buffer[..n]);
        bytes += n as u64;
        after_chunk();
    }
    let after = file.metadata().map_err(io_err)?;
    if bytes != before.len()
        || after.len() != before.len()
        || after.modified().map_err(io_err)? != before.modified().map_err(io_err)?
    {
        return Err(err(ErrorCode::SourceChanged));
    }
    Ok((
        size,
        format!("{:x}", hash.finalize()),
        crate::identification::signature(&prefix),
    ))
}
fn commit_import(
    db: &Database,
    task: &str,
    imports: Vec<(PathBuf, Vec<ImportedFile>)>,
    count: usize,
) -> Result<()> {
    let mut c = db
        .connection
        .lock()
        .map_err(|_| err(ErrorCode::DatabaseUnavailable))?;
    let tx = c.transaction().map_err(db_err)?;
    for (root, files) in imports {
        let source = text_path(&root)?;
        let existing: Option<i64> = tx
            .query_row(
                "SELECT item_id FROM import_sources WHERE source_key=?1",
                [&source],
                |r| r.get(0),
            )
            .optional()
            .map_err(db_err)?;
        let item = if let Some(id) = existing {
            tx.execute("DELETE FROM collection_item_files WHERE item_id=?1", [id])
                .map_err(db_err)?;
            id
        } else {
            let title = root
                .file_name()
                .unwrap_or(root.as_os_str())
                .to_str()
                .ok_or_else(|| err(ErrorCode::SourceUnavailable))?;
            tx.execute("INSERT INTO collection_items (title) VALUES (?1)", [title])
                .map_err(db_err)?;
            let id = tx.last_insert_rowid();
            tx.execute(
                "INSERT INTO import_sources VALUES (?1,?2)",
                params![source, id],
            )
            .map_err(db_err)?;
            id
        };
        for f in files {
            tx.execute(
                "INSERT INTO files (sha256,size_bytes) VALUES (?1,?2) ON CONFLICT DO NOTHING",
                params![f.hash, f.size],
            )
            .map_err(db_err)?;
            let file_id: i64 = tx
                .query_row(
                    "SELECT id FROM files WHERE sha256=?1 AND size_bytes=?2",
                    params![f.hash, f.size],
                    |r| r.get(0),
                )
                .map_err(db_err)?;
            let detected_format = f.signature.map_or(FileFormat::Unknown, |s| s.format);
            let detected_platform = f
                .signature
                .and_then(|s| s.platform)
                .map(crate::identification::wire);
            tx.execute("INSERT INTO file_identifications(file_id,format_id,platform_id,detector_version) VALUES (?1,?2,?3,?4) ON CONFLICT(file_id) DO UPDATE SET format_id=excluded.format_id,platform_id=excluded.platform_id,detector_version=excluded.detector_version",params![file_id,crate::identification::wire(detected_format),detected_platform,crate::identification::DETECTOR_VERSION]).map_err(db_err)?;
            tx.execute("INSERT INTO file_locations (file_id,path,path_key,verification_status,last_verified_at) VALUES (?1,?2,?2,'verified',strftime('%Y-%m-%dT%H:%M:%fZ','now')) ON CONFLICT(path_key) DO UPDATE SET file_id=excluded.file_id, verification_status='verified', last_verified_at=excluded.last_verified_at",params![file_id,f.path]).map_err(db_err)?;
            tx.execute(
                "INSERT INTO collection_item_files VALUES (?1,?2,?3)",
                params![item, file_id, f.relative],
            )
            .map_err(db_err)?;
        }
        tx.execute("UPDATE collection_items SET updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",[item]).map_err(db_err)?;
    }
    tx.execute("UPDATE tasks SET status='succeeded', processed_count=?2,total_count=?2,revision=revision+1,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now'),finished_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",params![task,count as i64]).map_err(db_err)?;
    tx.commit().map_err(db_err)
}

pub fn task_get(db: &Database, id: &str) -> Result<TaskSnapshot> {
    let id = positive_id(id, ErrorField::TaskId)?;
    let c = db
        .connection
        .lock()
        .map_err(|_| err(ErrorCode::DatabaseUnavailable))?;
    let row = c.query_row("SELECT id,kind,status,processed_count,total_count,revision,updated_at,error_code FROM tasks WHERE id=?1",[id],task_row).optional().map_err(db_err)?.ok_or_else(||err(ErrorCode::NotFound))?;
    decode_task(row)
}
type TaskRow = (
    i64,
    String,
    String,
    i64,
    Option<i64>,
    i64,
    String,
    Option<String>,
);
fn task_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<TaskRow> {
    Ok((
        r.get(0)?,
        r.get(1)?,
        r.get(2)?,
        r.get(3)?,
        r.get(4)?,
        r.get(5)?,
        r.get(6)?,
        r.get(7)?,
    ))
}
fn decode_task(r: TaskRow) -> Result<TaskSnapshot> {
    Ok(TaskSnapshot {
        id: r.0.to_string(),
        kind: parse_enum(&r.1)?,
        status: parse_enum(&r.2)?,
        processed_count: r.3.to_string(),
        total_count: r.4.map(|n| n.to_string()),
        revision: r.5.to_string(),
        updated_at: r.6,
        error: r.7.map(|s| parse_enum(&s).map(IpcError::new)).transpose()?,
    })
}
fn positive_id(s: &str, field: ErrorField) -> Result<i64> {
    s.parse::<i64>()
        .ok()
        .filter(|n| *n > 0 && n.to_string() == s)
        .ok_or_else(|| invalid(field))
}
fn page_cursor(page: &PageRequest, scope: &str) -> Result<i64> {
    if !(1..=100).contains(&page.limit) {
        return Err(invalid(ErrorField::Limit));
    }
    match &page.cursor {
        None => Ok(0),
        Some(s) => s
            .strip_prefix(&format!("{scope}:"))
            .ok_or_else(|| invalid(ErrorField::Cursor))
            .and_then(|id| positive_id(id, ErrorField::Cursor)),
    }
}
pub fn tasks_list(db: &Database, page: PageRequest) -> Result<TaskPage> {
    let cursor = page_cursor(&page, "tasks-v1")?;
    let c = db
        .connection
        .lock()
        .map_err(|_| err(ErrorCode::DatabaseUnavailable))?;
    let mut stmt = c.prepare("SELECT id,kind,status,processed_count,total_count,revision,updated_at,error_code FROM tasks WHERE (?1=0 OR id<?1) ORDER BY id DESC LIMIT ?2").map_err(db_err)?;
    let mut items = stmt
        .query_map(params![cursor, i64::from(page.limit) + 1], task_row)
        .map_err(db_err)?
        .map(|r| decode_task(r.map_err(db_err)?))
        .collect::<Result<Vec<_>>>()?;
    let more = items.len() > page.limit as usize;
    items.truncate(page.limit as usize);
    let next_cursor = if more {
        items.last().map(|v| format!("tasks-v1:{}", v.id))
    } else {
        None
    };
    Ok(TaskPage { items, next_cursor })
}
pub fn library_list(db: &Database, request: LibraryListRequest) -> Result<LibraryPage> {
    let query = request.query.unwrap_or_default().trim().to_owned();
    if query.chars().count() > 200 {
        return Err(invalid(ErrorField::Request));
    }
    let scope = format!("library-v1-{:x}", Sha256::digest(query.as_bytes()));
    let cursor = page_cursor(&request.page, &scope)?;
    let c = db
        .connection
        .lock()
        .map_err(|_| err(ErrorCode::DatabaseUnavailable))?;
    let mut stmt=c.prepare("SELECT id,title,platform FROM collection_items WHERE id>?1 AND (?2='' OR instr(lower(title),lower(?2))>0) ORDER BY id LIMIT ?3").map_err(db_err)?;
    let rows = stmt
        .query_map(
            params![cursor, query, i64::from(request.page.limit) + 1],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .map_err(db_err)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(db_err)?;
    let mut items = Vec::new();
    for (id, title, platform) in rows {
        let (file_count, total_bytes, formats, detected_platforms) =
            crate::identification::summary(&c, id)?;
        items.push(LibraryItemSummary {
            id: id.to_string(),
            title,
            platform,
            file_count,
            total_bytes,
            formats,
            detected_platforms,
        });
    }
    let more = items.len() > request.page.limit as usize;
    items.truncate(request.page.limit as usize);
    let next_cursor = if more {
        items.last().map(|v| format!("{scope}:{}", v.id))
    } else {
        None
    };
    Ok(LibraryPage { items, next_cursor })
}

#[cfg(test)]
mod tests;
