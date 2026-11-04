use crate::contracts::{AppInfo, CONTRACT_VERSION, IMPLEMENTED_COMMANDS};

#[tauri::command]
pub fn app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        contract_version: CONTRACT_VERSION,
        capabilities: IMPLEMENTED_COMMANDS
            .iter()
            .map(|command| {
                serde_json::to_value(command)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect(),
    }
}
