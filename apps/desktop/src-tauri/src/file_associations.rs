//! Bounded native-only launch inbox. Association opening is inspection, never execution.
use crate::{contracts::*, ever, packages};
use std::{
    collections::VecDeque,
    ffi::OsString,
    path::{Path, PathBuf},
    sync::Mutex,
};
use uuid::Uuid;
struct Pending {
    path: PathBuf,
    info: AssociatedFile,
}
#[derive(Default)]
pub struct FileInbox {
    pending: Mutex<VecDeque<Pending>>,
}
impl FileInbox {
    pub fn enqueue(&self, args: impl IntoIterator<Item = OsString>, cwd: &Path) {
        let mut queue = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        for arg in args.into_iter().take(32) {
            if queue.len() >= 16 {
                break;
            }
            let raw = PathBuf::from(arg);
            if raw.to_string_lossy().contains("://") || raw.to_string_lossy().starts_with("file:") {
                continue;
            }
            if raw.to_string_lossy().len() > 32768 || raw.to_string_lossy().starts_with('-') {
                continue;
            }
            let Some(ext) = raw.extension().and_then(|v| v.to_str()) else {
                continue;
            };
            let kind = if ext.eq_ignore_ascii_case("g4") {
                AssociatedFileKind::G4
            } else if ext.eq_ignore_ascii_case("g4e") {
                AssociatedFileKind::G4e
            } else {
                continue;
            };
            let path = if raw.is_absolute() {
                raw
            } else {
                cwd.join(raw)
            };
            if !path.is_absolute()
                || path.to_string_lossy().len() > 32768
                || queue.iter().any(|old| old.path == path)
            {
                continue;
            }
            let Some(name) = path.file_name() else {
                continue;
            };
            let info = AssociatedFile {
                token: Uuid::new_v4().to_string(),
                file_name: name.to_string_lossy().into_owned(),
                kind,
            };
            queue.push_back(Pending { path, info });
        }
    }
    pub fn next(&self) -> Option<AssociatedFile> {
        self.pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .front()
            .map(|p| p.info.clone())
    }
    pub fn read(&self, token: &str) -> Result<AssociatedFileResult, IpcError> {
        let pending = {
            let mut queue = self
                .pending
                .lock()
                .map_err(|_| IpcError::new(ErrorCode::UnexpectedError))?;
            if !queue.front().is_some_and(|p| p.info.token == token) {
                return Err(IpcError::new(ErrorCode::InvalidArgument));
            }
            queue.pop_front().unwrap()
        };
        match pending.info.kind {
            AssociatedFileKind::G4 => Ok(AssociatedFileResult {
                kind: pending.info.kind,
                package: Some(inspect_package(&pending.path)?),
                manifest: None,
            }),
            AssociatedFileKind::G4e => Ok(AssociatedFileResult {
                kind: pending.info.kind,
                package: None,
                manifest: Some(inspect_ever(&pending.path)?),
            }),
        }
    }
}
pub(crate) fn inspect_package(path: &Path) -> Result<PackageSummary, IpcError> {
    let verified = packages::read(path)?;
    Ok(PackageSummary {
        file_name: path
            .file_name()
            .ok_or_else(|| IpcError::new(ErrorCode::PackageInvalid))?
            .to_string_lossy()
            .into_owned(),
        package_version: packages::VERSION,
        collection_id: verified.manifest.collection.id,
        title: verified.manifest.collection.title,
        file_count: verified.manifest.files.len() as u32,
        total_bytes: verified.total_bytes.to_string(),
        verified: true,
    })
}
pub(crate) fn inspect_ever(path: &Path) -> Result<EverSummary, IpcError> {
    let doc = ever::read(path)?;
    let total = ever::validate(&doc)?;
    Ok(EverSummary {
        file_name: path
            .file_name()
            .ok_or_else(|| IpcError::new(ErrorCode::G4eInvalid))?
            .to_string_lossy()
            .into_owned(),
        schema_version: 1,
        representation: doc.representation,
        title: doc.manifest.collection.title,
        file_count: doc.manifest.files.len() as u32,
        total_bytes: total.to_string(),
        magnets: doc.torrents.into_iter().map(|t| t.magnet).collect(),
        content_verified: false,
    })
}
#[cfg(test)]
mod tests;
