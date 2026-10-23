//! Core scheduling primitives for Nebula.

pub mod error;
pub mod graph;
pub mod queue;
pub mod scheduler;

pub use error::NebulaError;
pub use graph::TaskGraph;
pub use scheduler::{Job, JobId, Scheduler};

/// Default number of worker slots per node.
pub const DEFAULT_SLOTS: usize = 64;
