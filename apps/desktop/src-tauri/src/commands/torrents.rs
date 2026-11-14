use crate::{
    contracts::*,
    database::Database,
    ever,
    library::ImportState,
    torrents::{self, Transfers},
};
use std::sync::Arc;
#[tauri::command]
pub async fn torrent_create(
    window: tauri::WebviewWindow,
    database: tauri::State<'_, Arc<Database>>,
    state: tauri::State<'_, Arc<ImportState>>,
    request: ExportManifestRequest,
) -> Result<TorrentCreateResult, IpcError> {
    crate::manifests::item_id(&request.item_id)?;
    let db = Arc::clone(database.inner());
    let state = Arc::clone(state.inner());
    tokio::task::spawn_blocking(move || {
        let path = rfd::FileDialog::new()
            .set_parent(&window)
            .add_filter("BitTorrent metainfo", &["torrent"])
            .set_file_name(format!("game4-item-{}.torrent", request.item_id))
            .save_file();
        let Some(path) = path else {
            return Ok(TorrentCreateResult {
                cancelled: true,
                file_name: None,
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
        torrents::write_new(&path, &created.torrent)?;
        Ok(TorrentCreateResult {
            cancelled: false,
            file_name: path.file_name().map(|n| n.to_string_lossy().into_owned()),
            magnet: Some(created.document.torrents[0].magnet.clone()),
        })
    })
    .await
    .map_err(|_| IpcError::new(ErrorCode::UnexpectedError))?
}
#[tauri::command]
pub async fn torrent_import(
    window: tauri::WebviewWindow,
    transfers: tauri::State<'_, Arc<Transfers>>,
) -> Result<TorrentImportResult, IpcError> {
    let transfers = Arc::clone(transfers.inner());
    tokio::task::spawn_blocking(move || {
        let path = rfd::FileDialog::new()
            .set_parent(&window)
            .add_filter("BitTorrent metainfo", &["torrent"])
            .pick_file();
        let Some(path) = path else {
            return Ok(TorrentImportResult {
                cancelled: true,
                preview: None,
            });
        };
        let bytes = torrents::read_file(&path)?;
        Ok(TorrentImportResult {
            cancelled: false,
            preview: Some(transfers.import(bytes)?),
        })
    })
    .await
    .map_err(|_| IpcError::new(ErrorCode::UnexpectedError))?
}
#[tauri::command]
pub async fn torrent_seed(
    database: tauri::State<'_, Arc<Database>>,
    state: tauri::State<'_, Arc<ImportState>>,
    transfers: tauri::State<'_, Arc<Transfers>>,
    request: StartSeedRequest,
) -> Result<TransferSnapshot, IpcError> {
    transfers
        .seed(
            Arc::clone(database.inner()),
            Arc::clone(state.inner()),
            request,
        )
        .await
}
#[tauri::command]
pub async fn torrent_download(
    window: tauri::WebviewWindow,
    transfers: tauri::State<'_, Arc<Transfers>>,
    request: StartDownloadRequest,
) -> Result<TransferStartResult, IpcError> {
    if !request.peer.trim().is_empty() || !request.dht {
        torrents::peer(&request.peer)?;
    }
    let parent = tokio::task::spawn_blocking(move || {
        rfd::FileDialog::new().set_parent(&window).pick_folder()
    })
    .await
    .map_err(|_| IpcError::new(ErrorCode::UnexpectedError))?;
    let Some(parent) = parent else {
        return Ok(TransferStartResult {
            cancelled: true,
            transfer: None,
        });
    };
    Ok(TransferStartResult {
        cancelled: false,
        transfer: Some(transfers.download(request, parent).await?),
    })
}
#[tauri::command]
pub async fn transfers_list(
    transfers: tauri::State<'_, Arc<Transfers>>,
) -> Result<TransferPage, IpcError> {
    Ok(transfers.list())
}
#[tauri::command]
pub async fn transfer_stop(
    transfers: tauri::State<'_, Arc<Transfers>>,
    request: TransferIdRequest,
) -> Result<TransferSnapshot, IpcError> {
    transfers.stop(&request.transfer_id).await
}

#[tauri::command]
pub async fn magnet_generate(
    database: tauri::State<'_, Arc<Database>>,
    state: tauri::State<'_, Arc<ImportState>>,
    request: ExportManifestRequest,
) -> Result<MagnetLink, IpcError> {
    crate::manifests::item_id(&request.item_id)?;
    let db = Arc::clone(database.inner());
    let state = Arc::clone(state.inner());
    tokio::task::spawn_blocking(move || {
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
        torrents::normalize_magnet(&created.document.torrents[0].magnet)
    })
    .await
    .map_err(|_| IpcError::new(ErrorCode::UnexpectedError))?
}
#[tauri::command]
pub async fn magnet_resolve(
    transfers: tauri::State<'_, Arc<Transfers>>,
    request: ResolveMagnetRequest,
) -> Result<TorrentImportResult, IpcError> {
    transfers.resolve_magnet(request).await
}
#[tauri::command]
pub async fn magnet_cancel(
    transfers: tauri::State<'_, Arc<Transfers>>,
    request: CancelMagnetRequest,
) -> Result<CancelMagnetResult, IpcError> {
    transfers.cancel_magnet(request)
}
