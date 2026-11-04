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
}

pub const IMPLEMENTED_COMMANDS: [ImplementedCommand; 2] = [
    ImplementedCommand::AppInfo,
    ImplementedCommand::DatabaseInfo,
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum DraftCommand {
    LibrarySelectSources,
    LibraryImport,
    LibraryVerify,
    LibraryList,
    TasksList,
    TaskGet,
    TaskCancel,
}
pub const DRAFT_COMMANDS: [DraftCommand; 7] = [
    DraftCommand::LibrarySelectSources,
    DraftCommand::LibraryImport,
    DraftCommand::LibraryVerify,
    DraftCommand::LibraryList,
    DraftCommand::TasksList,
    DraftCommand::TaskGet,
    DraftCommand::TaskCancel,
];

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
}

pub const ERROR_CODES: [ErrorCode; 10] = [
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

// Drafts: type declarations only. No handlers, subscriptions, or mutations exist yet.
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
