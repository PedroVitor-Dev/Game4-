//! Magnet parsing is offline; metadata resolution is explicitly networked and cancellable.
use super::*;
use futures_util::{StreamExt, stream};
use librqbit::AddTorrentResponse;
use std::collections::HashSet;
use tokio_util::sync::CancellationToken;
pub(super) struct Resolution {
    id: String,
    cancel: CancellationToken,
}
pub(super) fn initial_peer(value: &str, dht: bool) -> Result<Option<SocketAddr>> {
    if value.trim().is_empty() && dht {
        return Ok(None);
    }
    peer(value).map(Some)
}
/// Public v1 magnets only. Strip advisory parameters and never follow embedded URLs/peers.
pub fn normalize_magnet(value: &str) -> Result<MagnetLink> {
    let invalid = || err(ErrorCode::TorrentInvalid);
    if value.len() > 16384 || value.chars().any(char::is_control) {
        return Err(invalid());
    }
    let url = url::Url::parse(value.trim()).map_err(|_| invalid())?;
    if url.scheme() != "magnet"
        || url.host_str().is_some()
        || !url.path().is_empty()
        || url.fragment().is_some()
    {
        return Err(invalid());
    }
    let mut hash = None;
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "xt" => {
                if hash.is_some() {
                    return Err(invalid());
                }
                let raw = value.strip_prefix("urn:btih:").ok_or_else(invalid)?;
                let bytes = if raw.len() == 40 && raw.bytes().all(|c| c.is_ascii_hexdigit()) {
                    raw.to_ascii_lowercase()
                } else if raw.len() == 32 {
                    let mut out = Vec::new();
                    let mut bits = 0u32;
                    let mut count = 0;
                    for c in raw.bytes() {
                        let c = c.to_ascii_uppercase();
                        let n = match c {
                            b'A'..=b'Z' => c - b'A',
                            b'2'..=b'7' => c - b'2' + 26,
                            _ => return Err(invalid()),
                        };
                        bits = (bits << 5) | u32::from(n);
                        count += 5;
                        if count >= 8 {
                            count -= 8;
                            out.push((bits >> count) as u8);
                        }
                    }
                    out.iter().map(|b| format!("{b:02x}")).collect()
                } else {
                    return Err(invalid());
                };
                hash = Some(bytes);
            }
            "dn" | "tr" | "xl" => {} // Untrusted advisory values are never used as endpoints or paths.
            _ => return Err(invalid()),
        }
    }
    let info_hash = hash.ok_or_else(invalid)?;
    Ok(MagnetLink {
        magnet: format!("magnet:?xt=urn:btih:{info_hash}"),
        info_hash,
    })
}
impl Transfers {
    pub fn cancel_magnet(&self, request: CancelMagnetRequest) -> Result<CancelMagnetResult> {
        valid_uuid(&request.request_id)?;
        let resolution = self.resolution.lock().map_err(|_| failed())?;
        let cancelled = if let Some(r) = resolution.as_ref().filter(|r| r.id == request.request_id)
        {
            r.cancel.cancel();
            true
        } else {
            false
        };
        Ok(CancelMagnetResult { cancelled })
    }
    pub async fn resolve_magnet(
        self: &Arc<Self>,
        request: ResolveMagnetRequest,
    ) -> Result<TorrentImportResult> {
        valid_uuid(&request.request_id)?;
        let link = normalize_magnet(&request.magnet)?;
        let trackers = trackers::normalize(&request.trackers)?;
        let addr = initial_peer(&request.peer, request.dht || !trackers.is_empty())?;
        self.available()?;
        let cancel = self.cancel.child_token();
        {
            let mut active = self.resolution.lock().map_err(|_| failed())?;
            if active.is_some() {
                return Err(err(ErrorCode::TransferUnavailable));
            }
            *active = Some(Resolution {
                id: request.request_id,
                cancel: cancel.clone(),
            });
        }
        let result = self
            .resolve_metadata(link, addr, request.dht, trackers, cancel)
            .await;
        *self.resolution.lock().map_err(|_| failed())? = None;
        result
    }
    async fn resolve_metadata(
        self: &Arc<Self>,
        link: MagnetLink,
        addr: Option<SocketAddr>,
        dht: bool,
        trackers: Vec<String>,
        cancel: CancellationToken,
    ) -> Result<TorrentImportResult> {
        let session = Session::new_with_opts(
            PathBuf::new(),
            SessionOptions {
                cancellation_token: Some(cancel.child_token()),
                dht: self.dht_config(dht),
                disable_trackers: true,
                disable_local_service_discovery: true,
                persistence: None,
                fastresume: false,
                ipv4_only: true,
                listen: None,
                peer_limit: Some(8),
                ..Default::default()
            },
        )
        .await
        .map_err(|_| failed())?;
        // Keep discovery separate from metadata connections. The upstream magnet
        // resolver otherwise fans out to 128 peers with up to 32 MiB per peer.
        // Here only one metadata peer can allocate a buffer at a time.
        let metadata = match Session::new_with_opts(
            PathBuf::new(),
            SessionOptions {
                cancellation_token: Some(cancel.child_token()),
                dht: None,
                disable_trackers: true,
                disable_local_service_discovery: true,
                persistence: None,
                fastresume: false,
                ipv4_only: true,
                listen: (!trackers.is_empty()).then(|| ListenerOptions {
                    listen_addr: SocketAddr::V4(SocketAddrV4::new(self.listen_ip, 0)),
                    ipv4_only: true,
                    enable_upnp_port_forwarding: false,
                    ..Default::default()
                }),
                peer_limit: Some(1),
                ..Default::default()
            },
        )
        .await
        {
            Ok(s) => s,
            Err(_) => {
                session.stop().await;
                return Err(failed());
            }
        };
        let discover = session
            .get_dht()
            .map(|d| {
                d.get_peers(link.info_hash.parse().expect("validated hash"), None)
                    .boxed()
            })
            .unwrap_or_else(|| stream::empty().boxed());
        let tracker_lifetime = cancel.child_token();
        let _tracker_guard = tracker_lifetime.clone().drop_guard();
        let tracker_setup: Result<_> = async {
            Ok(if trackers.is_empty() {
                stream::empty().boxed()
            } else {
                let client = reqwest::Client::builder()
                    .redirect(reqwest::redirect::Policy::none())
                    .timeout(Duration::from_secs(10))
                    .build()
                    .map_err(|_| failed())?;
                let udp =
                    librqbit_tracker_comms::UdpTrackerClient::new(tracker_lifetime.clone(), None)
                        .await
                        .map_err(|_| failed())?;
                librqbit_tracker_comms::TrackerComms::start(
                    link.info_hash.parse().expect("validated hash"),
                    uuid_peer_id(),
                    trackers
                        .iter()
                        .map(|t| url::Url::parse(t).expect("validated tracker"))
                        .collect(),
                    Box::new(MetadataStats),
                    None,
                    metadata.listen_addr().expect("metadata listener").port(),
                    client,
                    udp,
                )
                .unwrap_or_else(|| stream::empty().boxed())
            })
        }
        .await;
        let tracker_rx = match tracker_setup {
            Ok(rx) => rx,
            Err(e) => {
                metadata.stop().await;
                session.stop().await;
                return Err(e);
            }
        };
        let mut peers = stream::select(stream::select(stream::iter(addr), discover), tracker_rx);
        let lookup = async {
            let mut seen = HashSet::new();
            while let Some(candidate) = peers.next().await {
                if peer(&candidate.to_string()).is_err() || !seen.insert(candidate) {
                    continue;
                }
                if seen.len() > 128 {
                    return Err(limit());
                }
                // list_only returns before any storage factory or payload file is opened.
                if let Ok(Ok(AddTorrentResponse::ListOnly(r))) = tokio::time::timeout(
                    Duration::from_secs(8),
                    metadata.add_torrent(
                        AddTorrent::from_url(link.magnet.clone()),
                        Some(AddTorrentOptions {
                            paused: true,
                            list_only: true,
                            disable_trackers: true,
                            initial_peers: Some(vec![candidate]),
                            peer_limit: Some(1),
                            ..Default::default()
                        }),
                    ),
                )
                .await
                {
                    return Ok(Some(r.torrent_bytes.to_vec()));
                }
            }
            Err(failed())
        };
        let response = tokio::select! {
            biased;
            _ = cancel.cancelled() => Ok(None),
            result = tokio::time::timeout(Duration::from_secs(60), lookup) => result.unwrap_or_else(|_| Err(failed())),
        };
        tracker_lifetime.cancel();
        metadata.stop().await;
        session.stop().await;
        let Some(raw) = response? else {
            return Ok(TorrentImportResult {
                cancelled: true,
                preview: None,
            });
        };
        if cancel.is_cancelled() {
            return Ok(TorrentImportResult {
                cancelled: true,
                preview: None,
            });
        }
        let manager = Arc::clone(self);
        tokio::task::spawn_blocking(move || {
            let meta = inspect(&raw)?;
            if meta.hash != link.info_hash {
                return Err(err(ErrorCode::TorrentInvalid));
            }
            Ok(TorrentImportResult {
                cancelled: false,
                preview: Some(manager.import(ever::route(&raw, &trackers)?)?),
            })
        })
        .await
        .map_err(|_| failed())?
    }
}

struct MetadataStats;
impl librqbit_tracker_comms::TorrentStatsProvider for MetadataStats {
    fn get(&self) -> librqbit_tracker_comms::TrackerCommsStats {
        librqbit_tracker_comms::TrackerCommsStats {
            total_bytes: 1,
            torrent_state: librqbit_tracker_comms::TrackerCommsStatsState::Live,
            ..Default::default()
        }
    }
}
fn uuid_peer_id() -> librqbit::dht::Id20 {
    let mut id = [0u8; 20];
    id[..4].copy_from_slice(b"G4MD");
    id[4..].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
    librqbit::dht::Id20::new(id)
}
