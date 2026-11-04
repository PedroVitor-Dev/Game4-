//! Deterministic trackerless BitTorrent v1 metainfo (BEP 3).
//! SHA-1 is used only for protocol compatibility; preservation uses SHA-256.
use anyhow::{Context, Result, ensure};
use sha1::{Digest, Sha1};
use std::{
    fs::{self, File, Metadata},
    io::Read,
    path::{Path, PathBuf},
};

pub struct Metainfo {
    pub bytes: Vec<u8>,
    pub magnet: String,
}

struct Entry {
    path: PathBuf,
    relative: Vec<String>,
    metadata: Metadata,
}

fn metadata(path: &Path) -> Result<Metadata> {
    let meta = fs::symlink_metadata(path)?;
    ensure!(
        !meta.file_type().is_symlink(),
        "Symbolic links are not supported"
    );
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        ensure!(
            meta.file_attributes() & 0x400 == 0,
            "Windows reparse points are not supported"
        );
    }
    ensure!(
        meta.is_file() || meta.is_dir(),
        "Only regular files and directories are supported"
    );
    Ok(meta)
}

fn collect(path: &Path, relative: Vec<String>, entries: &mut Vec<Entry>) -> Result<()> {
    ensure!(
        relative.len() <= 64 && entries.len() < 100_000,
        "Collection exceeds probe limits"
    );
    let meta = metadata(path)?;
    if meta.is_file() {
        entries.push(Entry {
            path: path.to_owned(),
            relative,
            metadata: meta,
        });
    } else {
        for child in fs::read_dir(path)? {
            let child = child?;
            let mut relative = relative.clone();
            relative.push(
                child
                    .file_name()
                    .into_string()
                    .map_err(|_| anyhow::anyhow!("Filenames must be valid Unicode"))?,
            );
            collect(&child.path(), relative, entries)?;
        }
    }
    Ok(())
}

fn string(out: &mut Vec<u8>, value: &[u8]) {
    out.extend_from_slice(value.len().to_string().as_bytes());
    out.push(b':');
    out.extend_from_slice(value);
}

fn integer(out: &mut Vec<u8>, value: u64) {
    out.push(b'i');
    out.extend_from_slice(value.to_string().as_bytes());
    out.push(b'e');
}

