use std::time::Duration;

use nebula_core::{Job, JobId, Scheduler};

fn job(id: u64, priority: u8) -> Job {
    Job { id: JobId(id), name: format!("job-{id}"), priority, timeout: Duration::from_secs(1) }
}

#[test]
fn higher_priority_jobs_run_first() {
    let mut s = Scheduler::new(1);
    s.submit(job(1, 1), &[]).unwrap();
    s.submit(job(2, 9), &[]).unwrap();
    assert_eq!(s.tick()[0].id, JobId(2));
}

#[test]
fn a_job_cannot_depend_on_itself() {
    let mut s = Scheduler::new(4);
    assert!(s.submit(job(7, 1), &[JobId(7)]).is_err());
}
