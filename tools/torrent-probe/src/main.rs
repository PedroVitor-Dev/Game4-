use anyhow::{Context, Result, bail, ensure};
use clap::{Parser, Subcommand};
use librqbit::{
    AddTorrent, AddTorrentOptions, CreateTorrentOptions, ListenerOptions, Session, SessionOptions,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    net::SocketAddr,
    num::NonZeroU32,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::time::{sleep, timeout};

const PIECE_LENGTH: u32 = 256 * 1024;
const PAYLOAD_SIZE: usize = 8 * 1024 * 1024 + 17;

#[derive(Parser)]
#[command(
    about = "Game4Ã¢Ë†Å¾ synthetic BitTorrent evaluation. No desktop integration or public discovery."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a new fixture directory. Never overwrites an existing directory.
    Prepare {
        #[arg(long)]
        directory: PathBuf,
    },
    /// Seed only the generated synthetic fixture. LAN binding requires an explicit address.
    Seed {
        #[arg(long)]
        directory: PathBuf,
        #[arg(long, default_value = "127.0.0.1:42420")]
        listen: SocketAddr,
        #[arg(long, default_value_t = 600)]
        seconds: u64,
    },
    /// Download the fixture using one explicitly supplied peer.
    Download {
        #[arg(long)]
        directory: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        peer: SocketAddr,
        #[arg(long)]
        resume: bool,
        #[arg(long)]
        magnet: bool,
        #[arg(long)]
        stop_after_bytes: Option<u64>,
        #[arg(long, default_value_t = 1048576)]
        download_bps: u32,
        #[arg(long, default_value_t = 180)]
        timeout_seconds: u64,
    },
    /// Evaluate two isolated sessions on loopback, including restart and corruption recovery.
    Loopback {
        #[arg(long)]
        directory: PathBuf,
    },
}

#[derive(Serialize, Deserialize)]
struct Fixture {
    engine: String,
    name: String,
    size_bytes: u64,
    sha256: String,
    magnet: String,
}

#[derive(Debug, Serialize)]
struct DownloadReport {
    completed: bool,
    resumed_bytes: u64,
    final_bytes: u64,
    total_bytes: u64,
    sha256_matches: Option<bool>,
    elapsed_seconds: f64,
    mode: &'static str,
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?
        .write_all(bytes)?;
    Ok(())
}

