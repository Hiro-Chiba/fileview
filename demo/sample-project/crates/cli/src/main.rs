use std::time::Duration;

use nebula_core::{Job, JobId, Scheduler, DEFAULT_SLOTS};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut scheduler = Scheduler::new(DEFAULT_SLOTS);

    let build = Job { id: JobId(1), name: "build".into(), priority: 9, timeout: Duration::from_secs(600) };
    let test = Job { id: JobId(2), name: "test".into(), priority: 5, timeout: Duration::from_secs(900) };
    scheduler.submit(build, &[])?;
    scheduler.submit(test, &[JobId(1)])?;

    loop {
        for job in scheduler.tick() {
            println!("dispatch {} (priority {})", job.name, job.priority);
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}
