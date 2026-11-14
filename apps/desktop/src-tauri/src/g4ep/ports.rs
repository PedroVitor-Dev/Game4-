//! Object-safe ports only; no placeholder adapter claims to encode, host or transfer data.
use super::types::*;
use std::{
    future::Future,
    io::{Read, Write},
    pin::Pin,
};
use tokio::io::{AsyncRead, AsyncWrite};
pub type PortFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;
pub type FragmentReader = Box<dyn Read + Send>;
pub type AsyncReader = dyn AsyncRead + Unpin + Send;
pub type AsyncWriter = dyn AsyncWrite + Unpin + Send;

/// Stage immutable shards under host-owned temporary names. Check hash/length before commit.
pub trait FragmentSink: Send {
    fn write_fragment(
        &mut self,
        fragment: &FragmentDescriptor,
        bytes: &mut dyn Read,
        ctx: &OperationContext,
    ) -> Result<()>;
}
/// Reads remain untrusted until the consumer has checked expected length and SHA-256.
pub trait FragmentSource: Send {
    fn read_fragment(
        &mut self,
        fragment: &FragmentDescriptor,
        ctx: &OperationContext,
    ) -> Result<FragmentReader>;
}
/// Blocking streaming work; a Tauri adapter must schedule it on spawn_blocking.
pub trait Encoder: Send + Sync {
    fn codec_id(&self) -> &CodecId;
    /// Read exactly declared_size bytes, detect extra/short input and never mutate the source.
    /// Emit full padded shards one stripe at a time; publish metadata only on complete success.
    fn encode(
        &self,
        source: &mut dyn Read,
        declared_size: u64,
        coding: CodingProfile,
        output: &mut dyn FragmentSink,
        ctx: &OperationContext,
    ) -> Result<ValidatedManifest>;
}
pub trait Decoder: Send + Sync {
    fn codec_id(&self) -> &CodecId;
    /// Use distinct verified shard indexes; require k per stripe and verify the final object.
    /// Write only to a staged output. The application atomically publishes after success.
    fn reconstruct(
        &self,
        manifest: &ValidatedManifest,
        fragments: &mut dyn FragmentSource,
        output: &mut dyn Write,
        ctx: &OperationContext,
    ) -> Result<ReconstructionReport>;
}
pub trait StorageNode: Send + Sync {
    fn node_id(&self) -> NodeId;
    fn capacity(&self, ctx: &OperationContext) -> Result<StorageCapacity>;
    /// Reserve quota first; use bounded temp I/O and atomic no-clobber commit after verification.
    fn commit_fragment(
        &self,
        fragment: &FragmentDescriptor,
        bytes: &mut dyn Read,
        ctx: &OperationContext,
    ) -> Result<StorageReceipt>;
    fn read_fragment(
        &self,
        fragment: &FragmentDescriptor,
        ctx: &OperationContext,
    ) -> Result<FragmentReader>;
    /// Requested inventory only, never an unrestricted filesystem listing. Results must be bounded.
    fn inspect(
        &self,
        requested: &[FragmentDescriptor],
        ctx: &OperationContext,
    ) -> Result<Vec<FragmentObservation>>;
}
pub trait PeerDiscovery: Send + Sync {
    /// Private allowlist only. No public bootstrap, trackers, PEX, LAN broadcast or announcement.
    fn discover<'a>(
        &'a self,
        network: &'a PrivateNetwork,
        ctx: &'a OperationContext,
    ) -> PortFuture<'a, Vec<Peer>>;
}
pub trait FragmentTransfer: Send + Sync {
    /// Verify peer enrollment/authentication, limits and length/hash; remote receipt is a claim.
    fn upload<'a>(
        &'a self,
        network: &'a PrivateNetwork,
        peer: &'a Peer,
        fragment: &'a FragmentDescriptor,
        source: &'a mut AsyncReader,
        ctx: &'a OperationContext,
    ) -> PortFuture<'a, StorageReceipt>;
    /// Use a host-owned temporary sink; success requires locally checked SHA-256 and exact size.
    fn download<'a>(
        &'a self,
        network: &'a PrivateNetwork,
        peer: &'a Peer,
        fragment: &'a FragmentDescriptor,
        output: &'a mut AsyncWriter,
        ctx: &'a OperationContext,
    ) -> PortFuture<'a, TransferReport>;
}
pub trait IntegrityVerification: Send + Sync {
    /// Stream SHA-256 with bounded buffers, exact length, cancellation and deadline checks.
    fn verify_fragment(
        &self,
        expected: &FragmentDescriptor,
        bytes: &mut dyn Read,
        ctx: &OperationContext,
    ) -> Result<FragmentVerification>;
    fn verify_object(
        &self,
        expected: &ObjectDescriptor,
        bytes: &mut dyn Read,
        ctx: &OperationContext,
    ) -> Result<ObjectDescriptor>;
}
pub trait RepairManager: Send + Sync {
    /// Deduplicate by coordinate and failure domain, exclude stale/corrupt claims and honor quotas.
    /// An offline host alone is not proof of corruption; an unrecoverable stripe must be explicit.
    fn plan(
        &self,
        manifest: &ValidatedManifest,
        network: &PrivateNetwork,
        observations: &[FragmentObservation],
        ctx: &OperationContext,
    ) -> Result<RepairPlan>;
    /// Revalidate expiring plans/allowlist and verify reconstructed bytes before new placement.
    /// A later service may schedule this only inside an explicitly enabled private session.
    fn execute<'a>(
        &'a self,
        plan: &'a RepairPlan,
        network: &'a PrivateNetwork,
        ctx: &'a OperationContext,
    ) -> PortFuture<'a, RepairReport>;
}
