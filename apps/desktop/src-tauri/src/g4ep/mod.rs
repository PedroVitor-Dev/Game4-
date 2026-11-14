//! G4EP architecture boundary, step 1. No runtime, codec or network adapter is installed.
//!
//! Types are internal Rust contracts, not a published wire format or an IPC capability.
//! See docs/g4ep/architecture.md for ownership, privacy, adapter and acceptance rules.
pub mod ports;
pub mod types;
pub use types::{Error, Result};
#[cfg(test)]
mod tests;
