//! Private direct fragment protocol v1, mutually authenticated TLS 1.3.
//! No DNS, DHT, tracker, relay, public bootstrap or automatic listener startup.
use super::{
    guardian::GuardianNode,
    local,
    ports::{AsyncReader, AsyncWriter, FragmentTransfer, PeerDiscovery, PortFuture, StorageNode},
    types::*,
};
use rustls::{
    RootCertStore,
    pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{self},
    future::Future,
    io::Write,
    net::{IpAddr, SocketAddr},
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::Semaphore,
    task::{JoinHandle, JoinSet},
};
use tokio_rustls::{TlsAcceptor, TlsConnector};
use tokio_util::{io::SyncIoBridge, sync::CancellationToken};
use ts_rs::TS;
const FRAME_LIMIT: usize = 32768;
const PROFILE_LIMIT: u64 = 16384;
const ALPN: &[u8] = b"game4-g4ep-private/1";
const SERVER_NAME: &str = "guardian.game4.local";
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PeerProfile {
    pub version: u32,
    pub node_id: String,
    pub address: String,
    pub certificate_sha256: String,
    pub certificate_der: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManualConfig {
    pub bind: String,
    pub max_connections: u8,
    pub timeout_seconds: u32,
    pub retries: u8,
    pub peers: Vec<PeerProfile>,
}
impl Default for ManualConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:42430".into(),
            max_connections: 4,
            timeout_seconds: 120,
            retries: 2,
            peers: vec![],
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NetworkOutcome {
    pub operation: String,
    pub peer_id: String,
    pub bytes: String,
    pub attempts: u8,
    pub available: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NetworkSnapshot {
    pub running: bool,
    pub node_id: String,
    pub certificate_sha256: String,
    pub config: ManualConfig,
    pub connections: u32,
    pub outcome: Option<NetworkOutcome>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Secret {
    version: u32,
    node_id: String,
    certificate_der: String,
    private_key_der: String,
}
pub struct Identity {
    node: NodeId,
    certificate: Vec<u8>,
    key: Vec<u8>,
}
fn io() -> Error {
    Error::TransportUnavailable
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn unhex(s: &str, max: usize) -> Result<Vec<u8>> {
    if s.is_empty()
        || s.len() > max * 2
        || !s.len().is_multiple_of(2)
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::InvalidNetwork);
    }
    s.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| {
            u8::from_str_radix(
                std::str::from_utf8(b).map_err(|_| Error::InvalidNetwork)?,
                16,
            )
            .map_err(|_| Error::InvalidNetwork)
        })
        .collect()
}
fn hash(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}
pub fn node_id(s: &str) -> Result<NodeId> {
    let bytes = unhex(s, 32)?;
    Ok(NodeId(bytes.try_into().map_err(|_| Error::InvalidNetwork)?))
}
pub fn private_address(s: &str) -> Result<SocketAddr> {
    let a: SocketAddr = s.parse().map_err(|_| Error::InvalidNetwork)?;
    if a.to_string() != s || s.contains('%') {
        return Err(Error::InvalidNetwork);
    }
    PrivateNetwork::new(
        a,
        vec![Peer {
            node: NodeId([0; 32]),
            address: a,
        }],
    )?;
    Ok(a)
}
fn checked_path(path: &Path) -> Result<()> {
    for a in path.ancestors() {
        match fs::symlink_metadata(a) {
            Ok(_) => return crate::library::reject_ancestors(a).map_err(|_| io()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Err(io())
}
impl Identity {
    /// Keep this directory private, separate from any exported profile or fragment directory.
    pub fn load(node: NodeId, directory: &Path) -> Result<Arc<Self>> {
        if !directory.is_absolute() {
            return Err(Error::InvalidNetwork);
        }
        checked_path(directory)?;
        fs::create_dir_all(directory)?;
        checked_path(directory)?;
        let path = directory.join(format!("{}.tls.json", hex(&node.0)));
        checked_path(&path)?;
        let secret = if path.exists() {
            let file = crate::library::open_source(&path).map_err(|_| io())?;
            if file.metadata()?.len() > 32768 {
                return Err(Error::InvalidNetwork);
            }
            serde_json::from_reader::<_, Secret>(file).map_err(|_| Error::InvalidNetwork)?
        } else {
            let generated = rcgen::generate_simple_self_signed(vec![SERVER_NAME.to_string()])
                .map_err(|_| io())?;
            let s = Secret {
                version: 1,
                node_id: hex(&node.0),
                certificate_der: hex(generated.cert.der().as_ref()),
                private_key_der: hex(&generated.signing_key.serialize_der()),
            };
            let mut temp = tempfile::NamedTempFile::new_in(directory)?;
            serde_json::to_writer(&mut temp, &s).map_err(|_| io())?;
            temp.as_file().sync_all()?;
            temp.persist_noclobber(&path)
                .map_err(|e| Error::Io(e.error))?;
            s
        };
        if secret.version != 1 || node_id(&secret.node_id)? != node {
            return Err(Error::InvalidNetwork);
        }
        Ok(Arc::new(Self {
            node,
            certificate: unhex(&secret.certificate_der, 8192)?,
            key: unhex(&secret.private_key_der, 8192)?,
        }))
    }
    pub fn profile(&self, address: &str) -> Result<PeerProfile> {
        private_address(address)?;
        Ok(PeerProfile {
            version: 1,
            node_id: hex(&self.node.0),
            address: address.into(),
            certificate_sha256: hash(&self.certificate),
            certificate_der: hex(&self.certificate),
        })
    }
    pub fn fingerprint(&self) -> String {
        hash(&self.certificate)
    }
}
impl PeerProfile {
    pub fn peer(&self) -> Result<Peer> {
        let certificate = unhex(&self.certificate_der, 8192)?;
        if self.version != 1 || self.certificate_sha256 != hash(&certificate) {
            return Err(Error::InvalidNetwork);
        }
        let mut roots = RootCertStore::empty();
        roots
            .add(CertificateDer::from(certificate))
            .map_err(|_| Error::InvalidNetwork)?;
        Ok(Peer {
            node: node_id(&self.node_id)?,
            address: private_address(&self.address)?,
        })
    }
}
impl ManualConfig {
    pub fn validate(&self, local: NodeId) -> Result<PrivateNetwork> {
        if !(1..=16).contains(&self.max_connections)
            || !(2..=600).contains(&self.timeout_seconds)
            || self.retries > 3
            || self.peers.is_empty()
            || self.peers.len() > 32
        {
            return Err(Error::InvalidNetwork);
        }
        let peers = self
            .peers
            .iter()
            .map(PeerProfile::peer)
            .collect::<Result<Vec<_>>>()?;
        if peers.iter().any(|p| p.node == local) {
            return Err(Error::InvalidNetwork);
        }
        let mut certs = HashSet::new();
        if self
            .peers
            .iter()
            .any(|p| !certs.insert(&p.certificate_sha256))
        {
            return Err(Error::InvalidNetwork);
        }
        PrivateNetwork::new(private_address(&self.bind)?, peers)
    }
}
pub fn read_profile(path: &Path) -> Result<PeerProfile> {
    let f = crate::library::open_source(path).map_err(|_| io())?;
    if f.metadata()?.len() > PROFILE_LIMIT {
        return Err(Error::InvalidNetwork);
    }
    let p: PeerProfile = serde_json::from_reader(f).map_err(|_| Error::InvalidNetwork)?;
    p.peer()?;
    Ok(p)
}
pub fn write_profile(path: &Path, p: &PeerProfile) -> Result<()> {
    p.peer()?;
    checked_path(path)?;
    let mut f = tempfile::NamedTempFile::new_in(path.parent().ok_or_else(io)?)?;
    serde_json::to_writer_pretty(&mut f, p).map_err(|_| io())?;
    f.flush()?;
    f.as_file().sync_all()?;
    f.persist_noclobber(path).map_err(|e| Error::Io(e.error))?;
    Ok(())
}
pub fn validate_settings(c: &ManualConfig, node: NodeId) -> Result<()> {
    private_address(&c.bind)?;
    if !(1..=16).contains(&c.max_connections)
        || !(2..=600).contains(&c.timeout_seconds)
        || c.retries > 3
        || c.peers.len() > 32
    {
        return Err(Error::InvalidNetwork);
    }
    if !c.peers.is_empty() {
        c.validate(node)?;
    }
    Ok(())
}
pub fn read_config(node: &GuardianNode) -> Result<ManualConfig> {
    let p = node.storage_root().join("manual-network.json");
    checked_path(&p)?;
    if !p.exists() {
        return Ok(ManualConfig::default());
    }
    let f = crate::library::open_source(&p).map_err(|_| io())?;
    if f.metadata()?.len() > 524288 {
        return Err(Error::InvalidNetwork);
    }
    let c: ManualConfig = serde_json::from_reader(f).map_err(|_| Error::InvalidNetwork)?;
    validate_settings(&c, node.node_id())?;
    Ok(c)
}
pub fn save_config(node: &GuardianNode, c: &ManualConfig) -> Result<()> {
    validate_settings(c, node.node_id())?;
    let p = node.storage_root().join("manual-network.json");
    checked_path(&p)?;
    let mut f = tempfile::NamedTempFile::new_in(node.storage_root())?;
    serde_json::to_writer_pretty(&mut f, c).map_err(|_| io())?;
    f.as_file().sync_all()?;
    f.persist(p).map_err(|e| Error::Io(e.error))?;
    Ok(())
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Fragment {
    group: String,
    index: u8,
    bytes: u32,
    sha256: String,
}
impl Fragment {
    fn from(f: &FragmentDescriptor) -> Self {
        Self {
            group: f.coordinate.stripe.to_string(),
            index: f.coordinate.shard,
            bytes: f.bytes,
            sha256: local::hex(f.sha256),
        }
    }
    fn checked(&self) -> Result<FragmentDescriptor> {
        let group = self
            .group
            .parse::<u64>()
            .ok()
            .filter(|g| g.to_string() == self.group && *g < 83333)
            .ok_or(Error::Protocol)?;
        if self.index >= 12
            || !(65536..=4194304).contains(&self.bytes)
            || !self.bytes.is_multiple_of(64)
        {
            return Err(Error::Protocol);
        }
        let bytes = unhex(&self.sha256, 32)?;
        Ok(FragmentDescriptor {
            coordinate: FragmentCoordinate {
                stripe: group,
                shard: self.index,
            },
            bytes: self.bytes,
            sha256: Sha256Digest(bytes.try_into().map_err(|_| Error::Protocol)?),
        })
    }
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum Operation {
    Probe,
    Available { fragments: Vec<Fragment> },
    Get { fragment: Fragment },
    Put { fragment: Fragment },
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Request {
    version: u32,
    node_id: String,
    operation: Operation,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Response {
    version: u32,
    node_id: String,
    status: String,
    available: Vec<bool>,
}
async fn bounded<T>(
    ctx: &OperationContext,
    seconds: u32,
    work: impl Future<Output = Result<T>>,
) -> Result<T> {
    ctx.checkpoint()?;
    let end = tokio::time::Instant::from_std(
        ctx.deadline
            .min(Instant::now() + Duration::from_secs(u64::from(seconds))),
    );
    tokio::select! {biased; _=ctx.cancellation.cancelled()=>Err(Error::Cancelled),r=tokio::time::timeout_at(end,work)=>r.map_err(|_|Error::NetworkTimeout)?}
}
async fn send<T: Serialize>(s: &mut (impl AsyncWrite + Unpin), v: &T) -> Result<()> {
    let b = serde_json::to_vec(v).map_err(|_| Error::Protocol)?;
    if b.len() > FRAME_LIMIT {
        return Err(Error::Protocol);
    }
    s.write_u32(b.len() as u32).await?;
    s.write_all(&b).await?;
    s.flush().await?;
    Ok(())
}
async fn receive<T: serde::de::DeserializeOwned>(s: &mut (impl AsyncRead + Unpin)) -> Result<T> {
    let n = s.read_u32().await? as usize;
    if n == 0 || n > FRAME_LIMIT {
        return Err(Error::Protocol);
    }
    let mut b = vec![0; n];
    s.read_exact(&mut b).await?;
    crate::ever::check_json(&b).map_err(|_| Error::Protocol)?;
    serde_json::from_slice(&b).map_err(|_| Error::Protocol)
}
fn tls_error(e: std::io::Error) -> Error {
    if matches!(
        e.kind(),
        std::io::ErrorKind::InvalidData | std::io::ErrorKind::PermissionDenied
    ) {
        Error::PeerAuthentication
    } else {
        Error::Io(e)
    }
}
fn remote_error(s: &str) -> Error {
    match s {
        "disabled" => Error::GuardianDisabled,
        "denied" => Error::GuardianNotApproved,
        "quota" => Error::QuotaExceeded,
        "integrity" => Error::IntegrityMismatch,
        "busy" => Error::ConnectionLimit,
        _ => Error::Protocol,
    }
}
fn error_status(e: &Error) -> &'static str {
    match e {
        Error::GuardianDisabled => "disabled",
        Error::GuardianNotApproved | Error::FragmentNotFound => "denied",
        Error::QuotaExceeded => "quota",
        Error::IntegrityMismatch => "integrity",
        Error::ConnectionLimit => "busy",
        _ => "unavailable",
    }
}
pub struct ManualTransport {
    node: Arc<GuardianNode>,
    identity: Arc<Identity>,
    config: ManualConfig,
    network: PrivateNetwork,
    slots: Arc<Semaphore>,
    cancel: CancellationToken,
}
pub struct RunningNode {
    pub transport: Arc<ManualTransport>,
    task: Option<JoinHandle<()>>,
}
impl RunningNode {
    pub fn stop(&self) {
        self.transport.cancel.cancel()
    }
    pub async fn stopped(mut self) {
        self.stop();
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }
    pub fn running(&self) -> bool {
        self.task.as_ref().is_some_and(|t| !t.is_finished())
            && !self.transport.cancel.is_cancelled()
    }
}
impl Drop for RunningNode {
    fn drop(&mut self) {
        self.stop();
    }
}
impl ManualTransport {
    pub fn new(
        node: Arc<GuardianNode>,
        identity: Arc<Identity>,
        config: ManualConfig,
    ) -> Result<Arc<Self>> {
        let network = config.validate(node.node_id())?;
        if node.node_id() != identity.node {
            return Err(Error::PeerAuthentication);
        }
        Ok(Arc::new(Self {
            node,
            identity,
            slots: Arc::new(Semaphore::new(config.max_connections as usize)),
            config,
            network,
            cancel: CancellationToken::new(),
        }))
    }
    fn peer_profile(&self, peer: &Peer) -> Result<&PeerProfile> {
        self.network.require_peer(peer)?;
        self.config
            .peers
            .iter()
            .find(|p| p.node_id == hex(&peer.node.0) && p.address == peer.address.to_string())
            .ok_or(Error::PeerNotAllowed)
    }
    fn tls_server(&self) -> Result<rustls::ServerConfig> {
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let mut roots = RootCertStore::empty();
        for p in &self.config.peers {
            roots
                .add(CertificateDer::from(unhex(&p.certificate_der, 8192)?))
                .map_err(|_| Error::InvalidNetwork)?;
        }
        let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
            Arc::new(roots),
            Arc::clone(&provider),
        )
        .build()
        .map_err(|_| Error::InvalidNetwork)?;
        let mut cfg = rustls::ServerConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(|_| io())?
            .with_client_cert_verifier(verifier)
            .with_single_cert(
                vec![CertificateDer::from(self.identity.certificate.clone())],
                PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(self.identity.key.clone())),
            )
            .map_err(|_| io())?;
        cfg.alpn_protocols = vec![ALPN.to_vec()];
        cfg.max_early_data_size = 0;
        Ok(cfg)
    }
    fn tls_client(&self, peer: &Peer) -> Result<rustls::ClientConfig> {
        let p = self.peer_profile(peer)?;
        let mut roots = RootCertStore::empty();
        roots
            .add(CertificateDer::from(unhex(&p.certificate_der, 8192)?))
            .map_err(|_| Error::InvalidNetwork)?;
        let mut cfg = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|_| io())?
        .with_root_certificates(roots)
        .with_client_auth_cert(
            vec![CertificateDer::from(self.identity.certificate.clone())],
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(self.identity.key.clone())),
        )
        .map_err(|_| io())?;
        cfg.alpn_protocols = vec![ALPN.to_vec()];
        cfg.enable_early_data = false;
        Ok(cfg)
    }
    async fn pace(&self, upload: bool, bytes: usize, ctx: &OperationContext) -> Result<()> {
        let node = Arc::clone(&self.node);
        let c = ctx.clone();
        tokio::task::spawn_blocking(move || node.pace_transfer(upload, bytes, &c))
            .await
            .map_err(|_| io())?
    }
    pub fn peers(&self) -> &[Peer] {
        self.network.peers()
    }
    pub fn connections(&self) -> u32 {
        u32::from(self.config.max_connections) - self.slots.available_permits() as u32
    }
    pub async fn start(self: &Arc<Self>) -> Result<RunningNode> {
        if !self.node.participation_active() {
            return Err(Error::GuardianDisabled);
        }
        let listener = TcpListener::bind(self.network.bind()).await?;
        let acceptor = TlsAcceptor::from(Arc::new(self.tls_server()?));
        let service = Arc::clone(self);
        let task = tokio::spawn(async move {
            let mut tasks = JoinSet::new();
            loop {
                tokio::select! {
                    _=service.cancel.cancelled()=>break,
                    _=tokio::time::sleep(Duration::from_millis(100))=>{if !service.node.participation_active(){break}},
                    Some(_)=tasks.join_next(),if !tasks.is_empty()=>{},
                    r=listener.accept()=>{let Ok((socket,address))=r else{break};if !service.node.participation_active()||!service.config.peers.iter().any(|p|p.peer().is_ok_and(|p|p.address.ip()==address.ip())){continue}
                        let Ok(permit)=Arc::clone(&service.slots).try_acquire_owned() else{continue};let service=Arc::clone(&service);let acceptor=acceptor.clone();
                        tasks.spawn(async move{let _permit=permit;let ctx=OperationContext{cancellation:service.cancel.child_token(),deadline:Instant::now()+Duration::from_secs(u64::from(service.config.timeout_seconds))};let _=bounded(&ctx,service.config.timeout_seconds,service.serve(socket,address.ip(),acceptor,&ctx)).await;ctx.cancellation.cancel();});
                    }
                }
            }
            service.cancel.cancel();
            while tasks.join_next().await.is_some() {}
        });
        Ok(RunningNode {
            transport: Arc::clone(self),
            task: Some(task),
        })
    }
    async fn connect(
        &self,
        peer: &Peer,
        ctx: &OperationContext,
    ) -> Result<tokio_rustls::client::TlsStream<TcpStream>> {
        let p = self.peer_profile(peer)?;
        let socket = bounded(ctx, 5, async {
            Ok(TcpStream::connect(peer.address).await?)
        })
        .await?;
        socket.set_nodelay(true)?;
        let connector = TlsConnector::from(Arc::new(self.tls_client(peer)?));
        let stream = bounded(ctx, 10, async {
            connector
                .connect(
                    ServerName::try_from(SERVER_NAME.to_string())
                        .map_err(|_| Error::InvalidNetwork)?,
                    socket,
                )
                .await
                .map_err(tls_error)
        })
        .await?;
        let state = stream.get_ref().1;
        if state.alpn_protocol() != Some(ALPN)
            || state
                .peer_certificates()
                .and_then(|v| v.first())
                .is_none_or(|c| hash(c.as_ref()) != p.certificate_sha256)
        {
            return Err(Error::PeerAuthentication);
        }
        Ok(stream)
    }
    fn response(&self, status: &str, available: Vec<bool>) -> Response {
        Response {
            version: 1,
            node_id: hex(&self.identity.node.0),
            status: status.into(),
            available,
        }
    }
    fn check_response(&self, r: &Response, peer: &Peer) -> Result<()> {
        if r.version != 1 || r.node_id != hex(&peer.node.0) {
            return Err(Error::PeerAuthentication);
        }
        if r.status != "ok" {
            return Err(remote_error(&r.status));
        }
        Ok(())
    }
    async fn serve(
        &self,
        socket: TcpStream,
        ip: IpAddr,
        acceptor: TlsAcceptor,
        ctx: &OperationContext,
    ) -> Result<()> {
        let mut stream = bounded(ctx, 10, async {
            acceptor.accept(socket).await.map_err(tls_error)
        })
        .await?;
        let state = stream.get_ref().1;
        let cert = state
            .peer_certificates()
            .and_then(|v| v.first())
            .ok_or(Error::PeerAuthentication)?;
        if state.alpn_protocol() != Some(ALPN) {
            return Err(Error::PeerAuthentication);
        }
        let pin = hash(cert.as_ref());
        let profile = self
            .config
            .peers
            .iter()
            .find(|p| p.certificate_sha256 == pin && p.peer().is_ok_and(|p| p.address.ip() == ip))
            .ok_or(Error::PeerAuthentication)?;
        let request: Request = receive(&mut stream).await?;
        if request.version != 1 || request.node_id != profile.node_id {
            return Err(Error::PeerAuthentication);
        }
        if !self.node.participation_active() {
            send(&mut stream, &self.response("disabled", vec![])).await?;
            return Err(Error::GuardianDisabled);
        }
        match request.operation {
            Operation::Probe => send(&mut stream, &self.response("ok", vec![])).await?,
            Operation::Available { fragments } => {
                if fragments.len() > 100 {
                    return Err(Error::MetadataLimit);
                }
                let fragments = fragments
                    .iter()
                    .map(Fragment::checked)
                    .collect::<Result<Vec<_>>>()?;
                let node = Arc::clone(&self.node);
                let c = ctx.clone();
                let available =
                    tokio::task::spawn_blocking(move || node.shared_availability(&fragments, &c))
                        .await
                        .map_err(|_| io())??;
                send(&mut stream, &self.response("ok", available)).await?;
            }
            Operation::Get { fragment } => {
                let fragment = fragment.checked()?;
                let node = Arc::clone(&self.node);
                let c = ctx.clone();
                let f = fragment.clone();
                let reader = tokio::task::spawn_blocking(move || node.read_fragment(&f, &c))
                    .await
                    .map_err(|_| io())?;
                let mut reader = match reader {
                    Ok(r) => r,
                    Err(e) => {
                        send(&mut stream, &self.response(error_status(&e), vec![])).await?;
                        return Err(e);
                    }
                };
                send(&mut stream, &self.response("ok", vec![])).await?;
                let (mut rx, tx) = tokio::io::duplex(65536);
                let task = tokio::task::spawn_blocking(move || {
                    std::io::copy(&mut reader, &mut SyncIoBridge::new(tx)).map_err(Error::Io)
                });
                let copied = tokio::io::copy(&mut rx, &mut stream).await?;
                task.await.map_err(|_| io())??;
                if copied != u64::from(fragment.bytes) {
                    return Err(Error::IntegrityMismatch);
                }
                stream.flush().await?;
            }
            Operation::Put { fragment } => {
                let fragment = fragment.checked()?;
                let node = Arc::clone(&self.node);
                let f = fragment.clone();
                let ready = tokio::task::spawn_blocking(move || node.can_receive(&f))
                    .await
                    .map_err(|_| io())?;
                if let Err(e) = ready {
                    send(&mut stream, &self.response(error_status(&e), vec![])).await?;
                    return Err(e);
                }
                send(&mut stream, &self.response("ok", vec![])).await?;
                let result = self.receive_into(&mut stream, &fragment, ctx).await;
                send(
                    &mut stream,
                    &self.response(
                        result.as_ref().map(|_| "ok").unwrap_or_else(error_status),
                        vec![],
                    ),
                )
                .await?;
                result?;
            }
        }
        Ok(())
    }
    async fn receive_into(
        &self,
        input: &mut (impl AsyncRead + Unpin),
        f: &FragmentDescriptor,
        ctx: &OperationContext,
    ) -> Result<StorageReceipt> {
        let (rx, mut tx) = tokio::io::duplex(65536);
        let node = Arc::clone(&self.node);
        let fragment = f.clone();
        let c = ctx.clone();
        let commit = tokio::task::spawn_blocking(move || {
            node.commit_fragment(&fragment, &mut SyncIoBridge::new(rx), &c)
        });
        let mut take = input.take(u64::from(f.bytes));
        let copied = tokio::io::copy(&mut take, &mut tx).await;
        drop(tx);
        let committed = commit.await.map_err(|_| io())?;
        let copied = copied?;
        if copied != u64::from(f.bytes) {
            return Err(Error::IntegrityMismatch);
        }
        committed
    }
    async fn retry<T, F, Fut>(&self, ctx: &OperationContext, mut work: F) -> Result<(T, u8)>
    where
        F: FnMut(OperationContext) -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        for attempt in 0..=self.config.retries {
            if !self.node.participation_active() {
                return Err(Error::GuardianDisabled);
            }
            if self.cancel.is_cancelled() {
                return Err(Error::Cancelled);
            }
            let permit = Arc::clone(&self.slots)
                .try_acquire_owned()
                .map_err(|_| Error::ConnectionLimit)?;
            let attempt_ctx = OperationContext {
                cancellation: ctx.cancellation.child_token(),
                deadline: ctx.deadline,
            };
            let result = tokio::select! {biased;_=self.cancel.cancelled()=>Err(Error::Cancelled),r=bounded(&attempt_ctx,self.config.timeout_seconds,work(attempt_ctx.clone()))=>r};
            attempt_ctx.cancellation.cancel();
            drop(permit);
            match result {
                Ok(v) => return Ok((v, attempt + 1)),
                Err(e) => {
                    let transient = matches!(
                        e,
                        Error::Io(_) | Error::NetworkTimeout | Error::ConnectionLimit
                    );
                    if !transient || attempt == self.config.retries {
                        return Err(e);
                    }
                    bounded(ctx, 2, async {
                        tokio::time::sleep(Duration::from_millis(100 * (1 << attempt))).await;
                        Ok(())
                    })
                    .await?;
                }
            }
        }
        Err(io())
    }
    pub async fn probe(&self, peer: &Peer, ctx: &OperationContext) -> Result<u8> {
        self.retry(ctx, |attempt| async move {
            let mut s = self.connect(peer, &attempt).await?;
            send(
                &mut s,
                &Request {
                    version: 1,
                    node_id: hex(&self.identity.node.0),
                    operation: Operation::Probe,
                },
            )
            .await?;
            let r = receive(&mut s).await?;
            self.check_response(&r, peer)
        })
        .await
        .map(|(_, a)| a)
    }
    pub async fn availability(
        &self,
        peer: &Peer,
        fragments: &[FragmentDescriptor],
        ctx: &OperationContext,
    ) -> Result<(Vec<bool>, u8)> {
        if fragments.len() > 100 {
            return Err(Error::MetadataLimit);
        }
        self.retry(ctx, |attempt| async move {
            let mut s = self.connect(peer, &attempt).await?;
            send(
                &mut s,
                &Request {
                    version: 1,
                    node_id: hex(&self.identity.node.0),
                    operation: Operation::Available {
                        fragments: fragments.iter().map(Fragment::from).collect(),
                    },
                },
            )
            .await?;
            let r: Response = receive(&mut s).await?;
            self.check_response(&r, peer)?;
            if r.available.len() != fragments.len() {
                return Err(Error::Protocol);
            }
            Ok(r.available)
        })
        .await
    }
    pub async fn fetch(
        &self,
        peer: &Peer,
        f: &FragmentDescriptor,
        ctx: &OperationContext,
    ) -> Result<u8> {
        let node = Arc::clone(&self.node);
        let fragment = f.clone();
        tokio::task::spawn_blocking(move || node.can_receive(&fragment))
            .await
            .map_err(|_| io())??;
        self.retry(ctx, |attempt| async move {
            let mut s = self.connect(peer, &attempt).await?;
            send(
                &mut s,
                &Request {
                    version: 1,
                    node_id: hex(&self.identity.node.0),
                    operation: Operation::Get {
                        fragment: Fragment::from(f),
                    },
                },
            )
            .await?;
            let r: Response = receive(&mut s).await?;
            self.check_response(&r, peer)?;
            self.receive_into(&mut s, f, &attempt).await?;
            Ok(())
        })
        .await
        .map(|(_, a)| a)
    }
    pub async fn push(
        &self,
        peer: &Peer,
        f: &FragmentDescriptor,
        ctx: &OperationContext,
    ) -> Result<u8> {
        self.retry(ctx, |attempt| async move {
            let node = Arc::clone(&self.node);
            let fragment = f.clone();
            let c = attempt.clone();
            let reader = tokio::task::spawn_blocking(move || node.read_fragment(&fragment, &c))
                .await
                .map_err(|_| io())??;
            let mut s = self.connect(peer, &attempt).await?;
            send(
                &mut s,
                &Request {
                    version: 1,
                    node_id: hex(&self.identity.node.0),
                    operation: Operation::Put {
                        fragment: Fragment::from(f),
                    },
                },
            )
            .await?;
            let r: Response = receive(&mut s).await?;
            self.check_response(&r, peer)?;
            let (mut rx, tx) = tokio::io::duplex(65536);
            let worker = tokio::task::spawn_blocking(move || {
                let mut reader = reader;
                std::io::copy(&mut reader, &mut SyncIoBridge::new(tx)).map_err(Error::Io)
            });
            let sent = tokio::io::copy(&mut rx, &mut s).await?;
            worker.await.map_err(|_| io())??;
            if sent != u64::from(f.bytes) {
                return Err(Error::IntegrityMismatch);
            }
            s.flush().await?;
            let response = receive(&mut s).await?;
            self.check_response(&response, peer)
        })
        .await
        .map(|(_, a)| a)
    }
}
impl PeerDiscovery for ManualTransport {
    fn discover<'a>(
        &'a self,
        network: &'a PrivateNetwork,
        ctx: &'a OperationContext,
    ) -> PortFuture<'a, Vec<Peer>> {
        Box::pin(async move {
            let mut found = Vec::new();
            for peer in network.peers() {
                self.peer_profile(peer)?;
                ctx.checkpoint()?;
                match self.probe(peer, ctx).await {
                    Ok(_) => found.push(*peer),
                    Err(Error::Cancelled | Error::Deadline) => return Err(Error::Cancelled),
                    Err(_) => {}
                }
            }
            Ok(found)
        })
    }
}
impl FragmentTransfer for ManualTransport {
    fn upload<'a>(
        &'a self,
        network: &'a PrivateNetwork,
        peer: &'a Peer,
        f: &'a FragmentDescriptor,
        source: &'a mut AsyncReader,
        ctx: &'a OperationContext,
    ) -> PortFuture<'a, StorageReceipt> {
        Box::pin(async move {
            network.require_peer(peer)?;
            self.peer_profile(peer)?;
            Fragment::from(f).checked()?;
            let node = Arc::clone(&self.node);
            let fragment = f.clone();
            tokio::task::spawn_blocking(move || node.can_share(&fragment))
                .await
                .map_err(|_| io())??;
            let buffer_permit = Arc::clone(&self.slots)
                .try_acquire_owned()
                .map_err(|_| Error::ConnectionLimit)?;
            // A single <=4 MiB bounded replay buffer permits safe retries of a caller-owned stream.
            let bytes = bounded(ctx, self.config.timeout_seconds, async {
                let mut b = Vec::with_capacity(f.bytes as usize);
                source
                    .take(u64::from(f.bytes) + 1)
                    .read_to_end(&mut b)
                    .await?;
                if b.len() != f.bytes as usize || Sha256::digest(&b).as_slice() != f.sha256.0 {
                    return Err(Error::IntegrityMismatch);
                }
                Ok(b)
            })
            .await?;
            drop(buffer_permit);
            let bytes = &bytes;
            self.retry(ctx, |attempt| async move {
                let mut s = self.connect(peer, &attempt).await?;
                send(
                    &mut s,
                    &Request {
                        version: 1,
                        node_id: hex(&self.identity.node.0),
                        operation: Operation::Put {
                            fragment: Fragment::from(f),
                        },
                    },
                )
                .await?;
                let r = receive(&mut s).await?;
                self.check_response(&r, peer)?;
                for chunk in bytes.chunks(65536) {
                    self.pace(true, chunk.len(), &attempt).await?;
                    s.write_all(chunk).await?;
                }
                s.flush().await?;
                let r = receive(&mut s).await?;
                self.check_response(&r, peer)
            })
            .await?;
            Ok(StorageReceipt {
                node: peer.node,
                sha256: f.sha256,
                bytes: f.bytes,
                observed_at: Instant::now(),
            })
        })
    }
    fn download<'a>(
        &'a self,
        network: &'a PrivateNetwork,
        peer: &'a Peer,
        f: &'a FragmentDescriptor,
        output: &'a mut AsyncWriter,
        ctx: &'a OperationContext,
    ) -> PortFuture<'a, TransferReport> {
        Box::pin(async move {
            network.require_peer(peer)?;
            self.peer_profile(peer)?;
            Fragment::from(f).checked()?;
            let node = Arc::clone(&self.node);
            let fragment = f.clone();
            tokio::task::spawn_blocking(move || node.can_receive(&fragment))
                .await
                .map_err(|_| io())??;
            // Retry before publication: bounded verified bytes prevent duplicate partial writes to output.
            let (bytes, _) = self
                .retry(ctx, |attempt| async move {
                    let mut s = self.connect(peer, &attempt).await?;
                    send(
                        &mut s,
                        &Request {
                            version: 1,
                            node_id: hex(&self.identity.node.0),
                            operation: Operation::Get {
                                fragment: Fragment::from(f),
                            },
                        },
                    )
                    .await?;
                    let r = receive(&mut s).await?;
                    self.check_response(&r, peer)?;
                    let mut bytes = vec![0; f.bytes as usize];
                    for chunk in bytes.chunks_mut(65536) {
                        self.pace(false, chunk.len(), &attempt).await?;
                        s.read_exact(chunk).await?;
                    }
                    if Sha256::digest(&bytes).as_slice() != f.sha256.0 {
                        return Err(Error::IntegrityMismatch);
                    }
                    Ok(bytes)
                })
                .await?;
            bounded(ctx, self.config.timeout_seconds, async {
                output.write_all(&bytes).await?;
                output.flush().await?;
                Ok(())
            })
            .await?;
            Ok(TransferReport {
                peer: *peer,
                fragment: f.clone(),
                transferred_bytes: u64::from(f.bytes),
            })
        })
    }
}

#[cfg(test)]
mod tests;
