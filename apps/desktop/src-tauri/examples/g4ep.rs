//! Offline G4EP step-2 utility: no app startup, database or network sessions.
use game4_desktop_lib::g4ep::{local, types::*};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1)
    }
}
fn run() -> std::result::Result<(), String> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let usage = "Usage: g4ep encode SOURCE --output NEW_DIRECTORY [--group-bytes 8388608] | g4ep verify DIRECTORY | g4ep verify-fragment DIRECTORY GROUP INDEX";
    let ctx = OperationContext {
        cancellation: CancellationToken::new(),
        deadline: Instant::now() + Duration::from_secs(3600),
    };
    let start = Instant::now();
    let error = |e: Error| e.to_string();
    let manifest = if (args.len() == 4 || args.len() == 6)
        && args[0] == "encode"
        && args[2] == "--output"
    {
        let group = if args.len() == 6 {
            if args[4] != "--group-bytes" {
                return Err(usage.into());
            }
            args[5]
                .to_str()
                .and_then(|s| s.parse::<u32>().ok())
                .ok_or(usage)?
        } else {
            8 * 1024 * 1024
        };
        if !group.is_multiple_of(8) {
            return Err("Group size must be divisible by 8".into());
        }
        let coding = CodingProfile::new(8, 4, group / 8).map_err(error)?;
        local::encode_file(
            &PathBuf::from(&args[1]),
            &PathBuf::from(&args[3]),
            coding,
            &ctx,
            &mut |_| {},
        )
        .map_err(error)?
    } else if args.len() == 2 && args[0] == "verify" {
        local::verify_bundle(&PathBuf::from(&args[1]), &ctx).map_err(error)?
    } else if args.len() == 4 && args[0] == "verify-fragment" {
        let root = PathBuf::from(&args[1]);
        let manifest = local::read_manifest(&root, &ctx).map_err(error)?;
        let group = args[2]
            .to_str()
            .and_then(|s| s.parse::<u64>().ok())
            .ok_or(usage)?;
        let index = args[3]
            .to_str()
            .and_then(|s| s.parse::<u8>().ok())
            .ok_or(usage)?;
        let fragment = manifest
            .manifest()
            .fragments
            .iter()
            .find(|f| {
                f.coordinate
                    == FragmentCoordinate {
                        stripe: group,
                        shard: index,
                    }
            })
            .ok_or("Fragment is not in the manifest")?;
        local::verify_fragment(&root, fragment, &ctx).map_err(error)?;
        println!(
            "{}",
            serde_json::json!({"verified":true,"group":group.to_string(),"index":index,"sha256":local::hex(fragment.sha256),"bytes":fragment.bytes,"elapsedSeconds":start.elapsed().as_secs_f64()})
        );
        return Ok(());
    } else {
        return Err(usage.into());
    };
    let m = manifest.manifest();
    let layout = m.coding.layout(m.object.bytes).map_err(error)?;
    println!(
        "{}",
        serde_json::json!({"operation":args[0].to_string_lossy(),"codec":m.codec.as_str(),"sourceBytes":m.object.bytes.to_string(),"sourceSha256":local::hex(m.object.sha256),"groups":layout.stripes.to_string(),"fragments":layout.fragments.to_string(),"fragmentBytes":m.coding.shard_bytes(),"encodedBytes":layout.encoded_bytes.to_string(),"elapsedSeconds":start.elapsed().as_secs_f64(),"networkStarted":false})
    );
    Ok(())
}
