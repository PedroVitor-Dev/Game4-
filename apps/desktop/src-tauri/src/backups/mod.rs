//! Private, versioned catalog backups. No backup SQL or referenced file is executed.
use crate::{
    contracts::*,
    database::{Database, migrations},
    library::ImportState,
};
use rusqlite::{
    Connection, ToSql, Transaction,
    types::{FromSql, FromSqlResult, ToSqlOutput, Value, ValueRef},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
    sync::Mutex,
    time::{Duration, Instant},
};
use uuid::Uuid;
const FORMAT: &str = "game4-library-backup";
const VERSION: u32 = 2;
const DATABASE_SCHEMA: u32 = 4;
const MAX_BYTES: usize = 64 * 1024 * 1024;
const MAX_ROWS: usize = 500_000;
type Result<T> = std::result::Result<T, IpcError>;
fn error(code: ErrorCode) -> IpcError {
    IpcError::new(code)
}
fn invalid() -> IpcError {
    error(ErrorCode::BackupInvalid)
}
fn sql_error(_: rusqlite::Error) -> IpcError {
    invalid()
}

// Decimal strings preserve SQLite's full signed-64-bit range across JSON tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
struct Decimal(i64);
impl From<Decimal> for String {
    fn from(v: Decimal) -> Self {
        v.0.to_string()
    }
}
impl TryFrom<String> for Decimal {
    type Error = &'static str;
    fn try_from(v: String) -> std::result::Result<Self, Self::Error> {
        v.parse::<i64>()
            .ok()
            .filter(|n| n.to_string() == v)
            .map(Self)
            .ok_or("invalid decimal")
    }
}
impl FromSql for Decimal {
    fn column_result(v: ValueRef<'_>) -> FromSqlResult<Self> {
        i64::column_result(v).map(Self)
    }
}
impl ToSql for Decimal {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Integer(self.0)))
    }
}

