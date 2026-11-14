use crate::{contracts::*, storage_settings::StorageSettings};
use std::sync::Arc;
#[tauri::command]
pub async fn storage_settings_get(
    storage: tauri::State<'_, Arc<StorageSettings>>,
) -> Result<StorageSettingsInfo, IpcError> {
    let storage = Arc::clone(storage.inner());
    tauri::async_runtime::spawn_blocking(move || storage.get())
        .await
        .map_err(|_| IpcError::new(ErrorCode::StorageUnavailable))?
}
#[tauri::command]
pub async fn storage_directory_choose(
    window: tauri::WebviewWindow,
    storage: tauri::State<'_, Arc<StorageSettings>>,
    request: StorageDirectoryRequest,
) -> Result<StorageDirectoryChoice, IpcError> {
    let storage = Arc::clone(storage.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let mut dialog = rfd::FileDialog::new().set_parent(&window);
        if let Ok(Some(path)) = storage.initial_directory(request.kind) {
            dialog = dialog.set_directory(path);
        }
        let path = dialog.pick_folder();
        let Some(path) = path else {
            return Ok(StorageDirectoryChoice {
                cancelled: true,
                settings: storage.get()?,
            });
        };
        Ok(StorageDirectoryChoice {
            cancelled: false,
            settings: storage.change(request.kind, Some(path))?,
        })
    })
    .await
    .map_err(|_| IpcError::new(ErrorCode::StorageUnavailable))?
}
#[tauri::command]
pub async fn storage_directory_reset(
    storage: tauri::State<'_, Arc<StorageSettings>>,
    request: StorageDirectoryRequest,
) -> Result<StorageSettingsInfo, IpcError> {
    let storage = Arc::clone(storage.inner());
    tauri::async_runtime::spawn_blocking(move || storage.change(request.kind, None))
        .await
        .map_err(|_| IpcError::new(ErrorCode::StorageUnavailable))?
}
