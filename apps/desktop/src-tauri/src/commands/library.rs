use crate::{
    contracts::*,
    database::Database,
    library::{self, ImportState},
};
use std::sync::Arc;

#[tauri::command]
pub async fn library_select_sources(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, Arc<ImportState>>,
    request: SelectSourcesRequest,
) -> Result<SourceSelection, IpcError> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let dialog = rfd::FileDialog::new().set_parent(&window);
        let paths = match request.kind {
            SourceKind::Files => dialog.pick_files(),
            SourceKind::Directories => dialog.pick_folders(),
        };
        state.select(paths.unwrap_or_default())
    })
    .await
    .map_err(|_| IpcError::new(ErrorCode::UnexpectedError))?
}
#[tauri::command]
pub async fn library_import(
    database: tauri::State<'_, Arc<Database>>,
    state: tauri::State<'_, Arc<ImportState>>,
    request: StartImportRequest,
) -> Result<TaskAccepted, IpcError> {
    library::validate_request(&request)?;
    let db = Arc::clone(database.inner());
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let _gate = state
            .gate
            .lock()
            .map_err(|_| IpcError::new(ErrorCode::UnexpectedError))?;
        if let Some(task) = library::existing(&db, &request)? {
            return Ok(task);
        }
        let paths = state.take(&request.selection_token)?;
        let task = library::create_task(&db, &request)?;
        let id = task.task_id.clone();
        let worker_db = Arc::clone(&db);
        let worker_state = Arc::clone(&state);
        tauri::async_runtime::spawn_blocking(move || {
            // Serialize imports; the UI and read commands remain responsive.
            let _gate = worker_state
                .worker
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let _ = library::run_import(&worker_db, &id, paths);
        });
        Ok(task)
    })
    .await
    .map_err(|_| IpcError::new(ErrorCode::DatabaseTaskFailed))?
}
#[tauri::command]
pub async fn library_list(
    database: tauri::State<'_, Arc<Database>>,
    request: LibraryListRequest,
) -> Result<LibraryPage, IpcError> {
    let db = Arc::clone(database.inner());
    tauri::async_runtime::spawn_blocking(move || library::library_list(&db, request))
        .await
        .map_err(|_| IpcError::new(ErrorCode::DatabaseTaskFailed))?
}
#[tauri::command]
pub async fn tasks_list(
    database: tauri::State<'_, Arc<Database>>,
    request: PageRequest,
) -> Result<TaskPage, IpcError> {
    let db = Arc::clone(database.inner());
    tauri::async_runtime::spawn_blocking(move || library::tasks_list(&db, request))
        .await
        .map_err(|_| IpcError::new(ErrorCode::DatabaseTaskFailed))?
}
#[tauri::command]
pub async fn task_get(
    database: tauri::State<'_, Arc<Database>>,
    request: TaskIdRequest,
) -> Result<TaskSnapshot, IpcError> {
    let db = Arc::clone(database.inner());
    tauri::async_runtime::spawn_blocking(move || library::task_get(&db, &request.task_id))
        .await
        .map_err(|_| IpcError::new(ErrorCode::DatabaseTaskFailed))?
}

#[tauri::command]
pub async fn library_item_files(
    database: tauri::State<'_, Arc<Database>>,
    request: LibraryItemFilesRequest,
) -> Result<LibraryItemFilesPage, IpcError> {
    let db = Arc::clone(database.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let c = db
            .connection
            .lock()
            .map_err(|_| IpcError::new(ErrorCode::DatabaseUnavailable))?;
        crate::identification::item_files(&c, request)
    })
    .await
    .map_err(|_| IpcError::new(ErrorCode::DatabaseTaskFailed))?
}
