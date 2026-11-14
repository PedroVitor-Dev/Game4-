use super::*;
use std::path::PathBuf;

pub fn target_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../packages/ipc-contracts")
}

pub fn typescript() -> String {
    let config = ts_rs::Config::default();
    let declarations = [
        ImplementedCommand::decl(&config),
        DraftCommand::decl(&config),
        ErrorCode::decl(&config),
        ErrorField::decl(&config),
        IpcError::decl(&config),
        AppInfo::decl(&config),
        DatabaseInfo::decl(&config),
        ExportManifestRequest::decl(&config),
        ExportManifestResult::decl(&config),
        LibraryBackupResult::decl(&config),
        LibraryRestorePreview::decl(&config),
        LibraryPrepareRestoreResult::decl(&config),
        LibraryRestoreRequest::decl(&config),
        LibraryRestoreResult::decl(&config),
        TaskKind::decl(&config),
        TaskStatus::decl(&config),
        VerificationStatus::decl(&config),
        PageRequest::decl(&config),
        LibraryListRequest::decl(&config),
        LibraryItemSummary::decl(&config),
        FileFormat::decl(&config),
        PlatformId::decl(&config),
        IdentificationEvidence::decl(&config),
        FormatSummary::decl(&config),
        PlatformSummary::decl(&config),
        LibraryFileInfo::decl(&config),
        LibraryItemFilesRequest::decl(&config),
        LibraryItemFilesPage::decl(&config),
        LibraryPage::decl(&config),
        SourceKind::decl(&config),
        SelectSourcesRequest::decl(&config),
        SourceSelection::decl(&config),
        StartImportRequest::decl(&config),
        StartVerifyRequest::decl(&config),
        LibraryIntegrity::decl(&config),
        PackageSummary::decl(&config),
        PackageOpenResult::decl(&config),
        EverRepresentation::decl(&config),
        EverSummary::decl(&config),
        EverOpenResult::decl(&config),
        EverExportResult::decl(&config),
        TorrentFileEntry::decl(&config),
        TorrentPreview::decl(&config),
        TorrentImportResult::decl(&config),
        TorrentCreateResult::decl(&config),
        StartSeedRequest::decl(&config),
        StartDownloadRequest::decl(&config),
        TransferIdRequest::decl(&config),
        TransferMode::decl(&config),
        TransferStatus::decl(&config),
        TransferSnapshot::decl(&config),
        TransferStartResult::decl(&config),
        TransferPage::decl(&config),
        TaskIdRequest::decl(&config),
        TaskAccepted::decl(&config),
        TaskSnapshot::decl(&config),
        TaskPage::decl(&config),
        CancelTaskResponse::decl(&config),
        TaskUpdatedEvent::decl(&config),
    ];
    let mut output =
        "// Generated from Rust contracts. Do not edit. Run npm.cmd run contracts:generate.\n"
            .to_owned();
    output.push_str(&format!(
        "export const IPC_CONTRACT_VERSION = {CONTRACT_VERSION} as const;\n"
    ));
    output.push_str(&format!(
        "export const IMPLEMENTED_COMMANDS = {} as const;\n",
        serde_json::to_string(&IMPLEMENTED_COMMANDS).unwrap()
    ));
    output.push_str(&format!(
        "export const DRAFT_COMMANDS = {} as const;\n",
        serde_json::to_string(&DRAFT_COMMANDS).unwrap()
    ));
    output.push_str(&format!(
        "export const ERROR_CODES = {} as const;\n",
        serde_json::to_string(ERROR_CODES.as_slice()).unwrap()
    ));
    output.push_str(&format!(
        "export const ERROR_FIELDS = {} as const;\n",
        serde_json::to_string(&ERROR_FIELDS).unwrap()
    ));
    output.push_str(&format!(
        "export const DRAFT_TASK_UPDATED_EVENT = {TASK_UPDATED_EVENT:?} as const;\n"
    ));
    output.push_str(&format!(
        "export const FILE_FORMATS = {} as const;\n",
        serde_json::to_string(crate::identification::FORMATS).unwrap()
    ));
    output.push_str(&format!(
        "export const PLATFORM_IDS = {} as const;\n",
        serde_json::to_string(crate::identification::PLATFORMS).unwrap()
    ));
    output.push_str(&format!(
        "export const IDENTIFICATION_EVIDENCE = {} as const;\n",
        serde_json::to_string(crate::identification::EVIDENCE).unwrap()
    ));
    for declaration in declarations {
        output.push_str("export ");
        output.push_str(&declaration);
        output.push('\n');
    }
    output
}

