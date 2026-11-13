//! Offline reference CLI. No database, network, extraction or execution.
use game4_desktop_lib::{manifests, packages};
use std::{path::PathBuf, sync::Arc};
fn main() {
    if let Err(message) = run() {
        eprintln!("{message}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let usage = "Usage: g4 read PACKAGE.g4 | g4 create --manifest MANIFEST.json --root DIRECTORY --output PACKAGE.g4";
    let fail = |e: game4_desktop_lib::contracts::IpcError| serde_json::to_string(&e).unwrap();
    if args.len() == 2 && args[0] == "read" {
        let result = packages::read(&PathBuf::from(&args[1])).map_err(fail)?;
        println!("{}",serde_json::to_string_pretty(&serde_json::json!({"packageVersion":packages::VERSION,"verified":true,"totalBytes":result.total_bytes.to_string(),"manifest":result.manifest})).unwrap());
        return Ok(());
    }
    if args.len() != 7
        || args[0] != "create"
        || args[1] != "--manifest"
        || args[3] != "--root"
        || args[5] != "--output"
    {
        return Err(usage.into());
    }
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(&args[2])
        .map_err(|_| "Cannot read manifest")?
        .take((manifests::MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read manifest")?;
    let manifest = manifests::parse(&bytes).map_err(fail)?;
    packages::validate_manifest(&manifest).map_err(fail)?;
    let root = PathBuf::from(&args[4]);
    // Resolve relative roots without canonicalizing away symlink/reparse ancestors.
    let root = if root.is_absolute() {
        root
    } else {
        std::env::current_dir()
            .map_err(|_| "Cannot resolve root")?
            .join(root)
    };
    if !root.is_dir() {
        return Err("Root must be a directory".into());
    }
    let sources: Vec<_> = manifest
        .files
        .iter()
        .map(|f| Arc::new(vec![root.join(&f.path)]))
        .collect();
    packages::create(&PathBuf::from(&args[6]), &manifest, &sources).map_err(fail)?;
    println!(
        "{}",
        serde_json::json!({"created":true,"packageVersion":packages::VERSION,"fileCount":manifest.files.len()})
    );
    Ok(())
}
