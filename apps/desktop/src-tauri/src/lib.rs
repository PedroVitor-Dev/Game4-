pub mod commands;
pub mod database;
pub mod library;
pub mod tasks;

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
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::application::app_info,
            commands::database::database_info
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Game4âˆž");
}
