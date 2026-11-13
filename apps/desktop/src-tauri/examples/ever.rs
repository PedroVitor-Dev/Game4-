//! Offline .g4e reference CLI. Never connects or starts a torrent session.
use game4_desktop_lib::{ever, manifests};
use std::{io::Read, path::PathBuf};
fn main() {
    if let Err(message) = run() {
        eprintln!("{message}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let fail = |e: game4_desktop_lib::contracts::IpcError| serde_json::to_string(&e).unwrap();
    let usage = "Usage: ever read FILE.g4e | ever create --manifest FILE.json --root PATH --output FILE.g4e | ever create-package FILE.g4 --output FILE.g4e | ever verify --manifest FILE.g4e --torrent FILE.torrent --root PATH";
    fn bounded(path: &std::ffi::OsStr, max: usize) -> Result<Vec<u8>, String> {
        let mut data = Vec::new();
        std::fs::File::open(path)
            .map_err(|_| "Cannot read input")?
            .take((max + 1) as u64)
            .read_to_end(&mut data)
            .map_err(|_| "Cannot read input")?;
        if data.len() > max {
            return Err("Input exceeds limit".into());
        }
        Ok(data)
    }
    if args.len() == 2 && args[0] == "read" {
        let doc = ever::read(&PathBuf::from(&args[1])).map_err(fail)?;
        println!("{}", serde_json::to_string_pretty(&doc).unwrap());
        return Ok(());
    }
    let (created, out) = if args.len() == 7
        && args[0] == "create"
        && args[1] == "--manifest"
        && args[3] == "--root"
        && args[5] == "--output"
    {
        let m = manifests::parse(&bounded(&args[2], manifests::MAX_BYTES)?).map_err(fail)?;
        (
            ever::create(&m, &PathBuf::from(&args[4])).map_err(fail)?,
            PathBuf::from(&args[6]),
        )
    } else if args.len() == 4 && args[0] == "create-package" && args[2] == "--output" {
        (
            ever::create_package(&PathBuf::from(&args[1])).map_err(fail)?,
            PathBuf::from(&args[3]),
        )
    } else if args.len() == 7
        && args[0] == "verify"
        && args[1] == "--manifest"
        && args[3] == "--torrent"
        && args[5] == "--root"
    {
        let doc = ever::read(&PathBuf::from(&args[2])).map_err(fail)?;
        ever::verify(
            &doc,
            &bounded(&args[4], ever::MAX_METAINFO)?,
            &PathBuf::from(&args[6]),
        )
        .map_err(fail)?;
        println!(
            "{}",
            serde_json::json!({"verified":true,"networkAccess":false})
        );
        return Ok(());
    } else {
        return Err(usage.into());
    };
    let (name, torrent) = ever::write_pair(&out, &created).map_err(fail)?;
    println!(
        "{}",
        serde_json::json!({"created":true,"fileName":name,"torrentFileName":torrent,"magnet":created.document.torrents[0].magnet,"networkAccess":false})
    );
    Ok(())
}
