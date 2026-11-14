//! G4EP ports and offline RS encoder. No network adapter or background runtime is installed.
//!
//! Types are internal Rust contracts, not a published wire format or an IPC capability.
//! See docs/g4ep/architecture.md for ownership, privacy, adapter and acceptance rules.
pub mod encoder;
pub mod local;
pub mod manifest;
pub mod ports;
pub mod types;
pub use types::{Error, Result};
#[cfg(test)]
mod tests;

#[cfg(test)]
mod encoder_tests;

#[cfg(test)]
mod manifest_tests;
