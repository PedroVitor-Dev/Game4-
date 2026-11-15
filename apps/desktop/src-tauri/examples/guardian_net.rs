//! Direct private TLS fragment exchange. No central service or public discovery.
use game4_desktop_lib::g4ep::{
    guardian::GuardianNode,
    network::{self, Identity, ManualTransport},
    ports::StorageNode,
    types::*,
};
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("{e}");
        std::process::exit(1)
    }
}
async fn run() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if args.len() < 3 {
        return Err("Usage: guardian_net STORE PRIVATE_KEYS profile ADDRESS NEW_PROFILE.json | enroll PROFILE.json | configure BIND MAX_CONNECTIONS TIMEOUT_SECONDS RETRIES | serve --enable | probe PEER_ID --enable | available|fetch|push PEER_ID FRAGMENT_ENTRY_ID --enable".into());
    }
    let arg = |i: usize| -> std::result::Result<&str, Box<dyn std::error::Error>> {
        args.get(i)
            .and_then(|v| v.to_str())
            .ok_or_else(|| "Missing argument".into())
    };
    let node = Arc::new(GuardianNode::open(&PathBuf::from(&args[0]))?);
    let identity = Identity::load(node.node_id(), &PathBuf::from(&args[1]))?;
    let mut config = network::read_config(&node)?;
    match arg(2)? {
        "profile" if args.len() == 5 => {
            network::write_profile(&PathBuf::from(&args[4]), &identity.profile(arg(3)?)?)?;
            println!(
                "{}",
                serde_json::json!({"profileCreated":true,"nodeId":network_id(node.node_id()),"certificateSha256":identity.fingerprint(),"networkStarted":false})
            );
        }
        "enroll" if args.len() == 4 => {
            let profile = network::read_profile(&PathBuf::from(&args[3]))?;
            config.peers.retain(|p| p.node_id != profile.node_id);
            config.peers.push(profile);
            network::save_config(&node, &config)?;
            println!(
                "{}",
                serde_json::json!({"enrolledPeers":config.peers.len(),"networkStarted":false})
            );
        }
        "configure" if args.len() == 7 => {
            config.bind = arg(3)?.into();
            config.max_connections = arg(4)?.parse()?;
            config.timeout_seconds = arg(5)?.parse()?;
            config.retries = arg(6)?.parse()?;
            network::save_config(&node, &config)?;
            println!(
                "{}",
                serde_json::json!({"configured":true,"networkStarted":false})
            );
        }
        operation @ ("serve" | "probe" | "available" | "fetch" | "push") => {
            let valid = match operation {
                "serve" => args.len() == 4,
                "probe" => args.len() == 5,
                _ => args.len() == 6,
            };
            if !valid || arg(args.len() - 1)? != "--enable" {
                return Err("Explicit --enable is required".into());
            }
            let mut c = node.snapshot(0)?.config;
            c.active = true;
            node.configure(c)?;
            let service = ManualTransport::new(Arc::clone(&node), identity, config.clone())?;
            if operation == "serve" {
                let server = service.start().await?;
                println!(
                    "{}",
                    serde_json::json!({"listening":config.bind,"nodeId":network_id(node.node_id()),"mutualTls":true,"publicDiscovery":false})
                );
                tokio::signal::ctrl_c().await?;
                server.stopped().await;
            } else {
                let id = network::node_id(arg(3)?)?;
                let peer = *service
                    .peers()
                    .iter()
                    .find(|p| p.node == id)
                    .ok_or("Peer not enrolled")?;
                let ctx = OperationContext {
                    cancellation: tokio_util::sync::CancellationToken::new(),
                    deadline: Instant::now()
                        + Duration::from_secs(
                            u64::from(config.timeout_seconds) * (u64::from(config.retries) + 1) + 2,
                        ),
                };
                let start = Instant::now();
                let (attempts, available, bytes) = if operation == "probe" {
                    (service.probe(&peer, &ctx).await?, None, 0)
                } else {
                    let f = node.entry(arg(4)?.parse()?)?;
                    let (a, v) = match operation {
                        "available" => {
                            let (a, n) = service
                                .availability(&peer, std::slice::from_ref(&f), &ctx)
                                .await?;
                            (n, a.first().copied())
                        }
                        "fetch" => (service.fetch(&peer, &f, &ctx).await?, None),
                        "push" => (service.push(&peer, &f, &ctx).await?, None),
                        _ => return Err("Invalid operation".into()),
                    };
                    (a, v, f.bytes)
                };
                println!(
                    "{}",
                    serde_json::json!({"operation":operation,"peerId":network_id(peer.node),"attempts":attempts,"available":available,"fragmentBytes":bytes,"elapsedSeconds":start.elapsed().as_secs_f64(),"mutualTls":true,"centralServer":false})
                );
            }
        }
        _ => return Err("Invalid Guardian network arguments".into()),
    }
    Ok(())
}
fn network_id(id: NodeId) -> String {
    game4_desktop_lib::g4ep::local::hex(Sha256Digest(id.0))
}
