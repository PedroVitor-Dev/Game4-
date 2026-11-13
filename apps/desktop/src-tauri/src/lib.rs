pub mod backups;
pub mod commands;
pub mod contracts;
pub mod database;
pub mod identification;
pub mod library;
pub mod manifests;
pub mod tasks;
pub mod verification;

use std::sync::Arc;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let path = app
                .path()
                .app_local_data_dir()?
                .join("database/game4.sqlite3");
            app.manage(Arc::new(database::Database::open(&path)?));
            app.manage(Arc::new(library::ImportState::default()));
            app.manage(Arc::new(backups::BackupState::default()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::application::app_info,
            commands::database::database_info,
            commands::library::library_select_sources,
            commands::library::library_import,
            commands::library::library_verify,
            commands::library::library_integrity,
            commands::library::library_list,
            commands::library::library_item_files,
            commands::library::tasks_list,
            commands::library::task_get,
            commands::manifests::library_export_manifest,
            commands::backups::library_save_backup,
            commands::backups::library_prepare_restore,
            commands::backups::library_restore_backup
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Game4∞");
}
