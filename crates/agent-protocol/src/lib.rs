//! Agent Protocol - Open specification for Agent communication
//!
//! This crate defines the core types, interfaces, and error taxonomy for the Agent Protocol.
//! It serves as the contract between Agent Runtimes and governance implementations.
//!
//! # License
//! MIT - See LICENSE-MIT file for details

pub mod errors;
pub mod interfaces;
pub mod types;

// Re-export commonly used types
pub use types::*;
// pub use errors::ProtocolError;
// pub use interfaces::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_compiles() {
        // This test ensures the crate compiles
        assert!(true);
    }
}
