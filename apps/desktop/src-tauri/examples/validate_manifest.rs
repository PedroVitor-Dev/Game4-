//! Offline semantic validation; does not open referenced paths or modify data.
use game4_desktop_lib::{
    contracts::{ErrorCode, IpcError},
    manifests,
};
use std::{io::Read, process::ExitCode};
fn validate() -> Result<serde_json::Value, IpcError> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 1 {
        return Err(IpcError::new(ErrorCode::InvalidArgument));
    }
    let file =
        std::fs::File::open(&args[0]).map_err(|_| IpcError::new(ErrorCode::SourceUnavailable))?;
    let mut bytes = Vec::new();
    file.take((manifests::MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| IpcError::new(ErrorCode::SourceUnavailable))?;
    let manifest = manifests::parse(&bytes)?;
    Ok(
        serde_json::json!({"valid":true,"schemaVersion":manifest.schema_version,"collectionId":manifest.collection.id,"fileCount":manifest.files.len()}),
    )
}
fn main() -> ExitCode {
    match validate() {
        Ok(summary) => {
            println!("{summary}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            println!("{}", serde_json::json!({"valid":false,"error":error}));
            ExitCode::FAILURE
        }
    }
}
