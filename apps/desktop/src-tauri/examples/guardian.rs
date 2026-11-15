//! Local Guardian service utility: no network listener, discovery or automatic admission.
use game4_desktop_lib::g4ep::{
    guardian::{GuardianConfig, GuardianNode},
    manifest,
    types::*,
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1)
    }
}
fn run() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if args.len() < 2 {
        return Err("Usage: guardian DIRECTORY status|verify|approve MANIFEST GROUP INDEX [--share]|import ID FILE --enable|export ID NEW_FILE --enable|revoke ID [--delete]|limits QUOTA_BYTES UPLOAD_BPS DOWNLOAD_BPS".into());
    }
    let n = GuardianNode::open(&PathBuf::from(&args[0]))?;
    let c = OperationContext {
        cancellation: tokio_util::sync::CancellationToken::new(),
        deadline: Instant::now() + Duration::from_secs(3600),
    };
    let arg = |i: usize| -> std::result::Result<&str, Box<dyn std::error::Error>> {
        args.get(i)
            .and_then(|a| a.to_str())
            .ok_or_else(|| "Missing/invalid argument".into())
    };
    match arg(1)? {
        "status" if args.len() == 2 => {}
        "prune" if args.len() == 2 => n.prune()?,
        "verify" if args.len() == 2 => n.verify_all(&c)?,
        "approve" if args.len() == 5 || (args.len() == 6 && arg(5)? == "--share") => {
            let doc =
                manifest::read(&PathBuf::from(&args[2])).map_err(|_| "Invalid .g4e manifest")?;
            n.approve(&doc, arg(3)?.parse()?, arg(4)?.parse()?, args.len() == 6)?;
        }
        "limits" if args.len() == 5 => n.configure(GuardianConfig {
            active: false,
            quota_bytes: arg(2)?.into(),
            upload_bps: arg(3)?.parse()?,
            download_bps: arg(4)?.parse()?,
        })?,
        "import" | "export" if args.len() == 5 && arg(4)? == "--enable" => {
            let mut config = n.snapshot(0)?.config;
            config.active = true;
            n.configure(config)?;
            let f = n.entry(arg(2)?.parse()?)?;
            if arg(1)? == "import" {
                n.import_file(&f, &PathBuf::from(&args[3]), &c)?;
            } else {
                n.export_file(&f, &PathBuf::from(&args[3]), &c)?;
            }
        }
        "revoke" if args.len() == 3 || (args.len() == 4 && arg(3)? == "--delete") => {
            n.revoke(arg(2)?.parse()?, args.len() == 4)?
        }
        _ => return Err("Invalid Guardian command or arguments".into()),
    }
    println!("{}", serde_json::to_string(&n.snapshot(0)?)?);
    Ok(())
}
