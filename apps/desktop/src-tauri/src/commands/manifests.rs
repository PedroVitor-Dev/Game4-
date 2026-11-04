use crate::{
    contracts::{ErrorCode, ExportManifestRequest, ExportManifestResult, IpcError},
    database::Database,
    manifests,
};
use std::sync::Arc;

#[tauri::command]
pub async fn library_export_manifest(
    window: tauri::WebviewWindow,
    database: tauri::State<'_, Arc<Database>>,
    request: ExportManifestRequest,
) -> Result<ExportManifestResult, IpcError> {
    manifests::item_id(&request.item_id)?;
    let db = Arc::clone(database.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let path = rfd::FileDialog::new()
            .set_parent(&window)
            .add_filter("JSON manifest", &["json"])
            .set_file_name(format!("game4-item-{}.manifest.v1.json", request.item_id))
            .save_file();
        let Some(path) = path else {
            return Ok(ExportManifestResult {
                cancelled: true,
                file_name: None,
            });
        };
        let manifest = manifests::from_catalog(&db, &request.item_id)?;
        let bytes = manifests::serialize(&manifest)?;
        manifests::write_new(&path, &bytes)?;
        Ok(ExportManifestResult {
            cancelled: false,
            file_name: path.file_name().map(|v| v.to_string_lossy().into_owned()),
        })
    })
    .await
    .map_err(|_| IpcError::new(ErrorCode::DatabaseTaskFailed))?
}
