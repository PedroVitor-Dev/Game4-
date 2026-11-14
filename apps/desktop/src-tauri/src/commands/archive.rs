use crate::{archive, contracts::*, database::Database};
use std::sync::Arc;
#[tauri::command]
pub async fn archive_request(
    window: tauri::WebviewWindow,
    database: tauri::State<'_, Arc<Database>>,
    state: tauri::State<'_, Arc<archive::ArchiveState>>,
    request: ArchiveRequest,
) -> Result<ArchiveResponse, IpcError> {
    let db = Arc::clone(database.inner());
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let mut c = db
            .connection
            .lock()
            .map_err(|_| IpcError::new(ErrorCode::DatabaseUnavailable))?;
        archive::dialog_request(&mut c, &state, &window, request)
    })
    .await
    .map_err(|_| IpcError::new(ErrorCode::DatabaseTaskFailed))?
}
