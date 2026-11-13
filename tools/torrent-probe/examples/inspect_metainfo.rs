//! Independent parser-only interoperability check. Never starts a session.
use anyhow::{Result, ensure};
use std::{fs::File, io::Read};
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    ensure!(args.len() == 1, "Usage: inspect_metainfo FILE.torrent");
    let mut bytes = Vec::new();
    File::open(&args[0])?
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 8 * 1024 * 1024, "Metainfo exceeds 8 MiB");
    let torrent = librqbit::torrent_from_bytes(&bytes)?;
    let info = &torrent.info.data;
    let file_count = info.files.as_ref().map_or(1, |f| f.len());
    println!(
        "{}",
        serde_json::json!({"parsed":true,"fileCount":file_count,"pieceLength":info.piece_length,"pieceCount":info.pieces.0.len()/20,"magnet":librqbit::Magnet::from_id20(torrent.info_hash,vec![],None).to_string(),"networkAccess":false})
    );
    Ok(())
}
