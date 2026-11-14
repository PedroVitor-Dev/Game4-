//! Private storage preferences. Changing defaults never relocates payload or catalog data.
use crate::{contracts::*, library};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};
type Result<T> = std::result::Result<T, IpcError>;
const MAX_SETTINGS: u64 = 65536;
fn unavailable(_: impl std::fmt::Debug) -> IpcError {
    IpcError::new(ErrorCode::StorageUnavailable)
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Preferences {
    version: u32,
    downloads: Option<PathBuf>,
    exports: Option<PathBuf>,
    backups: Option<PathBuf>,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            version: 1,
            downloads: None,
            exports: None,
            backups: None,
        }
    }
}
impl Preferences {
    fn directory(&self, kind: StorageDirectoryKind) -> &Option<PathBuf> {
        match kind {
            StorageDirectoryKind::Downloads => &self.downloads,
            StorageDirectoryKind::Exports => &self.exports,
            StorageDirectoryKind::Backups => &self.backups,
        }
    }
    fn set(&mut self, kind: StorageDirectoryKind, path: Option<PathBuf>) {
        *match kind {
            StorageDirectoryKind::Downloads => &mut self.downloads,
            StorageDirectoryKind::Exports => &mut self.exports,
            StorageDirectoryKind::Backups => &mut self.backups,
        } = path;
    }
}
pub struct StorageSettings {
    root: PathBuf,
    gate: Mutex<()>,
}
impl StorageSettings {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            gate: Mutex::new(()),
        }
    }
    fn path(&self) -> PathBuf {
        self.root.join("settings/storage.json")
    }
    fn read(&self) -> Result<Preferences> {
        library::reject_ancestors(&self.root).map_err(unavailable)?;
        let path = self.path();
        let parent = path.parent().unwrap();
        if parent.try_exists().map_err(unavailable)? {
            library::reject_ancestors(parent).map_err(unavailable)?;
        }
        match fs::symlink_metadata(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Preferences::default()),
            Err(e) => return Err(unavailable(e)),
            Ok(_) => {}
        }
        let f = library::open_source(&path).map_err(unavailable)?;
        if f.metadata().map_err(unavailable)?.len() > MAX_SETTINGS {
            return Err(unavailable("size"));
        }
        let mut bytes = Vec::new();
        f.take(MAX_SETTINGS + 1)
            .read_to_end(&mut bytes)
            .map_err(unavailable)?;
        if bytes.len() as u64 > MAX_SETTINGS {
            return Err(unavailable("size"));
        }
        let p: Preferences = serde_json::from_slice(&bytes).map_err(unavailable)?;
        if p.version != 1 {
            return Err(unavailable("version"));
        }
        for path in [&p.downloads, &p.exports, &p.backups].into_iter().flatten() {
            if !path.is_absolute() || path.to_string_lossy().len() > 32768 {
                return Err(unavailable("path"));
            }
        }
        Ok(p)
    }
    fn info(&self, p: &Preferences) -> StorageSettingsInfo {
        let show = |kind| {
            p.directory(kind).as_ref().map(|path| StorageDirectoryInfo {
                path: path.to_string_lossy().into_owned(),
                available: path.is_dir() && library::reject_ancestors(path).is_ok(),
            })
        };
        StorageSettingsInfo {
            downloads: show(StorageDirectoryKind::Downloads),
            exports: show(StorageDirectoryKind::Exports),
            backups: show(StorageDirectoryKind::Backups),
            app_data_directory: self.root.to_string_lossy().into_owned(),
        }
    }
    pub fn get(&self) -> Result<StorageSettingsInfo> {
        let _gate = self.gate.lock().map_err(unavailable)?;
        Ok(self.info(&self.read()?))
    }
    pub fn initial_directory(&self, kind: StorageDirectoryKind) -> Result<Option<PathBuf>> {
        let _gate = self.gate.lock().map_err(unavailable)?;
        let p = self.read()?;
        let path = p.directory(kind).clone();
        if let Some(path) = &path {
            validate_directory(path)?;
        }
        Ok(path)
    }
    pub fn change(
        &self,
        kind: StorageDirectoryKind,
        path: Option<PathBuf>,
    ) -> Result<StorageSettingsInfo> {
        let _gate = self.gate.lock().map_err(unavailable)?;
        let mut p = self.read()?;
        if let Some(path) = &path {
            validate_directory(path)?;
        }
        p.set(kind, path);
        let bytes = serde_json::to_vec_pretty(&p).map_err(unavailable)?;
        if bytes.len() as u64 > MAX_SETTINGS {
            return Err(unavailable("size"));
        }
        let path = self.path();
        let parent = path.parent().unwrap();
        fs::create_dir_all(parent).map_err(unavailable)?;
        library::reject_ancestors(parent).map_err(unavailable)?;
        let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(unavailable)?;
        temp.write_all(&bytes).map_err(unavailable)?;
        temp.as_file().sync_all().map_err(unavailable)?;
        temp.persist(&path).map_err(unavailable)?;
        Ok(self.info(&p))
    }
}
fn validate_directory(path: &Path) -> Result<()> {
    if !path.is_absolute() || path.to_string_lossy().len() > 32768 || !path.is_dir() {
        return Err(unavailable("directory"));
    }
    library::reject_ancestors(path).map_err(unavailable)?;
    // A private temporary probe is deleted on drop; existing user files are never touched.
    let probe = tempfile::NamedTempFile::new_in(path).map_err(unavailable)?;
    probe.as_file().sync_all().map_err(unavailable)?;
    probe.close().map_err(unavailable)
}
/// Each operation still asks the user to confirm a path in a native dialog.
pub fn dialog(storage: &StorageSettings, kind: StorageDirectoryKind) -> Result<rfd::FileDialog> {
    let dialog = rfd::FileDialog::new();
    Ok(match storage.initial_directory(kind)? {
        Some(path) => dialog.set_directory(path),
        None => dialog,
    })
}
#[cfg(test)]
mod tests;
