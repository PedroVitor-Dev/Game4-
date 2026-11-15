use crate::{
    contracts::*,
    g4ep::{guardian::GuardianNode, manifest, ports::StorageNode, types::*},
};
use std::{
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub struct GuardianState {
    pub node: Mutex<Option<Arc<GuardianNode>>>,
    pub base: PathBuf,
}
fn error() -> IpcError {
    IpcError::new(ErrorCode::StorageUnavailable)
}
fn mapped(e: Error) -> IpcError {
    IpcError::new(match e {
        Error::GuardianDisabled => ErrorCode::GuardianDisabled,
        Error::GuardianNotApproved => ErrorCode::GuardianNotApproved,
        Error::InvalidManifest => ErrorCode::InvalidArgument,
        Error::QuotaExceeded => ErrorCode::GuardianQuotaExceeded,
        Error::IntegrityMismatch => ErrorCode::G4eIntegrityFailed,
        _ => ErrorCode::StorageUnavailable,
    })
}
fn context() -> OperationContext {
    OperationContext {
        cancellation: tokio_util::sync::CancellationToken::new(),
        deadline: Instant::now() + Duration::from_secs(3600),
    }
}
impl GuardianState {
    pub fn new(base: PathBuf) -> Self {
        Self {
            node: Mutex::new(None),
            base,
        }
    }
    fn get(&self) -> std::result::Result<Arc<GuardianNode>, IpcError> {
        let mut state = self.node.lock().map_err(|_| error())?;
        if let Some(node) = state.as_ref() {
            return Ok(Arc::clone(node));
        }
        let pointer = self.base.join("guardian-directory.json");
        let path = if pointer.exists() {
            let f = crate::library::open_source(&pointer)?;
            if f.metadata().map_err(|_| error())?.len() > 32768 {
                return Err(error());
            }
            serde_json::from_reader::<_, PathBuf>(f).map_err(|_| error())?
        } else {
            self.base.join("guardian")
        };
        let node = Arc::new(GuardianNode::open(&path).map_err(mapped)?);
        *state = Some(Arc::clone(&node));
        Ok(node)
    }
}
#[tauri::command]
pub async fn guardian_request(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, Arc<GuardianState>>,
    request: GuardianRequest,
) -> std::result::Result<GuardianSnapshot, IpcError> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let mut node = state.get()?;
        let mut offset = 0;
        match request {
            GuardianRequest::Status { offset: n } => offset = n,
            GuardianRequest::Configure { config } => node.configure(config).map_err(mapped)?,
            GuardianRequest::ChooseDirectory => {
                if node.snapshot(0).map_err(mapped)?.config.active {
                    return Err(IpcError::new(ErrorCode::InvalidArgument));
                }
                if let Some(path) = rfd::FileDialog::new().set_parent(&window).pick_folder() {
                    let next = Arc::new(GuardianNode::open(&path).map_err(mapped)?);
                    std::fs::create_dir_all(&state.base).map_err(|_| error())?;
                    let mut temp =
                        tempfile::NamedTempFile::new_in(&state.base).map_err(|_| error())?;
                    serde_json::to_writer(&mut temp, &path).map_err(|_| error())?;
                    temp.flush().map_err(|_| error())?;
                    temp.as_file().sync_all().map_err(|_| error())?;
                    temp.persist(state.base.join("guardian-directory.json"))
                        .map_err(|_| error())?;
                    *state.node.lock().map_err(|_| error())? = Some(Arc::clone(&next));
                    node = next;
                }
            }
            GuardianRequest::Approve {
                group,
                index,
                sharing,
            } => {
                let n = group
                    .parse::<u64>()
                    .ok()
                    .filter(|v| v.to_string() == group)
                    .ok_or_else(|| IpcError::new(ErrorCode::InvalidArgument))?;
                if let Some(path) = rfd::FileDialog::new()
                    .set_parent(&window)
                    .add_filter("Game4 Ever Manifest", &["g4e"])
                    .pick_file()
                {
                    let doc = manifest::read(&path)?;
                    node.approve(&doc, n, index, sharing).map_err(mapped)?;
                }
            }
            GuardianRequest::Import { id } => {
                let f = node.entry(id).map_err(mapped)?;
                if let Some(path) = rfd::FileDialog::new()
                    .set_parent(&window)
                    .add_filter("G4EP fragment", &["g4frag"])
                    .pick_file()
                {
                    let mut source = crate::library::open_source(&path)?;
                    node.commit_fragment(&f, &mut source, &context())
                        .map_err(mapped)?;
                }
            }
            GuardianRequest::Export { id } => {
                let f = node.entry(id).map_err(mapped)?;
                if let Some(path) = rfd::FileDialog::new()
                    .set_parent(&window)
                    .set_file_name(format!("{}.g4frag", crate::g4ep::local::hex(f.sha256)))
                    .save_file()
                {
                    crate::library::reject_ancestors(path.parent().ok_or_else(error)?)?;
                    let mut source = node.read_fragment(&f, &context()).map_err(mapped)?;
                    let mut temp =
                        tempfile::NamedTempFile::new_in(path.parent().ok_or_else(error)?)
                            .map_err(|_| error())?;
                    std::io::copy(&mut source, &mut temp).map_err(|_| error())?;
                    temp.as_file().sync_all().map_err(|_| error())?;
                    temp.persist_noclobber(path).map_err(|_| error())?;
                }
            }
            GuardianRequest::Revoke { id, delete } => node.revoke(id, delete).map_err(mapped)?,
            GuardianRequest::Verify => node.verify_all(&context()).map_err(mapped)?,
            GuardianRequest::Prune => node.prune().map_err(mapped)?,
        }
        node.snapshot(offset).map_err(mapped)
    })
    .await
    .map_err(|_| error())?
}
