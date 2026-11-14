//! Version 1 desktop IPC DTOs. Draft DTOs do not register Tauri commands.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const CONTRACT_VERSION: u32 = 1;
pub const TASK_UPDATED_EVENT: &str = "game4://task-updated";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ImplementedCommand {
    AppInfo,
    DatabaseInfo,
    LibrarySelectSources,
    LibraryImport,
    LibraryVerify,
    LibraryIntegrity,
    LibraryList,
    LibraryItemFiles,
    TasksList,
    TaskGet,
    LibraryExportManifest,
    LibraryExportPackage,
    PackageOpen,
    LibraryExportEver,
    EverOpen,
    TransferMetricsInfo,
    TrackersValidate,
    MagnetGenerate,
    MagnetResolve,
    MagnetCancel,
    TorrentCreate,
    TorrentImport,
    TorrentSeed,
    TorrentDownload,
    TransfersList,
    TransferStop,
    TransferPause,
    TransferResume,
    TransferForget,
    LibrarySaveBackup,
    LibraryPrepareRestore,
    LibraryRestoreBackup,
}

pub const IMPLEMENTED_COMMANDS: [ImplementedCommand; 32] = [
    ImplementedCommand::AppInfo,
    ImplementedCommand::DatabaseInfo,
    ImplementedCommand::LibrarySelectSources,
    ImplementedCommand::LibraryImport,
    ImplementedCommand::LibraryVerify,
    ImplementedCommand::LibraryIntegrity,
    ImplementedCommand::LibraryList,
    ImplementedCommand::LibraryItemFiles,
    ImplementedCommand::TasksList,
    ImplementedCommand::TaskGet,
    ImplementedCommand::LibraryExportManifest,
    ImplementedCommand::LibraryExportPackage,
    ImplementedCommand::PackageOpen,
    ImplementedCommand::LibraryExportEver,
    ImplementedCommand::EverOpen,
    ImplementedCommand::TransferMetricsInfo,
    ImplementedCommand::TrackersValidate,
    ImplementedCommand::MagnetGenerate,
    ImplementedCommand::MagnetResolve,
    ImplementedCommand::MagnetCancel,
    ImplementedCommand::TorrentCreate,
    ImplementedCommand::TorrentImport,
    ImplementedCommand::TorrentSeed,
    ImplementedCommand::TorrentDownload,
    ImplementedCommand::TransfersList,
    ImplementedCommand::TransferStop,
    ImplementedCommand::TransferPause,
    ImplementedCommand::TransferResume,
    ImplementedCommand::TransferForget,
    ImplementedCommand::LibrarySaveBackup,
    ImplementedCommand::LibraryPrepareRestore,
    ImplementedCommand::LibraryRestoreBackup,
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum DraftCommand {
    TaskCancel,
}
pub const DRAFT_COMMANDS: [DraftCommand; 1] = [DraftCommand::TaskCancel];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    IpcUnavailable,
    ContractMismatch,
    InvalidResponse,
    UnexpectedError,
    DatabaseUnavailable,
    DatabaseTaskFailed,
    InvalidArgument,
    NotFound,
    TaskNotCancellable,
    CapabilityUnavailable,
    SourceUnavailable,
    SourceChanged,
    ImportLimitExceeded,
    VerificationLimitExceeded,
    ManifestInvalid,
    ManifestVersionUnsupported,
    ManifestWriteFailed,
    BackupInvalid,
    BackupVersionUnsupported,
    BackupWriteFailed,
    LibraryBusy,
    RestoreStale,
    RestoreFailed,
    PackageInvalid,
    PackageVersionUnsupported,
    PackageWriteFailed,
    PackageIntegrityFailed,
    PackageLimitExceeded,
    G4eInvalid,
    G4eUnsupportedVersion,
    G4eLimitExceeded,
    G4eIntegrityFailed,
    G4eWriteFailed,
    TorrentInvalid,
    TransferLimitExceeded,
    TransferFailed,
    TransferUnavailable,
}

