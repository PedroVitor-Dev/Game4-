use crate::{contracts::*, database::Database, library::ImportState, manifests, packages};
use std::sync::Arc;

#[tauri::command]
pub async fn library_export_package(
    window: tauri::WebviewWindow,
    storage: tauri::State<'_, Arc<crate::storage_settings::StorageSettings>>,
    database: tauri::State<'_, Arc<Database>>,
    state: tauri::State<'_, Arc<ImportState>>,
    request: ExportManifestRequest,
) -> Result<ExportManifestResult, IpcError> {
    manifests::item_id(&request.item_id)?;
    let storage = Arc::clone(storage.inner());
    let db = Arc::clone(database.inner());
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let path = crate::storage_settings::dialog(&storage, StorageDirectoryKind::Exports)?
            .set_parent(&window)
            .add_filter("Game4 package", &["g4"])
            .set_file_name(format!("game4-item-{}.g4", request.item_id))
            .save_file();
        let Some(path) = path else {
            return Ok(ExportManifestResult {
                cancelled: true,
                file_name: None,
            });
        };
        let _gate = state
            .gate
            .try_lock()
            .map_err(|_| IpcError::new(ErrorCode::LibraryBusy))?;
        let _worker = state
            .worker
            .try_lock()
            .map_err(|_| IpcError::new(ErrorCode::LibraryBusy))?;
        {
            let c = db
                .connection
                .lock()
                .map_err(|_| IpcError::new(ErrorCode::DatabaseUnavailable))?;
            let active: bool = c
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM tasks WHERE status IN ('queued','running'))",
                    [],
                    |r| r.get(0),
                )
                .map_err(|_| IpcError::new(ErrorCode::DatabaseUnavailable))?;
            if active {
                return Err(IpcError::new(ErrorCode::LibraryBusy));
            }
        }
        let (manifest, sources) = packages::from_catalog(&db, &request.item_id)?;
        packages::create(&path, &manifest, &sources)?;
        Ok(ExportManifestResult {
            cancelled: false,
            file_name: path.file_name().map(|v| v.to_string_lossy().into_owned()),
        })
    })
    .await
    .map_err(|_| IpcError::new(ErrorCode::UnexpectedError))?
}
#[tauri::command]
pub async fn package_open(window: tauri::WebviewWindow) -> Result<PackageOpenResult, IpcError> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = rfd::FileDialog::new()
            .set_parent(&window)
            .add_filter("Game4 package", &["g4"])
            .pick_file();
        let Some(path) = path else {
            return Ok(PackageOpenResult {
                cancelled: true,
                package: None,
            });
        };
        Ok(PackageOpenResult {
            cancelled: false,
            package: Some(crate::file_associations::inspect_package(&path)?),
        })
    })
    .await
    .map_err(|_| IpcError::new(ErrorCode::UnexpectedError))?
}
