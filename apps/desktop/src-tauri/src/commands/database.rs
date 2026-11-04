use crate::database::{Database, DatabaseInfo};
use std::sync::Arc;

#[derive(serde::Serialize)]
pub struct CommandError {
    code: &'static str,
}

#[tauri::command]
pub async fn database_info(
    database: tauri::State<'_, Arc<Database>>,
) -> Result<DatabaseInfo, CommandError> {
    let database = Arc::clone(database.inner());
    tauri::async_runtime::spawn_blocking(move || database.info())
        .await
        .map_err(|_| CommandError {
            code: "DATABASE_TASK_FAILED",
        })?
        .map_err(|_| CommandError {
            code: "DATABASE_UNAVAILABLE",
        })
}