macro_rules! table {
    ($name:ident,$table:literal,$order:literal,{$($field:ident:$ty:ty),+ $(,)?})=>{
        #[derive(Debug,Clone,Serialize,Deserialize)]
        #[serde(rename_all="camelCase",deny_unknown_fields)]
        struct $name { $( $field:$ty ),+ }
        impl $name {
            fn read(c:&Connection)->Result<Vec<Self>>{
                let columns=[$(stringify!($field)),+].join(",");
                let mut statement=c.prepare(&format!("SELECT {columns} FROM {} ORDER BY {} LIMIT {}",$table,$order,MAX_ROWS+1)).map_err(sql_error)?;
                let rows=statement.query_map([],|row|Ok(Self{$($field:row.get(stringify!($field))?),+})).map_err(sql_error)?.collect::<std::result::Result<Vec<_>,_>>().map_err(sql_error)?;
                Ok(rows)
            }
            fn insert(&self,tx:&Transaction<'_>)->Result<()> {
                let columns=[$(stringify!($field)),+];
                let placeholders=vec!["?";columns.len()].join(",");
                tx.execute(&format!("INSERT INTO {} ({}) VALUES ({placeholders})",$table,columns.join(",")),rusqlite::params![$(&self.$field),+]).map_err(sql_error)?;
                Ok(())
            }
        }
    }
}
table!(Content,"files","id",{id:Decimal,sha256:String,size_bytes:Decimal,created_at:String});
table!(Location,"file_locations","id",{id:Decimal,file_id:Decimal,path:String,path_key:String,verification_status:String,last_verified_at:Option<String>,created_at:String});
table!(Item,"collection_items","id",{id:Decimal,title:String,platform:Option<String>,edition:Option<String>,language:Option<String>,declared_source:Option<String>,declared_license:Option<String>,notes:Option<String>,created_at:String,updated_at:String});
table!(ItemFile,"collection_item_files","item_id,relative_path",{item_id:Decimal,file_id:Decimal,relative_path:String});
table!(Task,"tasks","id",{id:Decimal,kind:String,status:String,processed_count:Decimal,total_count:Option<Decimal>,error_code:Option<String>,created_at:String,started_at:Option<String>,finished_at:Option<String>,request_id:Option<String>,selection_token:Option<String>,revision:Decimal,updated_at:String});
table!(Source,"import_sources","source_key",{source_key:String,item_id:Decimal});
table!(Identity,"collection_manifest_ids","item_id",{item_id:Decimal,manifest_id:String});
table!(Identification,"file_identifications","file_id",{file_id:Decimal,format_id:String,platform_id:Option<String>,detector_version:Decimal});

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Data {
    files: Vec<Content>,
    file_locations: Vec<Location>,
    collection_items: Vec<Item>,
    collection_item_files: Vec<ItemFile>,
    tasks: Vec<Task>,
    import_sources: Vec<Source>,
    collection_manifest_ids: Vec<Identity>,
    #[serde(default)]
    file_identifications: Vec<Identification>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Backup {
    format: String,
    backup_version: u32,
    database_schema_version: u32,
    created_at: String,
    data: Data,
}

impl Data {
    fn read(c: &Connection) -> Result<Self> {
        let data = Self {
            files: Content::read(c)?,
            file_locations: Location::read(c)?,
            collection_items: Item::read(c)?,
            collection_item_files: ItemFile::read(c)?,
            tasks: Task::read(c)?,
            import_sources: Source::read(c)?,
            collection_manifest_ids: Identity::read(c)?,
            file_identifications: Identification::read(c)?,
        };
        data.check_limits()?;
        Ok(data)
    }
    fn check_limits(&self) -> Result<()> {
        let count = self.files.len()
            + self.file_locations.len()
            + self.collection_items.len()
            + self.collection_item_files.len()
            + self.tasks.len()
            + self.import_sources.len()
            + self.collection_manifest_ids.len()
            + self.file_identifications.len();
        if count > MAX_ROWS
            || self.files.iter().any(|r| r.id.0 <= 0)
            || self
                .file_locations
                .iter()
                .any(|r| r.id.0 <= 0 || r.path.contains('\0') || r.path_key.contains('\0'))
            || self.collection_items.iter().any(|r| r.id.0 <= 0)
            || self
                .tasks
                .iter()
                .any(|r| r.id.0 <= 0 || r.revision.0 < 0 || r.revision.0 == i64::MAX)
        {
            return Err(invalid());
        }
        if self.tasks.iter().any(|r| {
            let time = r.updated_at.as_bytes();
            let valid_time = time.len() == 24
                && time[4] == b'-'
                && time[7] == b'-'
                && time[10] == b'T'
                && time[13] == b':'
                && time[16] == b':'
                && time[19] == b'.'
                && time[23] == b'Z'
                && time
                    .iter()
                    .enumerate()
                    .all(|(i, c)| [4, 7, 10, 13, 16, 19, 23].contains(&i) || c.is_ascii_digit());
            !valid_time
                || (r.status == "failed" && r.error_code.is_none())
                || (["succeeded", "cancelled"].contains(&r.status.as_str())
                    && r.error_code.is_some())
                || r.error_code.as_ref().is_some_and(|code| {
                    serde_json::from_value::<ErrorCode>(serde_json::Value::String(code.clone()))
                        .is_err()
                })
        }) {
            return Err(invalid());
        }
        if self.collection_manifest_ids.iter().any(|r| {
            !Uuid::parse_str(&r.manifest_id).is_ok_and(|id| {
                id.get_version_num() == 4
                    && id.get_variant() == uuid::Variant::RFC4122
                    && id.to_string() == r.manifest_id
            })
        }) {
            return Err(invalid());
        }
        Ok(())
    }
    fn insert(&self, tx: &Transaction<'_>) -> Result<()> {
        for r in &self.files {
            r.insert(tx)?;
        }
        for r in &self.collection_items {
            r.insert(tx)?;
        }
        for r in &self.file_locations {
            r.insert(tx)?;
        }
        for r in &self.collection_item_files {
            r.insert(tx)?;
        }
        for r in &self.tasks {
            r.insert(tx)?;
        }
        for r in &self.import_sources {
            r.insert(tx)?;
        }
        for r in &self.file_identifications {
            r.insert(tx)?;
        }
        for r in &self.collection_manifest_ids {
            r.insert(tx)?;
        }
        Ok(())
    }
    fn fingerprint(&self) -> Result<Vec<u8>> {
        Ok(Sha256::digest(serde_json::to_vec(self).map_err(|_| invalid())?).to_vec())
    }
}
impl Backup {
    fn snapshot(c: &Connection) -> Result<Self> {
        Ok(Self {
            format: FORMAT.into(),
            backup_version: VERSION,
            database_schema_version: DATABASE_SCHEMA,
            created_at: c
                .query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ','now')", [], |r| {
                    r.get(0)
                })
                .map_err(sql_error)?,
            data: Data::read(c)?,
        })
    }
    fn bytes(&self) -> Result<Vec<u8>> {
        let mut bytes = serde_json::to_vec_pretty(self).map_err(|_| invalid())?;
        bytes.push(b'\n');
        if bytes.len() > MAX_BYTES {
            return Err(invalid());
        }
        Ok(bytes)
    }
    fn validate(&self) -> Result<()> {
        if !matches!(
            (self.backup_version, self.database_schema_version),
            (1, 3) | (2, 4)
        ) {
            return Err(error(ErrorCode::BackupVersionUnsupported));
        }
        if self.backup_version == 1 && !self.data.file_identifications.is_empty() {
            return Err(invalid());
        }
        if self.format != FORMAT || self.created_at.is_empty() || self.created_at.len() > 64 {
            return Err(invalid());
        }
        self.data.check_limits()?;
        // Only the application's embedded schema and parameterized inserts run.
        let mut candidate = Connection::open_in_memory().map_err(sql_error)?;
        candidate
            .pragma_update(None, "foreign_keys", true)
            .map_err(sql_error)?;
        migrations::apply(&mut candidate).map_err(|_| invalid())?;
        let tx = candidate.transaction().map_err(sql_error)?;
        self.data.insert(&tx)?;
        let invalid_times: i64 = tx
            .query_row(
                "SELECT count(*) FROM tasks WHERE julianday(updated_at) IS NULL",
                [],
                |row| row.get(0),
            )
            .map_err(sql_error)?;
        if invalid_times != 0 {
            return Err(invalid());
        }
        let violations: i64 = tx
            .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| {
                r.get(0)
            })
            .map_err(sql_error)?;
        if violations != 0 {
            return Err(invalid());
        }
        tx.commit().map_err(sql_error)?;
        Ok(())
    }
    fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_BYTES {
            return Err(invalid());
        }
        let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| invalid())?;
        let pair = (
            value.get("backupVersion").and_then(|v| v.as_u64()),
            value.get("databaseSchemaVersion").and_then(|v| v.as_u64()),
        );
        if let (Some(version), Some(schema)) = pair
            && !matches!((version, schema), (1, 3) | (2, 4))
        {
            return Err(error(ErrorCode::BackupVersionUnsupported));
        }
        if pair == (Some(2), Some(4))
            && value
                .get("data")
                .and_then(|d| d.get("fileIdentifications"))
                .is_none()
        {
            return Err(invalid());
        }
        let backup: Self = serde_json::from_slice(bytes).map_err(|_| invalid())?;
        // A compact input must also fit the writer's expanded representation.
        backup.bytes()?;
        backup.validate()?;
        Ok(backup)
    }
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let fail = || error(ErrorCode::BackupWriteFailed);
    if !path
        .extension()
        .is_some_and(|v| v.eq_ignore_ascii_case("json"))
    {
        return Err(fail());
    }
    let parent = path
        .parent()
        .filter(|v| !v.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|_| fail())?;
    file.write_all(bytes).map_err(|_| fail())?;
    file.as_file().sync_all().map_err(|_| fail())?;
    file.persist_noclobber(path).map_err(|_| fail())?;
    Ok(())
}
fn read_backup(path: &Path) -> Result<Backup> {
    let f = File::open(path).map_err(|_| invalid())?;
    let mut bytes = Vec::new();
    f.take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| invalid())?;
    Backup::parse(&bytes)
}
fn idle<T>(
    db: &Database,
    imports: &ImportState,
    operation: impl FnOnce(&mut Connection) -> Result<T>,
) -> Result<T> {
    let _admission = imports
        .gate
        .try_lock()
        .map_err(|_| error(ErrorCode::LibraryBusy))?;
    let _worker = imports
        .worker
        .try_lock()
        .map_err(|_| error(ErrorCode::LibraryBusy))?;
    let mut c = db
        .connection
        .lock()
        .map_err(|_| error(ErrorCode::DatabaseUnavailable))?;
    let active: i64 = c
        .query_row(
            "SELECT count(*) FROM tasks WHERE status IN ('queued','running')",
            [],
            |r| r.get(0),
        )
        .map_err(sql_error)?;
    if active != 0 {
        return Err(error(ErrorCode::LibraryBusy));
    }
    operation(&mut c)
}
pub fn save(db: &Database, imports: &ImportState, path: &Path) -> Result<()> {
    // Snapshot reads include committed WAL data; do not copy a live .sqlite file.
    let backup = idle(db, imports, |c| {
        let tx = c.transaction().map_err(sql_error)?;
        let backup = Backup::snapshot(&tx)?;
        tx.commit().map_err(sql_error)?;
        Ok(backup)
    })?;
    let bytes = backup.bytes()?;
    backup.validate()?;
    write_new(path, &bytes)
}

