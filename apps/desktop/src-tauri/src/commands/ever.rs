use crate::{contracts::*, database::Database, ever, library::ImportState, manifests};
use std::sync::Arc;
#[tauri::command]
pub async fn library_export_ever(
    window: tauri::WebviewWindow,
    storage: tauri::State<'_, Arc<crate::storage_settings::StorageSettings>>,
    database: tauri::State<'_, Arc<Database>>,
    state: tauri::State<'_, Arc<ImportState>>,
    request: ExportManifestRequest,
) -> Result<EverExportResult, IpcError> {
    manifests::item_id(&request.item_id)?;
    let storage = Arc::clone(storage.inner());
    let db = Arc::clone(database.inner());
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let path = crate::storage_settings::dialog(&storage, StorageDirectoryKind::Exports)?
            .set_parent(&window)
            .add_filter("Game4 Ever Manifest", &["g4e"])
            .set_file_name(format!("game4-item-{}.g4e", request.item_id))
            .save_file();
        let Some(path) = path else {
            return Ok(EverExportResult {
                cancelled: true,
                file_name: None,
                torrent_file_name: None,
                magnet: None,
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
        let active: bool = db
            .connection
            .lock()
            .map_err(|_| IpcError::new(ErrorCode::DatabaseUnavailable))?
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM tasks WHERE status IN ('queued','running'))",
                [],
                |r| r.get(0),
            )
            .map_err(|_| IpcError::new(ErrorCode::DatabaseUnavailable))?;
        if active {
            return Err(IpcError::new(ErrorCode::LibraryBusy));
        }
        let created = ever::from_catalog(&db, &request.item_id)?;
        let (file_name, torrent_file_name) = ever::write_pair(&path, &created)?;
        Ok(EverExportResult {
            cancelled: false,
            file_name: Some(file_name),
            torrent_file_name: Some(torrent_file_name),
            magnet: Some(created.document.torrents[0].magnet.clone()),
        })
    })
    .await
    .map_err(|_| IpcError::new(ErrorCode::UnexpectedError))?
}
#[tauri::command]
pub async fn ever_open(window: tauri::WebviewWindow) -> Result<EverOpenResult, IpcError> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = rfd::FileDialog::new()
            .set_parent(&window)
            .add_filter("Game4 Ever Manifest", &["g4e"])
            .pick_file();
        let Some(path) = path else {
            return Ok(EverOpenResult {
                cancelled: true,
                manifest: None,
            });
        };
        let doc = ever::read(&path)?;
        let total = ever::validate(&doc)?;
        Ok(EverOpenResult {
            cancelled: false,
            manifest: Some(EverSummary {
                file_name: path
                    .file_name()
                    .ok_or_else(|| IpcError::new(ErrorCode::G4eInvalid))?
                    .to_string_lossy()
                    .into_owned(),
                schema_version: 1,
                representation: doc.representation,
                title: doc.manifest.collection.title,
                file_count: doc.manifest.files.len() as u32,
                total_bytes: total.to_string(),
                magnets: doc.torrents.into_iter().map(|t| t.magnet).collect(),
                content_verified: false,
            }),
        })
    })
    .await
    .map_err(|_| IpcError::new(ErrorCode::UnexpectedError))?
}
