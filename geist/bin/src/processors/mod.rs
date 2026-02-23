//! Processors for the geist binary.
//!
//! These live in the composition root (bin crate) because they bridge
//! domain crates without coupling them: geist-edge (processing model)
//! + geist-policy (policy evaluation).

pub mod access_control;
