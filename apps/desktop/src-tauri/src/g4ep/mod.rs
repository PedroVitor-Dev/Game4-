//! G4EP codecs, reconstruction manifests, opt-in Guardian storage and private TLS transport.
//! Networking starts only in an explicitly enabled, enrolled private session.
//!
//! Types are internal Rust contracts, not a published wire format or an IPC capability.
//! See docs/g4ep/architecture.md for ownership, privacy, adapter and acceptance rules.
pub mod encoder;
pub mod guardian;
pub mod local;
pub mod manifest;
pub mod network;
pub mod ports;
pub mod types;
pub use types::{Error, Result};
#[cfg(test)]
mod tests;

#[cfg(test)]
mod encoder_tests;

#[cfg(test)]
mod manifest_tests;
