use std::collections::{HashMap, HashSet};

use crate::scheduler::JobId;
use crate::NebulaError;

/// Directed acyclic graph of job dependencies.
#[derive(Default)]
pub struct TaskGraph {
    edges: HashMap<JobId, Vec<JobId>>,
    done: HashSet<JobId>,
}

impl TaskGraph {
    pub fn add(&mut self, id: JobId, deps: &[JobId]) -> Result<(), NebulaError> {
        if deps.contains(&id) {
            return Err(NebulaError::Cycle(id.0));
        }
        self.edges.insert(id, deps.to_vec());
        Ok(())
    }

    pub fn is_ready(&self, id: JobId) -> bool {
        self.edges
            .get(&id)
            .map_or(true, |deps| deps.iter().all(|d| self.done.contains(d)))
    }

    pub fn complete(&mut self, id: JobId) {
        self.done.insert(id);
    }
}
