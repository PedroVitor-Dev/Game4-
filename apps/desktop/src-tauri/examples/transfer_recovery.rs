//! Local synthetic acceptance harness; `crash` intentionally exits without cleanup.
use game4_desktop_lib::{contracts::*, torrents::Transfers};
use librqbit::{AddTorrent, AddTorrentOptions, ListenerOptions, Session, SessionOptions};
use std::{io::Write, path::PathBuf, sync::Arc, time::Duration};
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let mode = args.get(1).map(String::as_str).unwrap_or("");
    let fixture = PathBuf::from(
        args.get(2)
            .ok_or_else(|| anyhow::anyhow!("expected mode and probe fixture directory"))?,
    );
    let reference: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixture.join("fixture.json"))?)?;
    anyhow::ensure!(
        reference["size_bytes"] == 8388625,
        "only the probe synthetic fixture is supported"
    );
    let raw = std::fs::read(fixture.join("fixture.torrent"))?;
    let parsed = librqbit::torrent_from_bytes(&raw)?;
    anyhow::ensure!(
        parsed.info.data.files.is_none()
            && parsed.info.data.length == Some(8388625)
            && parsed
                .info
                .data
                .name
                .as_ref()
                .is_some_and(|n| n.as_ref() == b"payload.bin"),
        "unexpected fixture layout"
    );
    let payload = fixture.join("seed/payload.bin");
    for ancestor in payload.ancestors() {
        let meta = std::fs::symlink_metadata(ancestor)?;
        anyhow::ensure!(
            !meta.file_type().is_symlink(),
            "fixture links are not supported"
        );
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            anyhow::ensure!(
                meta.file_attributes() & 0x400 == 0,
                "fixture reparse points are not supported"
            );
        }
    }
    anyhow::ensure!(
        std::fs::metadata(&payload)?.len() == 8388625,
        "unexpected fixture length"
    );
    use sha2::Digest;
    anyhow::ensure!(
        format!("{:x}", sha2::Sha256::digest(std::fs::read(&payload)?))
            == "ca1d758393b634d51aea5298b89b6d598403e41d5e72176e8114fc434b4dfe29",
        "unexpected fixture SHA-256"
    );
    if mode == "seed" {
        let session = Session::new_with_opts(
            fixture.join("seed"),
            SessionOptions {
                dht: None,
                disable_trackers: true,
                disable_local_service_discovery: true,
                persistence: None,
                ipv4_only: true,
                listen: Some(ListenerOptions {
                    listen_addr: "127.0.0.1:0".parse()?,
                    ipv4_only: true,
                    enable_upnp_port_forwarding: false,
                    ..Default::default()
                }),
                ratelimits: librqbit::limits::LimitsConfig {
                    upload_bps: std::num::NonZeroU32::new(524288),
                    download_bps: None,
                },
                ..Default::default()
            },
        )
        .await?;
        let h = session
            .add_torrent(
                AddTorrent::from_bytes(raw),
                Some(AddTorrentOptions {
                    paused: true,
                    overwrite: true,
                    disable_trackers: true,
                    ..Default::default()
                }),
            )
            .await?
            .into_handle()
            .ok_or_else(|| anyhow::anyhow!("missing handle"))?;
        h.wait_until_initialized().await?;
        anyhow::ensure!(h.stats().finished, "fixture incomplete");
        session.unpause(&h).await?;
        println!(
            "{}",
            serde_json::json!({"peer":session.listen_addr().unwrap().to_string()})
        );
        std::io::stdout().flush()?;
        tokio::time::sleep(Duration::from_secs(90)).await;
        session.stop().await;
        return Ok(());
    }
    let parent = fixture.join("desktop-received");
    std::fs::create_dir_all(&parent)?;
    let manager = Arc::new(
        Transfers::open(&fixture.join("desktop-state/state.sqlite3"))
            .map_err(|e| anyhow::anyhow!("{:?}", e.code))?,
    );
    let first = if mode == "crash" {
        let p = manager
            .import(raw)
            .map_err(|e| anyhow::anyhow!("{:?}", e.code))?;
        manager
            .download(
                StartDownloadRequest {
                    request_id: uuid::Uuid::new_v4().to_string(),
                    token: p.token,
                    peer: args
                        .get(3)
                        .ok_or_else(|| anyhow::anyhow!("expected peer"))?
                        .clone(),
                    dht: false,
                    trackers: vec![],
                    seed_after_download: false,
                },
                parent,
            )
            .await
            .map_err(|e| anyhow::anyhow!("{:?}", e.code))?
    } else if mode == "resume" {
        let before = manager
            .list()
            .items
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("missing transfer"))?;
        anyhow::ensure!(
            before.status == TransferStatus::Paused
                && before.listen_port.is_none()
                && before.telemetry.is_none(),
            "restart must stay offline"
        );
        println!(
            "{}",
            serde_json::json!({"recovered_status":before.status,"saved_id":before.id,"saved_verified_bytes":before.verified_bytes})
        );
        manager
            .resume(&before.id)
            .await
            .map_err(|e| anyhow::anyhow!("{:?}", e.code))?
    } else {
        anyhow::bail!("expected seed, crash or resume")
    };
    println!(
        "{}",
        serde_json::json!({"started_id":first.id,"verified_on_start":first.verified_bytes})
    );
    tokio::time::timeout(Duration::from_secs(60),async{loop{
  let s=manager.list().items.into_iter().find(|v|v.id==first.id).unwrap();
  if mode=="crash" && s.verified_bytes.parse::<u64>()?>=1048576{println!("{}",serde_json::json!({"abrupt_exit":true,"id":s.id,"verified_bytes":s.verified_bytes}));std::io::stdout().flush()?;std::process::exit(23);}
  anyhow::ensure!(s.status!=TransferStatus::Failed,"transfer failed");
  if s.status==TransferStatus::Completed{println!("{}",serde_json::to_string(&s)?);return Ok::<_,anyhow::Error>(());}
  tokio::time::sleep(Duration::from_millis(20)).await;
 }}).await??;
    manager.shutdown().await;
    Ok(())
}