pub const ERROR_CODES: [ErrorCode; 37] = [
    ErrorCode::IpcUnavailable,
    ErrorCode::ContractMismatch,
    ErrorCode::InvalidResponse,
    ErrorCode::UnexpectedError,
    ErrorCode::DatabaseUnavailable,
    ErrorCode::DatabaseTaskFailed,
    ErrorCode::InvalidArgument,
    ErrorCode::NotFound,
    ErrorCode::TaskNotCancellable,
    ErrorCode::CapabilityUnavailable,
    ErrorCode::SourceUnavailable,
    ErrorCode::SourceChanged,
    ErrorCode::ImportLimitExceeded,
    ErrorCode::VerificationLimitExceeded,
    ErrorCode::ManifestInvalid,
    ErrorCode::ManifestVersionUnsupported,
    ErrorCode::ManifestWriteFailed,
    ErrorCode::BackupInvalid,
    ErrorCode::BackupVersionUnsupported,
    ErrorCode::BackupWriteFailed,
    ErrorCode::LibraryBusy,
    ErrorCode::RestoreStale,
    ErrorCode::RestoreFailed,
    ErrorCode::PackageInvalid,
    ErrorCode::PackageVersionUnsupported,
    ErrorCode::PackageWriteFailed,
    ErrorCode::PackageIntegrityFailed,
    ErrorCode::PackageLimitExceeded,
    ErrorCode::G4eInvalid,
    ErrorCode::G4eUnsupportedVersion,
    ErrorCode::G4eLimitExceeded,
    ErrorCode::G4eIntegrityFailed,
    ErrorCode::G4eWriteFailed,
    ErrorCode::TorrentInvalid,
    ErrorCode::TransferLimitExceeded,
    ErrorCode::TransferFailed,
    ErrorCode::TransferUnavailable,
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ErrorField {
    Request,
    Cursor,
    Limit,
    TaskId,
    ItemId,
    SelectionToken,
    RequestId,
}

pub const ERROR_FIELDS: [ErrorField; 7] = [
    ErrorField::Request,
    ErrorField::Cursor,
    ErrorField::Limit,
    ErrorField::TaskId,
    ErrorField::ItemId,
    ErrorField::SelectionToken,
    ErrorField::RequestId,
];

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct IpcError {
    pub code: ErrorCode,
    pub retryable: bool,
    pub field: Option<ErrorField>,
}

impl IpcError {
    pub fn new(code: ErrorCode) -> Self {
        Self {
            code,
            retryable: false,
            field: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub contract_version: u32,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseInfo {
    pub schema_version: u32,
    #[ts(type = "number")]
    pub file_count: u64,
    #[ts(type = "number")]
    pub location_count: u64,
    #[ts(type = "number")]
    pub item_count: u64,
    #[ts(type = "number")]
    pub task_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    Import,
    Verify,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Interrupted,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Unverified,
    Verified,
    Changed,
    Missing,
    Inaccessible,
}

// Shared library/task DTOs. Cancellation and events remain drafts.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PageRequest {
    pub cursor: Option<String>,
    pub limit: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LibraryListRequest {
    pub page: PageRequest,
    pub query: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LibraryItemSummary {
    pub formats: Vec<FormatSummary>,
    pub detected_platforms: Vec<PlatformSummary>,
    pub id: String,
    pub title: String,
    pub platform: Option<String>,
    pub file_count: u32,
    pub total_bytes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPage {
    pub items: Vec<LibraryItemSummary>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Files,
    Directories,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SelectSourcesRequest {
    pub kind: SourceKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SourceSelection {
    pub selection_token: Option<String>,
    pub display_names: Vec<String>,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StartImportRequest {
    pub request_id: String,
    pub selection_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StartVerifyRequest {
    pub request_id: String,
    pub item_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TaskIdRequest {
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TaskAccepted {
    pub task_id: String,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TaskSnapshot {
    pub id: String,
    pub kind: TaskKind,
    pub status: TaskStatus,
    pub processed_count: String,
    pub total_count: Option<String>,
    pub revision: String,
    pub updated_at: String,
    pub error: Option<IpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TaskPage {
    pub items: Vec<TaskSnapshot>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CancelTaskResponse {
    pub task_id: String,
    pub cancellation_requested: bool,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TaskUpdatedEvent {
    pub contract_version: u32,
    pub task: TaskSnapshot,
}

pub mod export;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ExportManifestRequest {
    pub item_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ExportManifestResult {
    pub cancelled: bool,
    pub file_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LibraryBackupResult {
    pub cancelled: bool,
    pub file_name: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRestorePreview {
    pub restore_token: String,
    pub file_name: String,
    pub item_count: u32,
    pub file_count: u32,
    pub location_count: u32,
    pub task_count: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPrepareRestoreResult {
    pub cancelled: bool,
    pub preview: Option<LibraryRestorePreview>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRestoreRequest {
    pub restore_token: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRestoreResult {
    pub item_count: u32,
    pub file_count: u32,
    pub location_count: u32,
    pub task_count: u32,
    pub recovery_file_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum FileFormat {
    Unknown,
    Ines,
    N64Z64,
    N64V64,
    N64N64,
    SnesRom,
    GameBoyRom,
    GameBoyColorRom,
    GbaRom,
    NdsRom,
    N3dsRom,
    Cia,
    Nsp,
    Xci,
    MasterSystemRom,
    GameGearRom,
    MegaDriveRom,
    PcEngineRom,
    Adf,
    D64,
    Zip,
    SevenZip,
    Rar,
    Gzip,
    Chd,
    Iso9660,
    Cue,
    Pe,
    Elf,
    PsxExe,
    Xbe,
    DoomWad,
    RawBinary,
    RawRom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum PlatformId {
    Nes,
    Snes,
    GameBoy,
    GameBoyColor,
    GameBoyAdvance,
    NintendoDs,
    Nintendo3ds,
    NintendoSwitch,
    Nintendo64,
    MasterSystem,
    GameGear,
    MegaDrive,
    PcEngine,
    Amiga,
    Commodore64,
    Windows,
    Playstation,
    Xbox,
    Android,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum IdentificationEvidence {
    Signature,
    Extension,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FormatSummary {
    pub format: FileFormat,
    pub evidence: IdentificationEvidence,
    pub file_count: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PlatformSummary {
    pub platform: PlatformId,
    pub evidence: IdentificationEvidence,
    pub file_count: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LibraryFileInfo {
    pub path: String,
    pub size_bytes: String,
    pub sha256: String,
    pub format: FileFormat,
    pub format_evidence: IdentificationEvidence,
    pub platform: Option<PlatformId>,
    pub platform_evidence: IdentificationEvidence,
    pub detector_version: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LibraryItemFilesRequest {
    pub item_id: String,
    pub page: PageRequest,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LibraryItemFilesPage {
    pub files: Vec<LibraryFileInfo>,
    pub next_cursor: Option<String>,
}

/// Latest stored observations, not a guarantee about current filesystem state.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LibraryIntegrity {
    pub verified: u32,
    pub changed: u32,
    pub missing: u32,
    pub inaccessible: u32,
    pub unverified: u32,
    pub unlocated: u32,
    pub last_checked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PackageSummary {
    pub file_name: String,
    pub package_version: u32,
    pub collection_id: String,
    pub title: String,
    pub file_count: u32,
    pub total_bytes: String,
    pub verified: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PackageOpenResult {
    pub cancelled: bool,
    pub package: Option<PackageSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum EverRepresentation {
    OriginalFiles,
    G4Package,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EverSummary {
    pub file_name: String,
    pub schema_version: u32,
    pub representation: EverRepresentation,
    pub title: String,
    pub file_count: u32,
    pub total_bytes: String,
    pub magnets: Vec<String>,
    pub content_verified: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EverOpenResult {
    pub cancelled: bool,
    pub manifest: Option<EverSummary>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EverExportResult {
    pub cancelled: bool,
    pub file_name: Option<String>,
    pub torrent_file_name: Option<String>,
    pub magnet: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TorrentFileEntry {
    pub path: String,
    pub size_bytes: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TorrentPreview {
    pub token: String,
    pub name: String,
    pub info_hash: String,
    pub total_bytes: String,
    pub files: Vec<TorrentFileEntry>,
    pub trackers: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TorrentImportResult {
    pub cancelled: bool,
    pub preview: Option<TorrentPreview>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TorrentCreateResult {
    pub cancelled: bool,
    pub file_name: Option<String>,
    pub magnet: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StartSeedRequest {
    pub request_id: String,
    pub item_id: String,
    pub listen_port: u16,
    #[serde(default)]
    pub dht: bool,
    #[serde(default)]
    pub trackers: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StartDownloadRequest {
    pub request_id: String,
    pub token: String,
    pub peer: String,
    #[serde(default)]
    pub dht: bool,
    #[serde(default)]
    pub trackers: Vec<String>,
    #[serde(default)]
    pub seed_after_download: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MagnetLink {
    pub magnet: String,
    pub info_hash: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ResolveMagnetRequest {
    pub request_id: String,
    pub magnet: String,
    pub peer: String,
    pub dht: bool,
    #[serde(default)]
    pub trackers: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CancelMagnetRequest {
    pub request_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CancelMagnetResult {
    pub cancelled: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TransferIdRequest {
    pub transfer_id: String,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransferMode {
    Seed,
    Download,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransferStatus {
    Paused,
    Resuming,
    Active,
    Stopping,
    Stopped,
    Completed,
    Failed,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TransferSnapshot {
    pub id: String,
    pub name: String,
    pub info_hash: String,
    pub mode: TransferMode,
    pub status: TransferStatus,
    pub total_bytes: String,
    pub verified_bytes: String,
    pub uploaded_bytes: String,
    pub listen_port: Option<u16>,
    pub dht: bool,
    pub output_name: Option<String>,
    pub error: Option<IpcError>,
    pub tracker_count: u32,
    pub received_bytes: String,
    pub seed_after_download: bool,
    pub is_seeding: bool,
    pub telemetry: Option<TransferTelemetry>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TransferStartResult {
    pub cancelled: bool,
    pub transfer: Option<TransferSnapshot>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TransferPage {
    pub items: Vec<TransferSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TrackerList {
    pub trackers: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TorrentReferenceRequest {
    pub item_id: String,
    #[serde(default)]
    pub trackers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TransferTelemetry {
    pub connected_peers: u32,
    pub seeders: u32,
    pub leechers: u32,
    pub download_bps: String,
    pub upload_bps: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TransferMetricsInfo {
    pub connected_peer_metrics: bool,
    pub seed_after_download: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TransferForgetResult {
    pub forgotten: bool,
}
