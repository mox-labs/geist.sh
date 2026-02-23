//! Compositors orchestrate processors through the processing lifecycle.
//!
//! A compositor takes a set of processors and runs them through the
//! ext_proc phase model (request headers → request body → response headers → response body).
//!
//! # Available Compositors
//!
//! - [`Sequence`] — Linear pipeline. Runs processors in order, short-circuits on
//!   `ImmediateResponse` or error.

mod sequence;

pub use sequence::{FailureMode, Sequence, SequenceBuilder, SequenceOutcome};
