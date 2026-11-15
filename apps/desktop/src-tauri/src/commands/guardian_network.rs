use super::guardian::GuardianState;
use crate::{
    contracts::*,
    g4ep::{
        network::{self, Identity, ManualTransport},
        ports::StorageNode,
        types::*,
    },
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
fn mapped(e: Error) -> IpcError {
    IpcError::new(match e {
        Error::InvalidNetwork | Error::PeerNotAllowed | Error::Protocol => {
            ErrorCode::InvalidArgument
        }
        Error::PeerAuthentication => ErrorCode::GuardianPeerAuthentication,
        Error::NetworkTimeout | Error::Deadline => ErrorCode::GuardianNetworkTimeout,
        Error::ConnectionLimit => ErrorCode::GuardianConnectionLimit,
        Error::GuardianDisabled => ErrorCode::GuardianDisabled,
        Error::GuardianNotApproved => ErrorCode::GuardianNotApproved,
        Error::QuotaExceeded => ErrorCode::GuardianQuotaExceeded,
        Error::IntegrityMismatch => ErrorCode::G4eIntegrityFailed,
        _ => ErrorCode::GuardianNetworkFailed,
    })
}
#[tauri::command]
pub async fn guardian_network_request(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, Arc<GuardianState>>,
    request: GuardianNetworkRequest,
) -> std::result::Result<NetworkSnapshot, IpcError> {
    let state = Arc::clone(state.inner());
    let clone = Arc::clone(&state);
    let node = tauri::async_runtime::spawn_blocking(move || clone.get())
        .await
        .map_err(|_| IpcError::new(ErrorCode::StorageUnavailable))??;
    let clone = Arc::clone(&node);
    let keys = state.base.join("guardian-identities");
    let (identity, mut config) = tauri::async_runtime::spawn_blocking(move || {
        Ok::<_, Error>((
            Identity::load(clone.node_id(), &keys)?,
            network::read_config(&clone)?,
        ))
    })
    .await
    .map_err(|_| mapped(Error::TransportUnavailable))?
    .map_err(mapped)?;
    // Changing peer enrollment/listener settings requires a stopped session.
    let mut session = state.network.lock().await;
    if session.as_ref().is_some_and(|s| !s.running())
        && let Some(old) = session.take()
    {
        old.stopped().await;
    }
    let mut outcome = None;
    match request {
        GuardianNetworkRequest::Status => {}
        GuardianNetworkRequest::Stop => {
            if let Some(old) = session.take() {
                old.stopped().await;
            }
        }
        GuardianNetworkRequest::Start => {
            if session.is_some() {
                return Err(mapped(Error::ConnectionLimit));
            }
            let transport =
                ManualTransport::new(Arc::clone(&node), Arc::clone(&identity), config.clone())
                    .map_err(mapped)?;
            *session = Some(transport.start().await.map_err(mapped)?);
        }
        GuardianNetworkRequest::Configure {
            bind,
            max_connections,
            timeout_seconds,
            retries,
        } => {
            if session.is_some() {
                return Err(mapped(Error::ConnectionLimit));
            }
            config.bind = bind;
            config.max_connections = max_connections;
            config.timeout_seconds = timeout_seconds;
            config.retries = retries;
            let clone = Arc::clone(&node);
            let c = config.clone();
            tauri::async_runtime::spawn_blocking(move || network::save_config(&clone, &c))
                .await
                .map_err(|_| mapped(Error::TransportUnavailable))?
                .map_err(mapped)?;
        }
        GuardianNetworkRequest::AddPeer { address } => {
            if session.is_some() {
                return Err(mapped(Error::ConnectionLimit));
            }
            let path = tauri::async_runtime::spawn_blocking(move || {
                rfd::FileDialog::new()
                    .set_parent(&window)
                    .add_filter("Guardian public profile", &["json"])
                    .pick_file()
            })
            .await
            .map_err(|_| mapped(Error::TransportUnavailable))?;
            if let Some(path) = path {
                let mut profile =
                    tauri::async_runtime::spawn_blocking(move || network::read_profile(&path))
                        .await
                        .map_err(|_| mapped(Error::TransportUnavailable))?
                        .map_err(mapped)?;
                if !address.is_empty() {
                    profile.address = address;
                }
                config.peers.push(profile);
                let clone = Arc::clone(&node);
                let c = config.clone();
                tauri::async_runtime::spawn_blocking(move || network::save_config(&clone, &c))
                    .await
                    .map_err(|_| mapped(Error::TransportUnavailable))?
                    .map_err(mapped)?;
            }
        }
        GuardianNetworkRequest::RemovePeer { node_id } => {
            if session.is_some() {
                return Err(mapped(Error::ConnectionLimit));
            }
            config.peers.retain(|p| p.node_id != node_id);
            let clone = Arc::clone(&node);
            let c = config.clone();
            tauri::async_runtime::spawn_blocking(move || network::save_config(&clone, &c))
                .await
                .map_err(|_| mapped(Error::TransportUnavailable))?
                .map_err(mapped)?;
        }
        GuardianNetworkRequest::ExportProfile { address } => {
            let profile = identity.profile(&address).map_err(mapped)?;
            tauri::async_runtime::spawn_blocking(move || {
                if let Some(path) = rfd::FileDialog::new()
                    .set_parent(&window)
                    .set_file_name("guardian-peer.json")
                    .save_file()
                {
                    network::write_profile(&path, &profile)?;
                }
                Ok::<_, Error>(())
            })
            .await
            .map_err(|_| mapped(Error::TransportUnavailable))?
            .map_err(mapped)?;
        }
        action => {
            let (peer_id, id, operation) = match action {
                GuardianNetworkRequest::Probe { node_id } => (node_id, None, "probe"),
                GuardianNetworkRequest::Availability { node_id, id } => {
                    (node_id, Some(id), "availability")
                }
                GuardianNetworkRequest::Fetch { node_id, id } => (node_id, Some(id), "download"),
                GuardianNetworkRequest::Push { node_id, id } => (node_id, Some(id), "upload"),
                _ => return Err(mapped(Error::Protocol)),
            };
            let service = Arc::clone(
                &session
                    .as_ref()
                    .ok_or_else(|| mapped(Error::GuardianDisabled))?
                    .transport,
            );
            let peer = *service
                .peers()
                .iter()
                .find(|p| network::node_id(&peer_id).is_ok_and(|id| id == p.node))
                .ok_or_else(|| mapped(Error::PeerNotAllowed))?;
            let fragment = if let Some(id) = id {
                let n = Arc::clone(&node);
                Some(
                    tauri::async_runtime::spawn_blocking(move || n.entry(id))
                        .await
                        .map_err(|_| mapped(Error::TransportUnavailable))?
                        .map_err(mapped)?,
                )
            } else {
                None
            };
            let ctx = OperationContext {
                cancellation: CancellationToken::new(),
                deadline: Instant::now()
                    + Duration::from_secs(
                        u64::from(config.timeout_seconds) * (u64::from(config.retries) + 1) + 2,
                    ),
            };
            // Never hold the session lock while waiting for the network: Stop remains usable.
            drop(session);
            let (attempts, available) = match (operation, fragment.as_ref()) {
                ("probe", _) => (service.probe(&peer, &ctx).await.map_err(mapped)?, None),
                ("availability", Some(f)) => {
                    let (a, n) = service
                        .availability(&peer, std::slice::from_ref(f), &ctx)
                        .await
                        .map_err(mapped)?;
                    (n, a.first().copied())
                }
                ("download", Some(f)) => {
                    (service.fetch(&peer, f, &ctx).await.map_err(mapped)?, None)
                }
                ("upload", Some(f)) => (service.push(&peer, f, &ctx).await.map_err(mapped)?, None),
                _ => return Err(mapped(Error::Protocol)),
            };
            outcome = Some(NetworkOutcome {
                operation: operation.into(),
                peer_id,
                bytes: fragment
                    .map(|f| f.bytes.to_string())
                    .unwrap_or_else(|| "0".into()),
                attempts,
                available,
            });
            session = state.network.lock().await;
        }
    }
    Ok(NetworkSnapshot {
        running: session.as_ref().is_some_and(|s| s.running()),
        node_id: crate::g4ep::local::hex(Sha256Digest(node.node_id().0)),
        certificate_sha256: identity.fingerprint(),
        config,
        connections: session.as_ref().map_or(0, |s| s.transport.connections()),
        outcome,
    })
}
use tokio_util::sync::CancellationToken;