pub fn create(path: &Path, piece_length: u32) -> Result<Metainfo> {
    ensure!(
        piece_length.is_power_of_two() && (16_384..=16_777_216).contains(&piece_length),
        "Piece length must be a power of two between 16 KiB and 16 MiB"
    );
    let single_file = metadata(path)?.is_file();
    let path = path.canonicalize()?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .context("Input needs a Unicode filename")?;
    let mut entries = Vec::new();
    collect(&path, Vec::new(), &mut entries)?;
    entries.sort_by(|left, right| left.relative.cmp(&right.relative));
    let total = entries.iter().try_fold(0u64, |total, entry| {
        total
            .checked_add(entry.metadata.len())
            .context("Collection size overflow")
    })?;
    ensure!(
        total > 0,
        "Empty datasets are not supported; empty files inside a nonempty collection are supported"
    );

    let mut pieces = Vec::new();
    let mut hasher = Sha1::new();
    let mut filled = 0usize;
    let mut buffer = [0u8; 64 * 1024];
    for entry in &entries {
        let mut file = File::open(&entry.path)?;
        let mut read_bytes = 0u64;
        loop {
            let limit = buffer.len().min(piece_length as usize - filled);
            let count = file.read(&mut buffer[..limit])?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
            filled += count;
            read_bytes += count as u64;
            if filled == piece_length as usize {
                pieces.extend_from_slice(&hasher.finalize_reset());
                filled = 0;
            }
        }
        ensure!(
            read_bytes == entry.metadata.len(),
            "Input changed while hashing"
        );
    }
    // Carry a partial piece across files, including trailing zero-byte files.
    // Never append the hash of an empty hasher at an exact piece boundary.
    if filled > 0 {
        pieces.extend_from_slice(&hasher.finalize());
    }
    for entry in &entries {
        let current = metadata(&entry.path)?;
        ensure!(
            current.len() == entry.metadata.len()
                && current.modified()? == entry.metadata.modified()?,
            "Input changed while hashing"
        );
    }
    ensure!(
        pieces.len() as u64 == total.div_ceil(piece_length as u64) * 20,
        "Internal piece geometry mismatch"
    );

    // Dictionary keys are emitted in canonical byte order, as required by BEP 3.
    let mut info = vec![b'd'];
    if single_file {
        string(&mut info, b"length");
        integer(&mut info, total);
    } else {
        string(&mut info, b"files");
        info.push(b'l');
        for entry in &entries {
            info.push(b'd');
            string(&mut info, b"length");
            integer(&mut info, entry.metadata.len());
            string(&mut info, b"path");
            info.push(b'l');
            for component in &entry.relative {
                string(&mut info, component.as_bytes());
            }
            info.extend_from_slice(b"ee");
        }
        info.push(b'e');
    }
    string(&mut info, b"name");
    string(&mut info, name.as_bytes());
    string(&mut info, b"piece length");
    integer(&mut info, piece_length as u64);
    string(&mut info, b"pieces");
    string(&mut info, &pieces);
    info.push(b'e');
    let info_hash = format!("{:x}", Sha1::digest(&info));
    let mut bytes = b"d4:info".to_vec();
    bytes.extend_from_slice(&info);
    bytes.push(b'e');
    // Decode with the transport library as an additional structural check.
    librqbit::torrent_from_bytes(&bytes)?;
    Ok(Metainfo {
        bytes,
        magnet: format!("magnet:?xt=urn:btih:{info_hash}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const PIECE: usize = 16 * 1024;

    fn assert_pieces(bytes: &[u8], data: &[u8]) {
        let torrent = librqbit::torrent_from_bytes(bytes).unwrap();
        let expected: Vec<u8> = data
            .chunks(PIECE)
            .flat_map(|chunk| Sha1::digest(chunk).to_vec())
            .collect();
        assert_eq!(torrent.info.data.pieces.0, expected.as_slice());
    }

    #[test]
    fn exact_and_partial_piece_boundaries_have_correct_hashes() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("payload.bin");
        for length in [1, PIECE - 1, PIECE, PIECE + 1, PIECE * 2, PIECE * 2 + 1] {
            let data: Vec<u8> = (0..length).map(|index| (index % 251) as u8).collect();
            fs::write(&path, &data).unwrap();
            let result = create(&path, PIECE as u32).unwrap();
            assert_pieces(&result.bytes, &data);
            assert_eq!(result.bytes, create(&path, PIECE as u32).unwrap().bytes);
        }
    }

    #[test]
    fn multifile_hashes_cross_files_and_keep_trailing_empty_files() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("collection");
        fs::create_dir_all(root.join("nested")).unwrap();
        let first = vec![3; PIECE - 3];
        let second = vec![9; 9];
        fs::write(root.join("a.bin"), &first).unwrap();
        fs::write(root.join("nested/β.bin"), &second).unwrap();
        fs::write(root.join("z-empty.bin"), []).unwrap();
        let result = create(&root, PIECE as u32).unwrap();
        assert_pieces(&result.bytes, &[first, second].concat());
        let torrent = librqbit::torrent_from_bytes(&result.bytes).unwrap();
        let files = torrent.info.data.files.as_ref().unwrap();
        assert_eq!(files.len(), 3);
        assert_eq!(files.last().unwrap().length, 0);
    }

    #[test]
    fn rejects_empty_datasets_and_invalid_piece_lengths() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("empty.bin");
        fs::write(&path, []).unwrap();
        assert!(create(&path, PIECE as u32).is_err());
        assert!(create(&path, 0).is_err());
        assert!(create(&path, 20_000).is_err());
    }
}
