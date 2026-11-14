use crate::{
    backups, backups::BackupState, contracts::*, database::Database, library::ImportState,
};
use std::sync::Arc;
#[tauri::command]
pub async fn library_save_backup(
    window: tauri::WebviewWindow,
    storage: tauri::State<'_, Arc<crate::storage_settings::StorageSettings>>,
    database: tauri::State<'_, Arc<Database>>,
    imports: tauri::State<'_, Arc<ImportState>>,
) -> Result<LibraryBackupResult, IpcError> {
    let storage = Arc::clone(storage.inner());
    let db = Arc::clone(database.inner());
    let imports = Arc::clone(imports.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let path = crate::storage_settings::dialog(&storage, StorageDirectoryKind::Backups)?
            .set_parent(&window)
            .add_filter("Game4 library JSON", &["json"])
            .set_file_name("game4-library-backup-v2.json")
            .save_file();
        let Some(path) = path else {
            return Ok(LibraryBackupResult {
                cancelled: true,
                file_name: None,
            });
        };
        backups::save(&db, &imports, &path)?;
        Ok(LibraryBackupResult {
            cancelled: false,
            file_name: path.file_name().map(|v| v.to_string_lossy().into_owned()),
        })
    })
    .await
    .map_err(|_| IpcError::new(ErrorCode::DatabaseTaskFailed))?
}
#[tauri::command]
pub async fn library_prepare_restore(
    window: tauri::WebviewWindow,
    database: tauri::State<'_, Arc<Database>>,
    imports: tauri::State<'_, Arc<ImportState>>,
    backups: tauri::State<'_, Arc<BackupState>>,
) -> Result<LibraryPrepareRestoreResult, IpcError> {
    let db = Arc::clone(database.inner());
    let imports = Arc::clone(imports.inner());
    let backups = Arc::clone(backups.inner());
    tauri::async_runtime::spawn_blocking(move || {
        backups.discard();
        let path = rfd::FileDialog::new()
            .set_parent(&window)
            .add_filter("Game4 library JSON", &["json"])
            .pick_file();
        let Some(path) = path else {
            return Ok(LibraryPrepareRestoreResult {
                cancelled: true,
                preview: None,
            });
        };
        Ok(LibraryPrepareRestoreResult {
            cancelled: false,
            preview: Some(backups.prepare(&db, &imports, &path)?),
        })
    })
    .await
    .map_err(|_| IpcError::new(ErrorCode::DatabaseTaskFailed))?
}
#[tauri::command]
pub async fn library_restore_backup(
    database: tauri::State<'_, Arc<Database>>,
    imports: tauri::State<'_, Arc<ImportState>>,
    backups: tauri::State<'_, Arc<BackupState>>,
    request: LibraryRestoreRequest,
) -> Result<LibraryRestoreResult, IpcError> {
    let db = Arc::clone(database.inner());
    let imports = Arc::clone(imports.inner());
    let backups = Arc::clone(backups.inner());
    tauri::async_runtime::spawn_blocking(move || {
        backups.restore(&db, &imports, &request.restore_token)
    })
    .await
    .map_err(|_| IpcError::new(ErrorCode::DatabaseTaskFailed))?
}
