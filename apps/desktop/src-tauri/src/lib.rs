pub mod backups;
pub mod commands;
pub mod contracts;
pub mod database;
pub mod ever;
pub mod identification;
pub mod library;
pub mod manifests;
pub mod packages;
pub mod storage_settings;
pub mod tasks;
pub mod torrents;
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
            app.manage(Arc::new(storage_settings::StorageSettings::new(
                app.path().app_local_data_dir()?,
            )));
            app.manage(Arc::new(backups::BackupState::default()));
            let transfers_path = app
                .path()
                .app_local_data_dir()?
                .join("transfers/state.sqlite3");
            app.manage(Arc::new(
                torrents::Transfers::open(&transfers_path)
                    .unwrap_or_else(|_| torrents::Transfers::unavailable()),
            ));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::application::app_info,
            commands::storage::storage_settings_get,
            commands::storage::storage_directory_choose,
            commands::storage::storage_directory_reset,
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
            commands::packages::library_export_package,
            commands::packages::package_open,
            commands::ever::library_export_ever,
            commands::ever::ever_open,
            commands::torrents::transfer_metrics_info,
            commands::torrents::trackers_validate,
            commands::torrents::magnet_generate,
            commands::torrents::magnet_resolve,
            commands::torrents::magnet_cancel,
            commands::torrents::torrent_create,
            commands::torrents::torrent_import,
            commands::torrents::torrent_seed,
            commands::torrents::torrent_download,
            commands::torrents::transfers_list,
            commands::torrents::transfer_stop,
            commands::torrents::transfer_pause,
            commands::torrents::transfer_resume,
            commands::torrents::transfer_forget,
            commands::torrents::transfer_set_limits,
            commands::backups::library_save_backup,
            commands::backups::library_prepare_restore,
            commands::backups::library_restore_backup
        ])
        .build(tauri::generate_context!())
        .expect("failed to build Game4∞")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                app.state::<Arc<torrents::Transfers>>().abort();
            }
        });
}
