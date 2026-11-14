//! Checked planning values. Constructors perform no disk or network I/O.
use std::{collections::HashSet, net::SocketAddr, time::Instant};
use tokio_util::sync::CancellationToken;

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("source length differs from its declared length")]
    SourceLengthMismatch,
    #[error("source changed while encoding")]
    SourceChanged,
    #[error("fragment sink did not consume the complete fragment")]
    IncompleteSink,
    #[error("destination already exists")]
    DestinationExists,
    #[error("invalid experimental encoding bundle")]
    InvalidBundle,
    #[error("experimental bundle metadata limit exceeded")]
    MetadataLimit,
    #[error("Reed-Solomon operation failed")]
    Codec(#[from] reed_solomon_simd::Error),
    #[error("invalid coding profile")]
    InvalidProfile,
    #[error("source exceeds the supported planning limit")]
    SourceLimit,
    #[error("invalid fragment map")]
    InvalidManifest,
    #[error("peer is outside the private allowlist")]
    PeerNotAllowed,
    #[error("invalid private network configuration")]
    InvalidNetwork,
    #[error("operation cancelled")]
    Cancelled,
    #[error("operation deadline reached")]
    Deadline,
    #[error("not enough distinct verified shards to reconstruct a stripe")]
    InsufficientShards,
    #[error("content integrity mismatch")]
    IntegrityMismatch,
    #[error("storage quota exceeded")]
    QuotaExceeded,
    #[error("fragment not found")]
    FragmentNotFound,
    #[error("transport unavailable")]
    TransportUnavailable,
    #[error("I/O operation failed")]
    Io(#[from] std::io::Error),
}

/// Full SHA-256 bytes; independent from UUIDs and BitTorrent SHA-1 info hashes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sha256Digest(pub [u8; 32]);
/// Expected enrolled peer identity. This value alone does not authenticate a socket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub [u8; 32]);

pub const MAX_SOURCE_BYTES: u64 = 1 << 40;
pub const MAX_FRAGMENT_RECORDS: u64 = 1_000_000;
pub const MAX_STRIPE_BUFFER_BYTES: u64 = 64 * 1024 * 1024;
/// Application codec registration key; no particular implementation or public wire ID is frozen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodecId(String);
impl CodecId {
    pub fn new(value: &str) -> Result<Self> {
        if value.is_empty()
            || value.len() > 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"-_.".contains(&b))
        {
            return Err(Error::InvalidManifest);
        }
        Ok(Self(value.into()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
/// Draft planning geometry, not a codec identifier. Exact codec/wire details remain gated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodingProfile {
    data_shards: u8,
    parity_shards: u8,
    shard_bytes: u32,
}
impl CodingProfile {
    pub fn new(data_shards: u8, parity_shards: u8, shard_bytes: u32) -> Result<Self> {
        let total = u16::from(data_shards) + u16::from(parity_shards);
        if data_shards < 2
            || parity_shards == 0
            || total > 255
            || !(64 * 1024..=4 * 1024 * 1024).contains(&shard_bytes)
            || u64::from(total) * u64::from(shard_bytes) > MAX_STRIPE_BUFFER_BYTES
        {
            return Err(Error::InvalidProfile);
        }
        Ok(Self {
            data_shards,
            parity_shards,
            shard_bytes,
        })
    }
    pub fn data_shards(self) -> u8 {
        self.data_shards
    }
    pub fn parity_shards(self) -> u8 {
        self.parity_shards
    }
    pub fn total_shards(self) -> u16 {
        u16::from(self.data_shards) + u16::from(self.parity_shards)
    }
    pub fn shard_bytes(self) -> u32 {
        self.shard_bytes
    }
    pub fn stripe_data_bytes(self) -> u64 {
        u64::from(self.data_shards) * u64::from(self.shard_bytes)
    }
    pub fn stripe_buffer_bytes(self) -> u64 {
        u64::from(self.total_shards()) * u64::from(self.shard_bytes)
    }
    /// Counts are arithmetic only. No source is read and no fragments are allocated.
    pub fn layout(self, source_bytes: u64) -> Result<Layout> {
        if source_bytes > MAX_SOURCE_BYTES {
            return Err(Error::SourceLimit);
        }
        let stripes = source_bytes.div_ceil(self.stripe_data_bytes());
        let fragments = stripes
            .checked_mul(u64::from(self.total_shards()))
            .ok_or(Error::SourceLimit)?;
        if fragments > MAX_FRAGMENT_RECORDS {
            return Err(Error::SourceLimit);
        }
        Ok(Layout {
            source_bytes,
            stripes,
            fragments,
            encoded_bytes: fragments * u64::from(self.shard_bytes),
            padding_bytes: stripes * self.stripe_data_bytes() - source_bytes,
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    pub source_bytes: u64,
    pub stripes: u64,
    pub fragments: u64,
    pub encoded_bytes: u64,
    pub padding_bytes: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FragmentCoordinate {
    pub stripe: u64,
    pub shard: u8,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FragmentDescriptor {
    pub coordinate: FragmentCoordinate,
    pub bytes: u32,
    pub sha256: Sha256Digest,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectDescriptor {
    pub bytes: u64,
    pub sha256: Sha256Digest,
}
/// Untrusted metadata until validated. No filenames or filesystem paths are portable here.
#[derive(Debug, Clone)]
pub struct PreservationManifest {
    pub codec: CodecId,
    pub object: ObjectDescriptor,
    pub coding: CodingProfile,
    pub fragments: Vec<FragmentDescriptor>,
}
/// Structural validation is not verification of stored bytes or publisher authenticity.
#[derive(Debug, Clone)]
pub struct ValidatedManifest(PreservationManifest);
impl ValidatedManifest {
    pub fn new(manifest: PreservationManifest) -> Result<Self> {
        let layout = manifest.coding.layout(manifest.object.bytes)?;
        if manifest.fragments.len() as u64 != layout.fragments {
            return Err(Error::InvalidManifest);
        }
        let mut seen = HashSet::with_capacity(manifest.fragments.len());
        for f in &manifest.fragments {
            if f.coordinate.stripe >= layout.stripes
                || u16::from(f.coordinate.shard) >= manifest.coding.total_shards()
                || f.bytes != manifest.coding.shard_bytes()
                || !seen.insert(f.coordinate)
            {
                return Err(Error::InvalidManifest);
            }
        }
        Ok(Self(manifest))
    }
    pub fn manifest(&self) -> &PreservationManifest {
        &self.0
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Peer {
    pub node: NodeId,
    pub address: SocketAddr,
}
/// Explicit private-experiment enrollment. Does not imply TLS/authentication is implemented.
#[derive(Debug, Clone)]
pub struct PrivateNetwork {
    bind: SocketAddr,
    peers: Vec<Peer>,
}
impl PrivateNetwork {
    pub fn new(bind: SocketAddr, peers: Vec<Peer>) -> Result<Self> {
        let private_ip = |ip: std::net::IpAddr| match ip {
            std::net::IpAddr::V4(ip) => {
                ip.is_private()
                    || ip.is_loopback()
                    || ip.is_link_local()
                    || (ip.octets()[0] == 100 && (64..=127).contains(&ip.octets()[1]))
            }
            std::net::IpAddr::V6(ip) => {
                ip.is_loopback() || ip.is_unique_local() || ip.is_unicast_link_local()
            }
        };
        let invalid = |address: SocketAddr| address.port() == 0 || !private_ip(address.ip());
        if invalid(bind) || peers.is_empty() || peers.len() > 32 {
            return Err(Error::InvalidNetwork);
        }
        let mut ids = HashSet::new();
        let mut addresses = HashSet::new();
        for p in &peers {
            if invalid(p.address) || !ids.insert(p.node) || !addresses.insert(p.address) {
                return Err(Error::InvalidNetwork);
            }
        }
        Ok(Self { bind, peers })
    }
    pub fn bind(&self) -> SocketAddr {
        self.bind
    }
    pub fn peers(&self) -> &[Peer] {
        &self.peers
    }
    pub fn require_peer(&self, peer: &Peer) -> Result<()> {
        if self.peers.contains(peer) {
            Ok(())
        } else {
            Err(Error::PeerNotAllowed)
        }
    }
}
/// Every long-running implementation must check both cancellation and deadline repeatedly.
#[derive(Debug, Clone)]
pub struct OperationContext {
    pub cancellation: CancellationToken,
    pub deadline: Instant,
}
impl OperationContext {
    pub fn checkpoint(&self) -> Result<()> {
        if self.cancellation.is_cancelled() {
            Err(Error::Cancelled)
        } else if Instant::now() >= self.deadline {
            Err(Error::Deadline)
        } else {
            Ok(())
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct StorageCapacity {
    pub quota_bytes: u64,
    pub used_bytes: u64,
    pub reserved_bytes: u64,
}
/// Observation from a check, not a perpetual durability promise.
#[derive(Debug, Clone)]
pub struct StorageReceipt {
    pub node: NodeId,
    pub sha256: Sha256Digest,
    pub bytes: u32,
    pub observed_at: Instant,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FragmentHealth {
    Unknown,
    Verified,
    Missing,
    Corrupt,
    Inaccessible,
}
#[derive(Debug, Clone)]
pub struct FragmentObservation {
    pub node: NodeId,
    pub fragment: FragmentDescriptor,
    pub health: FragmentHealth,
    pub observed_at: Instant,
}
#[derive(Debug, Clone)]
pub struct ReconstructionReport {
    pub object: ObjectDescriptor,
    pub written_bytes: u64,
    pub reconstructed_stripes: u64,
}
#[derive(Debug, Clone)]
pub struct FragmentVerification {
    pub fragment: FragmentDescriptor,
    pub observed_at: Instant,
}
#[derive(Debug, Clone)]
pub struct TransferReport {
    pub peer: Peer,
    pub fragment: FragmentDescriptor,
    pub transferred_bytes: u64,
}
#[derive(Debug, Clone)]
pub enum RepairMethod {
    CopyVerified { source: Peer },
    ReconstructStripe { inputs: Vec<FragmentCoordinate> },
}
#[derive(Debug, Clone)]
pub struct RepairAction {
    pub fragment: FragmentDescriptor,
    pub target: Peer,
    pub method: RepairMethod,
}
#[derive(Debug, Clone)]
pub struct RepairPlan {
    pub manifest: ValidatedManifest,
    pub actions: Vec<RepairAction>,
    pub expires_at: Instant,
}
#[derive(Debug, Clone)]
pub struct RepairReport {
    pub completed: u32,
    pub pending: u32,
    pub receipts: Vec<StorageReceipt>,
}
