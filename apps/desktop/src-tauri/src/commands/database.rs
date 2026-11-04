use crate::{
    contracts::{DatabaseInfo, ErrorCode, IpcError},
    database::{Database, DatabaseStats},
};
use std::sync::Arc;

fn wire_count(value: i64) -> Result<u64, IpcError> {
    if !(0..=9_007_199_254_740_991).contains(&value) {
        return Err(IpcError::new(ErrorCode::InvalidResponse));
    }
    Ok(value as u64)
}

fn response(stats: DatabaseStats) -> Result<DatabaseInfo, IpcError> {
    Ok(DatabaseInfo {
        schema_version: u32::try_from(stats.schema_version)
            .ok()
            .filter(|value| *value > 0)
            .ok_or_else(|| IpcError::new(ErrorCode::InvalidResponse))?,
        file_count: wire_count(stats.file_count)?,
        location_count: wire_count(stats.location_count)?,
        item_count: wire_count(stats.item_count)?,
        task_count: wire_count(stats.task_count)?,
    })
}

#[tauri::command]
pub async fn database_info(
    database: tauri::State<'_, Arc<Database>>,
) -> Result<DatabaseInfo, IpcError> {
    let database = Arc::clone(database.inner());
    let stats = tauri::async_runtime::spawn_blocking(move || database.info())
        .await
        .map_err(|_| IpcError::new(ErrorCode::DatabaseTaskFailed))?
        .map_err(|_| IpcError::new(ErrorCode::DatabaseUnavailable))?;
    response(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_negative_and_unsafe_json_counts() {
        assert!(wire_count(-1).is_err());
        assert!(wire_count(9_007_199_254_740_992).is_err());
        assert_eq!(
            wire_count(9_007_199_254_740_991).unwrap(),
            9_007_199_254_740_991
        );
    }
}