struct Prepared {
    backup: Backup,
    fingerprint: Vec<u8>,
    token: String,
    created: Instant,
}
#[derive(Default)]
pub struct BackupState {
    prepared: Mutex<Option<Prepared>>,
}
impl BackupState {
    pub fn prepare(
        &self,
        db: &Database,
        imports: &ImportState,
        path: &Path,
    ) -> Result<LibraryRestorePreview> {
        let backup = read_backup(path)?;
        let fingerprint = idle(db, imports, |c| {
            let tx = c.transaction().map_err(sql_error)?;
            let fingerprint = Data::read(&tx)?.fingerprint()?;
            tx.commit().map_err(sql_error)?;
            Ok(fingerprint)
        })?;
        let token = Uuid::new_v4().to_string();
        let preview = LibraryRestorePreview {
            restore_token: token.clone(),
            file_name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            item_count: backup.data.collection_items.len() as u32,
            file_count: backup.data.files.len() as u32,
            location_count: backup.data.file_locations.len() as u32,
            task_count: backup.data.tasks.len() as u32,
        };
        *self
            .prepared
            .lock()
            .map_err(|_| error(ErrorCode::UnexpectedError))? = Some(Prepared {
            backup,
            fingerprint,
            token,
            created: Instant::now(),
        });
        Ok(preview)
    }
    pub fn discard(&self) {
        self.prepared
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
    }
    pub fn restore(
        &self,
        db: &Database,
        imports: &ImportState,
        token: &str,
    ) -> Result<LibraryRestoreResult> {
        let prepared = {
            let mut pending = self
                .prepared
                .lock()
                .map_err(|_| error(ErrorCode::UnexpectedError))?;
            if !pending
                .as_ref()
                .is_some_and(|p| p.token == token && p.created.elapsed() < Duration::from_secs(300))
            {
                return Err(error(ErrorCode::RestoreStale));
            }
            pending.take().unwrap()
        };
        idle(db, imports, |c| {
            let tx = c
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .map_err(sql_error)?;
            let current = Backup::snapshot(&tx)?;
            if current.data.fingerprint()? != prepared.fingerprint {
                return Err(error(ErrorCode::RestoreStale));
            }
            let recovery_bytes = current.bytes()?;
            let directory = db.path.parent().unwrap_or(Path::new(".")).join("backups");
            std::fs::create_dir_all(&directory).map_err(|_| error(ErrorCode::BackupWriteFailed))?;
            let name = format!("before-restore-{}.game4-library.json", Uuid::new_v4());
            write_new(&directory.join(&name), &recovery_bytes)?;
            // One transaction: deletion/insertion/health reset either all commit or all roll back.
            replace(&tx, &prepared.backup.data).map_err(|_| error(ErrorCode::RestoreFailed))?;
            // Health/task resets must not make the restored catalog unsavable.
            Backup::snapshot(&tx)
                .and_then(|backup| backup.bytes())
                .map_err(|_| error(ErrorCode::RestoreFailed))?;
            tx.commit().map_err(|_| error(ErrorCode::RestoreFailed))?;
            imports.invalidate_selections();
            Ok(LibraryRestoreResult {
                item_count: prepared.backup.data.collection_items.len() as u32,
                file_count: prepared.backup.data.files.len() as u32,
                location_count: prepared.backup.data.file_locations.len() as u32,
                task_count: prepared.backup.data.tasks.len() as u32,
                recovery_file_name: name,
            })
        })
    }
}
fn replace(tx: &Transaction<'_>, data: &Data) -> Result<()> {
    tx.execute_batch("DELETE FROM file_identifications; DELETE FROM collection_manifest_ids; DELETE FROM import_sources; DELETE FROM collection_item_files; DELETE FROM file_locations; DELETE FROM collection_items; DELETE FROM files; DELETE FROM tasks;").map_err(sql_error)?;
    data.insert(tx)?;
    tx.execute(
        "UPDATE file_locations SET verification_status='unverified',last_verified_at=NULL",
        [],
    )
    .map_err(sql_error)?;
    tx.execute("UPDATE tasks SET status='interrupted',error_code='UNEXPECTED_ERROR',revision=revision+1,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now'),finished_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE status IN ('queued','running')",[]).map_err(sql_error)?;
    Ok(())
}

#[cfg(test)]
mod tests;
