//! One bounded worker for streaming content results in the existing picker.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};

use super::content::{self, ContentHit};

struct Request {
    id: u64,
    root: PathBuf,
    query: String,
    hidden: bool,
}
#[derive(Default)]
struct Mailbox {
    request: Option<Request>,
    response: Option<ContentResponse>,
}
struct Shared {
    mailbox: Mutex<Mailbox>,
    wake: Condvar,
    generation: AtomicU64,
    stopped: AtomicBool,
}
/// A bounded snapshot. Intermediate snapshots may be replaced by newer ones.
pub struct ContentResponse {
    pub id: u64,
    pub hits: Vec<ContentHit>,
    pub done: bool,
    pub error: Option<String>,
}
/// Latest-query-wins content search. Dropping it cancels its managed process.
pub struct ContentSearch {
    shared: Arc<Shared>,
}
impl Default for ContentSearch {
    fn default() -> Self {
        Self::new()
    }
}
impl ContentSearch {
    pub fn new() -> Self {
        let shared = Arc::new(Shared {
            mailbox: Mutex::new(Mailbox::default()),
            wake: Condvar::new(),
            generation: AtomicU64::new(0),
            stopped: AtomicBool::new(false),
        });
        let worker = Arc::clone(&shared);
        std::thread::spawn(move || loop {
            let request = {
                let mut mailbox = worker.mailbox.lock().unwrap_or_else(|e| e.into_inner());
                while mailbox.request.is_none() && !worker.stopped.load(Ordering::Acquire) {
                    mailbox = worker.wake.wait(mailbox).unwrap_or_else(|e| e.into_inner());
                }
                if worker.stopped.load(Ordering::Acquire) {
                    break;
                }
                mailbox.request.take().unwrap()
            };
            let cancelled = || {
                worker.stopped.load(Ordering::Acquire)
                    || worker.generation.load(Ordering::Acquire) != request.id
            };
            if cancelled() {
                continue;
            }
            let mut hits = Vec::new();
            let publish = |hits: &Vec<ContentHit>, done, error| {
                let mut mailbox = worker.mailbox.lock().unwrap_or_else(|e| e.into_inner());
                if !cancelled() {
                    mailbox.response = Some(ContentResponse {
                        id: request.id,
                        hits: hits.clone(),
                        done,
                        error,
                    });
                }
            };
            let result = content::search(
                &request.root,
                &request.query,
                request.hidden,
                15,
                &cancelled,
                &mut |hit| {
                    hits.push(hit);
                    publish(&hits, false, None);
                },
            );
            let error = match result {
                Ok(summary) if summary.changed_results > 0 => Some(
                    "Some files changed during search; search again for current results".to_owned(),
                ),
                Ok(_) => None,
                Err(error) => Some(error.to_string()),
            };
            publish(&hits, true, error);
        });
        Self { shared }
    }
    pub fn request(&self, root: PathBuf, query: String, hidden: bool) -> u64 {
        let mut mailbox = self
            .shared
            .mailbox
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let id = self.shared.generation.fetch_add(1, Ordering::AcqRel) + 1;
        mailbox.response = None;
        mailbox.request = Some(Request {
            id,
            root,
            query,
            hidden,
        });
        self.shared.wake.notify_one();
        id
    }
    pub fn cancel(&self) {
        let mut mailbox = self
            .shared
            .mailbox
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        self.shared.generation.fetch_add(1, Ordering::AcqRel);
        mailbox.request = None;
        mailbox.response = None;
    }
    pub fn poll(&self) -> Option<ContentResponse> {
        self.shared
            .mailbox
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .response
            .take()
    }
}
impl Drop for ContentSearch {
    fn drop(&mut self) {
        self.shared.stopped.store(true, Ordering::Release);
        self.cancel();
        self.shared.wake.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn superseded_queries_and_cancelled_results_cannot_reappear() {
        if std::process::Command::new("rg")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("file.txt"), "alpha\nbeta\n").unwrap();
        let service = ContentSearch::new();
        for _ in 0..50 {
            service.request(root.path().to_owned(), "alpha".to_owned(), false);
        }
        let latest = service.request(root.path().to_owned(), "beta".to_owned(), false);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            assert!(
                std::time::Instant::now() < deadline,
                "content worker did not finish"
            );
            if let Some(response) = service.poll() {
                assert_eq!(response.id, latest);
                assert!(response.error.is_none(), "{:?}", response.error);
                if response.done {
                    assert_eq!(response.hits.len(), 1);
                    assert_eq!(response.hits[0].line_number, 2);
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        service.request(root.path().to_owned(), "alpha".to_owned(), false);
        service.cancel();
        std::thread::sleep(std::time::Duration::from_millis(30));
        assert!(service.poll().is_none());
    }
}