pub fn examples() -> String {
    let app_info = AppInfo {
        version: "0.1.0".to_owned(),
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
    };
    let database_info = DatabaseInfo {
        schema_version: 4,
        file_count: 0,
        location_count: 0,
        item_count: 0,
        task_count: 0,
    };
    let error = IpcError::new(ErrorCode::DatabaseUnavailable);
    let event = TaskUpdatedEvent {
        contract_version: CONTRACT_VERSION,
        task: TaskSnapshot {
            id: "1".into(),
            kind: TaskKind::Import,
            status: TaskStatus::Running,
            processed_count: "2".into(),
            total_count: Some("5".into()),
            revision: "3".into(),
            updated_at: "2026-11-04T12:00:00.000Z".into(),
            error: None,
        },
    };
    let descriptor = crate::ever::parse(include_bytes!(
        "../../../../../packages/g4e-format/examples/original-files.g4e"
    ))
    .unwrap();
    let magnet = descriptor.torrents[0].magnet.clone();
    let ever_open = EverOpenResult {
        cancelled: false,
        manifest: Some(EverSummary {
            file_name: "original-files.g4e".into(),
            schema_version: 1,
            representation: EverRepresentation::OriginalFiles,
            title: descriptor.manifest.collection.title,
            file_count: 3,
            total_bytes: "40".into(),
            magnets: vec![magnet.clone()],
            content_verified: false,
        }),
    };
    let ever_export = EverExportResult {
        cancelled: false,
        file_name: Some("original-files.g4e".into()),
        torrent_file_name: Some("original-files.torrent".into()),
        magnet: Some(magnet),
    };
    let transfer = TransferSnapshot {
        id: "7b22b2eb-a47e-43e2-975b-8c01b541db17".into(),
        name: "originals".into(),
        info_hash: "f0d6cd560e79272cf3acd08e0ee78e193b535c04".into(),
        mode: TransferMode::Download,
        status: TransferStatus::Completed,
        total_bytes: "40".into(),
        verified_bytes: "40".into(),
        uploaded_bytes: "0".into(),
        listen_port: None,
        output_name: Some("game4-download-example".into()),
        error: None,
    };
    let torrent_import = TorrentImportResult {
        cancelled: false,
        preview: Some(TorrentPreview {
            token: "7b22b2eb-a47e-43e2-975b-8c01b541db17".into(),
            name: "originals".into(),
            info_hash: transfer.info_hash.clone(),
            total_bytes: "40".into(),
            files: vec![TorrentFileEntry {
                path: "game.bin".into(),
                size_bytes: "40".into(),
            }],
        }),
    };
    format!("{}\n", serde_json::to_string_pretty(&serde_json::json!({"appInfo": app_info, "databaseInfo": database_info, "error": error, "draftTaskEvent": event,"everOpen":ever_open,"everExport":ever_export,"torrentImport":torrent_import,"transfer":transfer})).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_contracts_match_rust_types_and_serialization() {
        let directory = target_directory();
        assert_eq!(
            std::fs::read_to_string(directory.join("src/generated.ts")).unwrap(),
            typescript(),
            "Run contracts:generate and commit updated bindings"
        );
        assert_eq!(
            std::fs::read_to_string(directory.join("examples.json")).unwrap(),
            examples(),
            "Run contracts:generate and commit updated examples"
        );
    }

    #[test]
    fn wire_vocabulary_and_nulls_are_stable() {
        assert_eq!(
            serde_json::to_value(TaskStatus::Interrupted).unwrap(),
            "interrupted"
        );
        assert_eq!(
            serde_json::to_value(VerificationStatus::Inaccessible).unwrap(),
            "inaccessible"
        );
        let error = serde_json::to_value(IpcError::new(ErrorCode::InvalidArgument)).unwrap();
        assert_eq!(error["code"], "INVALID_ARGUMENT");
        assert!(error["field"].is_null());
        let values: serde_json::Value = serde_json::from_str(&examples()).unwrap();
        assert_eq!(values["draftTaskEvent"]["task"]["processedCount"], "2");
        assert!(values["draftTaskEvent"]["task"]["error"].is_null());
    }
    #[test]
    fn advertised_commands_match_registry_and_exclude_drafts() {
        let info = crate::commands::application::app_info();
        assert_eq!(info.contract_version, CONTRACT_VERSION);
        let active: Vec<String> = IMPLEMENTED_COMMANDS
            .iter()
            .map(|value| {
                serde_json::to_value(value)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect();
        assert_eq!(info.capabilities, active);
        let unique: std::collections::HashSet<_> = active.iter().collect();
        assert_eq!(unique.len(), active.len());
        for command in DRAFT_COMMANDS {
            let name = serde_json::to_value(command).unwrap();
            assert!(!active.iter().any(|active| active == name.as_str().unwrap()));
        }
    }
}
