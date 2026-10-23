use std::fmt;

#[derive(Debug)]
pub enum NebulaError {
    Cycle(u64),
    Storage(String),
    Timeout { job: u64, secs: u64 },
}

impl fmt::Display for NebulaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cycle(id) => write!(f, "job {id} depends on itself"),
            Self::Storage(msg) => write!(f, "storage error: {msg}"),
            Self::Timeout { job, secs } => write!(f, "job {job} timed out after {secs}s"),
        }
    }
}

impl std::error::Error for NebulaError {}
