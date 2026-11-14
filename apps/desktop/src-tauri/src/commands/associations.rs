use crate::{contracts::*, file_associations::FileInbox};
use std::sync::Arc;
#[tauri::command]
pub fn associated_file_next(inbox: tauri::State<'_, Arc<FileInbox>>) -> Option<AssociatedFile> {
    inbox.next()
}
#[tauri::command]
pub async fn associated_file_read(
    inbox: tauri::State<'_, Arc<FileInbox>>,
    request: AssociatedFileRequest,
) -> Result<AssociatedFileResult, IpcError> {
    let inbox = Arc::clone(inbox.inner());
    tauri::async_runtime::spawn_blocking(move || inbox.read(&request.token))
        .await
        .map_err(|_| IpcError::new(ErrorCode::UnexpectedError))?
}