fn hash(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn load_torrent(directory: &Path) -> Result<Vec<u8>> {
    let bytes = fs::read(directory.join("fixture.torrent"))?;
    let meta = librqbit::torrent_from_bytes(&bytes)?;
    let info = &meta.info.data;
    ensure!(
        info.files.is_none() && info.length == Some(PAYLOAD_SIZE as u64),
        "Only the single-file synthetic fixture is supported"
    );
    ensure!(
        info.name
            .as_ref()
            .is_some_and(|name| name.as_ref() == b"payload.bin"),
        "Unexpected torrent filename"
    );
    ensure!(
        info.piece_length == PIECE_LENGTH
            && info.pieces.0.len() == (PAYLOAD_SIZE.div_ceil(PIECE_LENGTH as usize) * 20),
        "Unexpected piece geometry"
    );
    Ok(bytes)
}

fn read_fixture(directory: &Path) -> Result<Fixture> {
    let fixture: Fixture = serde_json::from_slice(&fs::read(directory.join("fixture.json"))?)?;
    ensure!(
        fixture.name == "payload.bin" && fixture.size_bytes == PAYLOAD_SIZE as u64,
        "Only this tool's synthetic fixture is supported"
    );
    ensure!(
        fixture.sha256.len() == 64 && fixture.sha256.bytes().all(|b| b.is_ascii_hexdigit()),
        "Invalid expected SHA-256"
    );
    let bytes = load_torrent(directory)?;
    let meta = librqbit::torrent_from_bytes(&bytes)?;
    ensure!(
        fixture.magnet == librqbit::Magnet::from_id20(meta.info_hash, vec![], None).to_string(),
        "Magnet must match fixture metadata without external trackers"
    );
    Ok(fixture)
}

async fn prepare(directory: &Path) -> Result<Fixture> {
    fs::create_dir(directory).context("Choose a new directory under an existing parent")?;
    fs::create_dir(directory.join("seed"))?;
    let path = directory.join("seed/payload.bin");
    let mut data = vec![0; PAYLOAD_SIZE];
    let mut state = 0x4a61_6d65_34ff_0031u64;
    for byte in &mut data {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        *byte = state as u8;
    }
    write_new(&path, &data)?;
    let torrent = librqbit::create_torrent(
        &path,
        CreateTorrentOptions {
            piece_length: Some(PIECE_LENGTH),
            ..Default::default()
        },
        &librqbit::spawn_utils::BlockingSpawner::new(1),
    )
    .await?;
    let fixture = Fixture {
        engine: librqbit::version().to_owned(),
        name: "payload.bin".into(),
        size_bytes: PAYLOAD_SIZE as u64,
        sha256: hash(&path)?,
        magnet: torrent.as_magnet().to_string(),
    };
    write_new(&directory.join("fixture.torrent"), &torrent.as_bytes()?)?;
    write_new(
        &directory.join("fixture.json"),
        &serde_json::to_vec_pretty(&fixture)?,
    )?;
    Ok(fixture)
}

async fn session(
    output: PathBuf,
    listen: Option<SocketAddr>,
    download_bps: Option<NonZeroU32>,
) -> Result<Arc<Session>> {
    Session::new_with_opts(
        output,
        SessionOptions {
            dht: None,
            disable_trackers: true,
            disable_local_service_discovery: true,
            persistence: None,
            fastresume: false,
            ipv4_only: true,
            listen: listen.map(|listen_addr| ListenerOptions {
                listen_addr,
                ipv4_only: true,
                enable_upnp_port_forwarding: false,
                ..Default::default()
            }),
            ratelimits: librqbit::limits::LimitsConfig {
                download_bps,
                ..Default::default()
            },
            peer_limit: Some(4),
            ..Default::default()
        },
    )
    .await
}

async fn start_seed(
    directory: &Path,
    listen: SocketAddr,
) -> Result<(Arc<Session>, Arc<librqbit::ManagedTorrent>)> {
    let fixture = read_fixture(directory)?;
    ensure!(
        hash(&directory.join("seed/payload.bin"))? == fixture.sha256,
        "Seed SHA-256 mismatch"
    );
    let session = session(directory.join("seed"), Some(listen), None).await?;
    let result = async {
        let handle = session
            .add_torrent(
                AddTorrent::from_bytes(load_torrent(directory)?),
                Some(AddTorrentOptions {
                    overwrite: true,
                    disable_trackers: true,
                    ..Default::default()
                }),
            )
            .await?
            .into_handle()
            .context("Missing seed handle")?;
        timeout(Duration::from_secs(30), handle.wait_until_initialized()).await??;
        ensure!(handle.stats().finished, "Seed is not complete");
        Ok(handle)
    }
    .await;
    match result {
        Ok(handle) => Ok((session, handle)),
        Err(error) => {
            session.stop().await;
            Err(error)
        }
    }
}

struct DownloadOptions {
    resume: bool,
    magnet: bool,
    stop_after_bytes: Option<u64>,
    download_bps: u32,
    timeout_seconds: u64,
}

async fn download(
    directory: &Path,
    output: &Path,
    peer: SocketAddr,
    options: DownloadOptions,
) -> Result<DownloadReport> {
    let fixture = read_fixture(directory)?;
    ensure!(
        peer.is_ipv4() && peer.port() != 0 && !peer.ip().is_unspecified(),
        "Provide an explicit IPv4 peer and nonzero port"
    );
    ensure!(
        options.download_bps >= 16384,
        "Download limit must be at least 16384 bytes/s"
    );
    if output.exists() {
        ensure!(
            options.resume || fs::read_dir(output)?.next().is_none(),
            "Output must be empty; use --resume only for this fixture's partial download"
        );
    } else {
        fs::create_dir_all(output)?;
    }
    let marker = output.join(".game4-probe-sha256");
    if options.resume {
        ensure!(
            fs::read_to_string(&marker).context("Resume requires this probe's output directory")?
                == fixture.sha256,
            "Resume fixture mismatch"
        );
    } else {
        write_new(&marker, fixture.sha256.as_bytes())?;
    }
    if let Some(bytes) = options.stop_after_bytes {
        ensure!(
            bytes > 0 && bytes < fixture.size_bytes,
            "Partial threshold must be inside the file"
        );
    }
    let started = Instant::now();
    let session = session(
        output.to_owned(),
        None,
        NonZeroU32::new(options.download_bps),
    )
    .await?;
    let result = timeout(Duration::from_secs(options.timeout_seconds), async {
        let add = if options.magnet {
            AddTorrent::from_url(fixture.magnet.clone())
        } else {
            AddTorrent::from_bytes(load_torrent(directory)?)
        };
        let handle = session
            .add_torrent(
                add,
                Some(AddTorrentOptions {
                    overwrite: options.resume,
                    disable_trackers: true,
                    initial_peers: Some(vec![peer]),
                    ..Default::default()
                }),
            )
            .await?
            .into_handle()
            .context("Missing download handle")?;
        handle.wait_until_initialized().await?;
        let resumed_bytes = handle.stats().progress_bytes;
        loop {
            let stats = handle.stats();
            if let Some(error) = stats.error {
                bail!("Torrent failed: {error}");
            }
            if stats.finished
                || options
                    .stop_after_bytes
                    .is_some_and(|threshold| stats.progress_bytes >= threshold)
            {
                break;
            }
            sleep(Duration::from_millis(100)).await;
        }
        session.pause(&handle).await?;
        let stats = handle.stats();
        Ok::<_, anyhow::Error>(DownloadReport {
            completed: stats.finished,
            resumed_bytes,
            final_bytes: stats.progress_bytes,
            total_bytes: stats.total_bytes,
            sha256_matches: None,
            elapsed_seconds: started.elapsed().as_secs_f64(),
            mode: if options.magnet { "magnet" } else { "torrent" },
        })
    })
    .await;
    session.stop().await;
    let mut report = result.context("Transfer timed out; partial data was retained")??;
    if report.completed {
        report.sha256_matches = Some(hash(&output.join(&fixture.name))? == fixture.sha256);
        ensure!(
            report.sha256_matches == Some(true),
            "Final SHA-256 mismatch"
        );
    }
    Ok(report)
}

async fn check_creator_boundary(directory: &Path) -> Result<serde_json::Value> {
    let path = directory.join("boundary.bin");
    write_new(&path, &vec![7; PIECE_LENGTH as usize])?;
    let torrent = librqbit::create_torrent(
        &path,
        CreateTorrentOptions {
            piece_length: Some(PIECE_LENGTH),
            ..Default::default()
        },
        &librqbit::spawn_utils::BlockingSpawner::new(1),
    )
    .await?;
    let observed = torrent.meta.info.data.pieces.0.len() / 20;
    Ok(
        serde_json::json!({ "file_bytes": PIECE_LENGTH, "expected_piece_hashes": 1, "observed_piece_hashes": observed, "passed": observed == 1 }),
    )
}

async fn loopback(directory: &Path) -> Result<()> {
    let fixture = prepare(directory).await?;
    let (seed, seed_handle) = start_seed(directory, "127.0.0.1:0".parse()?).await?;
    let peer = seed.listen_addr().context("Missing seed listener")?;
    let result = async {
        let output = directory.join("download");
        let partial = download(directory, &output, peer, DownloadOptions { resume: false, magnet: false, stop_after_bytes: Some(512 * 1024), download_bps: 512 * 1024, timeout_seconds: 60 }).await?;
        ensure!(!partial.completed && partial.final_bytes > 0, "Partial download did not stop before completion");
        let resumed = download(directory, &output, peer, DownloadOptions { resume: true, magnet: false, stop_after_bytes: None, download_bps: 1024 * 1024, timeout_seconds: 90 }).await?;
        ensure!(resumed.completed && resumed.resumed_bytes > 0 && resumed.resumed_bytes < fixture.size_bytes, "Restart did not reuse verified partial pieces");
        let mut first = [0u8];
        File::open(output.join("payload.bin"))?.read_exact(&mut first)?;
        first[0] ^= 0xff;
        OpenOptions::new().write(true).open(output.join("payload.bin"))?.write_all(&first)?;
        ensure!(hash(&output.join("payload.bin"))? != fixture.sha256, "Corruption fixture did not change the file");
        let repaired = download(directory, &output, peer, DownloadOptions { resume: true, magnet: false, stop_after_bytes: None, download_bps: 1024 * 1024, timeout_seconds: 60 }).await?;
        ensure!(repaired.completed && repaired.resumed_bytes < fixture.size_bytes, "Corruption was not detected during recheck");
        let magnet = download(directory, &directory.join("magnet-download"), peer, DownloadOptions { resume: false, magnet: true, stop_after_bytes: None, download_bps: 2 * 1024 * 1024, timeout_seconds: 90 }).await?;
        ensure!(magnet.completed, "Magnet transfer incomplete");
        let creator_boundary = check_creator_boundary(directory).await?;
        let report = serde_json::json!({
            "engine": format!("librqbit {}", librqbit::version()), "topology": "two sessions on one Windows host; loopback TCP",
            "fixture_sha256": fixture.sha256, "transport_checks_passed": true,
            "partial": partial, "restart_resume": resumed, "corruption_repair": repaired, "magnet": magnet,
            "seed_uploaded_bytes": seed_handle.stats().uploaded_bytes, "creator_exact_piece_boundary": creator_boundary,
            "two_computer_test": "pending; second computer unavailable", "candidate_approved": false,
            "unvalidated": ["physical LAN", "NAT/WAN", "DHT", "trackers", "multi-file", "cross-client interoperability", "performance benchmarks"]
        });
        write_new(&directory.join("report.json"), &serde_json::to_vec_pretty(&report)?)?;
        println!("{}", serde_json::to_string_pretty(&report)?);
        Ok(())
    }.await;
    seed.stop().await;
    result
}

#[tokio::main]
async fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Prepare { directory } => println!(
            "{}",
            serde_json::to_string_pretty(&prepare(&directory).await?)?
        ),
        Command::Seed {
            directory,
            listen,
            seconds,
        } => {
            ensure!(
                listen.is_ipv4(),
                "Only IPv4 is supported in this experiment"
            );
            let (session, _handle) = start_seed(&directory, listen).await?;
            println!(
                "Seed ready at {}. DHT, trackers, LSD, and UPnP disabled. Ctrl+C stops sharing.",
                session.listen_addr().unwrap()
            );
            tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = sleep(Duration::from_secs(seconds)) => {} }
            session.stop().await;
        }
        Command::Download {
            directory,
            output,
            peer,
            resume,
            magnet,
            stop_after_bytes,
            download_bps,
            timeout_seconds,
        } => {
            let report = download(
                &directory,
                &output,
                peer,
                DownloadOptions {
                    resume,
                    magnet,
                    stop_after_bytes,
                    download_bps,
                    timeout_seconds,
                },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::Loopback { directory } => loopback(&directory).await?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn fixture_is_valid_and_preparation_never_overwrites() {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("fixture");
        let fixture = prepare(&directory).await.unwrap();
        assert_eq!(read_fixture(&directory).unwrap().sha256, fixture.sha256);
        assert!(prepare(&directory).await.is_err());
        assert_eq!(
            hash(&directory.join("seed/payload.bin")).unwrap(),
            fixture.sha256
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn rejects_untrusted_paths_trackers_and_resume_directories_before_networking() {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("fixture");
        let mut fixture = prepare(&directory).await.unwrap();
        let output = temp.path().join("unmarked");
        fs::create_dir(&output).unwrap();
        let error = download(
            &directory,
            &output,
            "127.0.0.1:1".parse().unwrap(),
            DownloadOptions {
                resume: true,
                magnet: false,
                stop_after_bytes: None,
                download_bps: 1048576,
                timeout_seconds: 1,
            },
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("Resume requires"));
        fixture
            .magnet
            .push_str("&tr=https://example.invalid/announce");
        fs::write(
            directory.join("fixture.json"),
            serde_json::to_vec(&fixture).unwrap(),
        )
        .unwrap();
        assert!(read_fixture(&directory).is_err());
        let path = directory.join("fixture.torrent");
        let mut bytes = fs::read(&path).unwrap();
        let index = bytes.windows(11).position(|w| w == b"payload.bin").unwrap();
        bytes[index..index + 11].copy_from_slice(b"../evil.bin");
        fs::write(&path, bytes).unwrap();
        assert!(load_torrent(&directory).is_err());
    }
}
