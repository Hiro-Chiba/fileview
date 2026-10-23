use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::graph::TaskGraph;
use crate::queue::JobQueue;
use crate::NebulaError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JobId(pub u64);

#[derive(Debug, Clone)]
pub struct Job {
    pub id: JobId,
    pub name: String,
    pub priority: u8,
    pub timeout: Duration,
}

pub struct Scheduler {
    queue: JobQueue<Job>,
    graph: TaskGraph,
    running: HashMap<JobId, Instant>,
    slots: usize,
}

impl Scheduler {
    pub fn new(slots: usize) -> Self {
        Self {
            queue: JobQueue::with_capacity(1024),
            graph: TaskGraph::default(),
            running: HashMap::new(),
            slots,
        }
    }

    /// Submit a job once all of its dependencies have finished.
    pub fn submit(&mut self, job: Job, deps: &[JobId]) -> Result<(), NebulaError> {
        self.graph.add(job.id, deps)?;
        if self.graph.is_ready(job.id) {
            self.queue.push(job.priority, job);
        }
        Ok(())
    }

    /// Pop as many ready jobs as there are free slots.
    pub fn tick(&mut self) -> Vec<Job> {
        let free = self.slots.saturating_sub(self.running.len());
        let jobs: Vec<Job> = (0..free).filter_map(|_| self.queue.pop()).collect();
        for job in &jobs {
            self.running.insert(job.id, Instant::now());
        }
        jobs
    }

    // TODO: requeue jobs whose worker disappeared instead of failing them
    pub fn reap_timeouts(&mut self, limit: Duration) -> Vec<JobId> {
        let now = Instant::now();
        let expired: Vec<JobId> = self
            .running
            .iter()
            .filter(|(_, started)| now.duration_since(**started) > limit)
            .map(|(id, _)| *id)
            .collect();
        for id in &expired {
            self.running.remove(id);
        }
        expired
    }
}
