//! Adversarial update sequences checked against independent full scans.

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use fileview::workspace::{WorkspaceEngine, WorkspaceIndex};

#[test]
fn mixed_updates_always_match_a_fresh_index() {
    let temp = tempfile::tempdir().unwrap();
    for folder in 0..16 {
        fs::create_dir(temp.path().join(format!("group-{folder}"))).unwrap();
    }
    let mut index = WorkspaceIndex::new(temp.path()).unwrap();
    index.rebuild(&|| false).unwrap();
    let mut seed = 0x1234_5678_u64;
    for batch in 0..24 {
        let mut changed = Vec::new();
        for operation in 0..24 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let id = (seed >> 32) % 256;
            let relative = PathBuf::from(format!("group-{}/file-{id}.rs", id % 16));
            let path = temp.path().join(&relative);
            if (seed >> 16).is_multiple_of(3) {
                match fs::remove_file(&path) {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => panic!("{error}"),
                }
            } else {
                fs::write(&path, format!("batch {batch}, operation {operation}")).unwrap();
            }
            changed.push(relative);
        }
        if batch % 6 == 0 {
            fs::write(
                temp.path().join(".ignore"),
                if batch % 12 == 0 {
                    "group-3/\n"
                } else {
                    "group-7/\n"
                },
            )
            .unwrap();
            changed.push(".ignore".into());
        }
        index.reconcile(&changed, &|| false).unwrap();
        let mut fresh = WorkspaceIndex::new(temp.path()).unwrap();
        fresh.rebuild(&|| false).unwrap();
        for query in ["", "ext:rs", "type:dir", "file-1"] {
            let incremental = index.search(query, true, usize::MAX, &|| false).unwrap();
            let rebuilt = fresh.search(query, true, usize::MAX, &|| false).unwrap();
            assert_eq!(
                serde_json::to_value(incremental).unwrap(),
                serde_json::to_value(rebuilt).unwrap(),
                "batch {batch}, query {query}"
            );
        }
    }
}

#[test]
fn burst_updates_and_concurrent_queries_converge() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("nested")).unwrap();
    let engine = WorkspaceEngine::with_cache(temp.path(), None).unwrap();
    engine.wait_ready(Duration::from_secs(10)).unwrap();
    std::thread::scope(|scope| {
        for thread in 0..4 {
            let engine = &engine;
            scope.spawn(move || {
                for query in 0..100 {
                    engine.request_search(&format!("missing-{thread}-{query}"), false, 15);
                }
            });
        }
    });
    let latest = engine.request_search("ext:rs", false, 1000);
    for file in 0..500 {
        fs::write(temp.path().join(format!("nested/file-{file}.rs")), "data").unwrap();
    }
    for file in 0..100 {
        fs::remove_file(temp.path().join(format!("nested/file-{file}.rs"))).unwrap();
    }
    let start = Instant::now();
    loop {
        if let Some(result) = engine.poll_search() {
            assert_eq!(result.request_id, latest);
            assert!(result.error.is_none());
            if result.matches.len() == 400 {
                assert!(result.matches.iter().all(|entry| entry.path.is_file()));
                break;
            }
        }
        assert!(
            start.elapsed() < Duration::from_secs(15),
            "burst did not converge: {:?}",
            engine.status()
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}
