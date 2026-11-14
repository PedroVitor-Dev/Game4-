//! Handle-only engine storage. Seeding cannot write, truncate or delete originals.
use super::*;
use anyhow::{Result, bail, ensure};
use librqbit::{
    ManagedTorrentShared, TorrentMetadata,
    storage::{BoxStorageFactory, StorageFactory, StorageFactoryExt, TorrentStorage},
};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    sync::Mutex,
};
pub(super) struct Slot {
    file: Mutex<File>,
    len: u64,
    path: PathBuf,
}
#[derive(Clone)]
pub(super) struct Handles {
    slots: Arc<Vec<Slot>>,
    read_only: Arc<AtomicBool>,
}
impl Handles {
    pub fn flush(&self) -> Result<()> {
        if !self.read_only.load(Ordering::Acquire) {
            for slot in self.slots.iter() {
                let f = slot.file.lock().map_err(|_| anyhow::anyhow!("file lock"))?;
                ensure!(f.metadata()?.len() == slot.len, "incomplete file");
                f.sync_all()?;
            }
        }
        Ok(())
    }
    pub fn seal(&self) -> Result<()> {
        self.flush()?;
        self.read_only.store(true, Ordering::Release);
        Ok(())
    }
    pub fn seed(root: &Path, meta: &Layout) -> Result<Self> {
        let mut slots = Vec::new();
        for (p, n) in &meta.files {
            let path = if root.is_dir() {
                root.join(p)
            } else {
                ensure!(
                    meta.files.len() == 1 && root.file_name().and_then(|s| s.to_str()) == Some(p),
                    "source layout changed"
                );
                root.into()
            };
            let f =
                library::open_source(&path).map_err(|_| anyhow::anyhow!("source unavailable"))?;
            ensure!(f.metadata()?.len() == *n, "source length changed");
            slots.push(Slot {
                file: Mutex::new(f),
                len: *n,
                path: PathBuf::from(p),
            });
        }
        Ok(Self {
            slots: Arc::new(slots),
            read_only: Arc::new(AtomicBool::new(true)),
        })
    }
    pub fn download(root: &Path, meta: &Layout) -> Result<Self> {
        let mut slots = Vec::new();
        for (p, n) in &meta.files {
            let path = root.join(p);
            let parent = path.parent().ok_or_else(|| anyhow::anyhow!("no parent"))?;
            std::fs::create_dir_all(parent)?;
            library::reject_ancestors(parent)
                .map_err(|_| anyhow::anyhow!("redirected directory"))?;
            let mut options = OpenOptions::new();
            options.create_new(true).read(true).write(true);
            #[cfg(windows)]
            {
                use std::os::windows::fs::OpenOptionsExt;
                options.share_mode(1).custom_flags(0x00200000);
            }
            let f = options.open(path)?;
            ensure!(f.metadata()?.is_file(), "not a regular output");
            slots.push(Slot {
                file: Mutex::new(f),
                len: *n,
                path: PathBuf::from(p),
            });
        }
        Ok(Self {
            slots: Arc::new(slots),
            read_only: Arc::new(AtomicBool::new(false)),
        })
    }
    fn slot(&self, id: usize, offset: u64, length: usize) -> Result<&Slot> {
        let s = self
            .slots
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("file index"))?;
        ensure!(
            offset
                .checked_add(length as u64)
                .is_some_and(|v| v <= s.len),
            "out of bounds IO"
        );
        Ok(s)
    }
}
impl StorageFactory for Handles {
    type Storage = Self;
    fn create(&self, _: &ManagedTorrentShared, _: &TorrentMetadata) -> Result<Self> {
        Ok(self.clone())
    }
    fn clone_box(&self) -> BoxStorageFactory {
        self.clone().boxed()
    }
}
impl TorrentStorage for Handles {
    fn init(&mut self, _: &ManagedTorrentShared, meta: &TorrentMetadata) -> Result<()> {
        ensure!(
            meta.file_infos.len() == self.slots.len(),
            "file count mismatch"
        );
        for (s, f) in self.slots.iter().zip(meta.file_infos.iter()) {
            ensure!(
                s.path == f.relative_filename && s.len == f.len && !f.attrs.padding,
                "metadata mismatch"
            );
        }
        Ok(())
    }
    fn pread_exact(&self, id: usize, offset: u64, buf: &mut [u8]) -> Result<()> {
        let s = self.slot(id, offset, buf.len())?;
        let mut f = s.file.lock().map_err(|_| anyhow::anyhow!("file lock"))?;
        f.seek(SeekFrom::Start(offset))?;
        f.read_exact(buf)?;
        Ok(())
    }
    fn pwrite_all(&self, id: usize, offset: u64, buf: &[u8]) -> Result<()> {
        ensure!(
            !self.read_only.load(Ordering::Acquire),
            "original is read-only"
        );
        let s = self.slot(id, offset, buf.len())?;
        let mut f = s.file.lock().map_err(|_| anyhow::anyhow!("file lock"))?;
        f.seek(SeekFrom::Start(offset))?;
        f.write_all(buf)?;
        Ok(())
    }
    fn ensure_file_length(&self, id: usize, len: u64) -> Result<()> {
        let s = self.slot(id, 0, 0)?;
        ensure!(s.len == len, "length mismatch");
        if self.read_only.load(Ordering::Acquire) {
            ensure!(
                s.file
                    .lock()
                    .map_err(|_| anyhow::anyhow!("file lock"))?
                    .metadata()?
                    .len()
                    == len,
                "original length changed"
            );
        }
        Ok(())
    } // Downloads grow on verified-position writes; no eager allocation/truncation.
    fn remove_file(&self, _: usize, _: &Path) -> Result<()> {
        bail!("storage never deletes files")
    }
    fn remove_directory_if_empty(&self, _: &Path) -> Result<()> {
        bail!("storage never deletes directories")
    }
    fn take(&self) -> Result<Box<dyn TorrentStorage>> {
        Ok(Box::new(self.clone()))
    }
}
