mod migrations;

use rusqlite::Connection;
use std::{path::Path, sync::Mutex, time::Duration};

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("failed to create the database directory")]
    Io(#[from] std::io::Error),
    #[error("SQLite operation failed")]
    Sqlite(#[from] rusqlite::Error),
    #[error("database schema {found} is newer than supported schema {supported}")]
    NewerSchema { found: i64, supported: i64 },
    #[error("invalid negative database schema version {0}")]
    InvalidSchemaVersion(i64),
    #[error("database connection lock is unavailable")]
    LockUnavailable,
}

pub type Result<T> = std::result::Result<T, DatabaseError>;

pub struct Database {
    connection: Mutex<Connection>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseInfo {
    pub schema_version: i64,
    pub file_count: i64,
    pub location_count: i64,
    pub item_count: i64,
    pub task_count: i64,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)?;
        }
        let mut connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", true)?;
        migrations::apply(&mut connection)?;
        // WAL allows readers during writes; SQLite manages its sidecar files.
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn info(&self) -> Result<DatabaseInfo> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| DatabaseError::LockUnavailable)?;
        Ok(connection.query_row(
            "SELECT (SELECT user_version FROM pragma_user_version),
                    (SELECT count(*) FROM files), (SELECT count(*) FROM file_locations),
                    (SELECT count(*) FROM collection_items), (SELECT count(*) FROM tasks)",
            [],
            |row| {
                Ok(DatabaseInfo {
                    schema_version: row.get(0)?,
                    file_count: row.get(1)?,
                    location_count: row.get(2)?,
                    item_count: row.get(3)?,
                    task_count: row.get(4)?,
                })
            },
        )?)
    }
}

#[cfg(test)]
mod tests;
