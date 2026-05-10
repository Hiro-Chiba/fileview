//! Context budget tracking for AI workflows.
//!
//! Keeps a running token estimate for the user's marked file set, plus a
//! background worker that computes per-file token counts off the UI thread.
//!
//! The estimate uses `cl100k_base` via `tiktoken_rs`, which is accurate for
//! GPT family models and a reasonable approximation for Claude (typically
//! within 5 to 10 percent). The status bar surface labels the result as
//! `estimate` to avoid implying byte-exact precision.

use std::path::PathBuf;
use std::str::FromStr;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use crate::mcp::token::estimate_file_tokens;

/// Models the budget bar can target.
///
/// The variant order is the cycle order driven by `Alt+M`.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BudgetModel {
    #[default]
    Sonnet46_200k,
    Sonnet46_1M,
    Opus47_200k,
    Haiku45_200k,
    Gpt4o_128k,
}

impl BudgetModel {
    /// Maximum context window in tokens.
    pub fn window_tokens(&self) -> usize {
        match self {
            Self::Sonnet46_200k => 200_000,
            Self::Sonnet46_1M => 1_000_000,
            Self::Opus47_200k => 200_000,
            Self::Haiku45_200k => 200_000,
            Self::Gpt4o_128k => 128_000,
        }
    }

    /// Long display label, used in non-narrow status bars.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Sonnet46_200k => "Sonnet 4.6",
            Self::Sonnet46_1M => "Sonnet 4.6 1M",
            Self::Opus47_200k => "Opus 4.7",
            Self::Haiku45_200k => "Haiku 4.5",
            Self::Gpt4o_128k => "GPT-4o",
        }
    }

    /// Compact label, used for Narrow and Ultra status bars.
    pub fn short_label(&self) -> &'static str {
        match self {
            Self::Sonnet46_200k => "S4.6",
            Self::Sonnet46_1M => "S4.6/1M",
            Self::Opus47_200k => "O4.7",
            Self::Haiku45_200k => "H4.5",
            Self::Gpt4o_128k => "4o",
        }
    }

    /// Stable string used when persisting to config.
    pub fn as_config_str(&self) -> &'static str {
        match self {
            Self::Sonnet46_200k => "sonnet-4-6-200k",
            Self::Sonnet46_1M => "sonnet-4-6-1m",
            Self::Opus47_200k => "opus-4-7-200k",
            Self::Haiku45_200k => "haiku-4-5-200k",
            Self::Gpt4o_128k => "gpt-4o-128k",
        }
    }

    /// Cycle to the next model, wrapping after the last variant.
    pub fn next(&self) -> Self {
        match self {
            Self::Sonnet46_200k => Self::Sonnet46_1M,
            Self::Sonnet46_1M => Self::Opus47_200k,
            Self::Opus47_200k => Self::Haiku45_200k,
            Self::Haiku45_200k => Self::Gpt4o_128k,
            Self::Gpt4o_128k => Self::Sonnet46_200k,
        }
    }
}

impl FromStr for BudgetModel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "sonnet-4-6-200k" | "sonnet-4-6" | "sonnet" => Ok(Self::Sonnet46_200k),
            "sonnet-4-6-1m" | "sonnet-1m" => Ok(Self::Sonnet46_1M),
            "opus-4-7-200k" | "opus-4-7" | "opus" => Ok(Self::Opus47_200k),
            "haiku-4-5-200k" | "haiku-4-5" | "haiku" => Ok(Self::Haiku45_200k),
            "gpt-4o-128k" | "gpt-4o" | "4o" => Ok(Self::Gpt4o_128k),
            other => Err(format!("unknown budget model: {}", other)),
        }
    }
}

/// One result message from the worker thread.
#[derive(Debug)]
pub struct BudgetResult {
    pub path: PathBuf,
    pub tokens: Result<usize, String>,
}

enum BudgetMessage {
    Estimate(PathBuf),
    Shutdown,
}

/// Background worker that computes token counts for paths.
///
/// One worker is created per `AppState` and lives for the duration of the
/// session. It owns a single dedicated thread; tiktoken's BPE table is
/// initialized once on the worker thread, so the UI thread never pays the
/// 30 to 80 ms first-call cost.
pub struct BudgetWorker {
    tx: Sender<BudgetMessage>,
    rx: Receiver<BudgetResult>,
    handle: Option<JoinHandle<()>>,
}

impl BudgetWorker {
    /// Spawn the worker thread.
    pub fn spawn() -> Self {
        let (req_tx, req_rx) = mpsc::channel::<BudgetMessage>();
        let (res_tx, res_rx) = mpsc::channel::<BudgetResult>();

        let handle = thread::Builder::new()
            .name("fv-budget-worker".into())
            .spawn(move || {
                while let Ok(msg) = req_rx.recv() {
                    match msg {
                        BudgetMessage::Shutdown => break,
                        BudgetMessage::Estimate(path) => {
                            let tokens = estimate_file_tokens(&path).map_err(|e| e.to_string());
                            if res_tx.send(BudgetResult { path, tokens }).is_err() {
                                break;
                            }
                        }
                    }
                }
            })
            .expect("spawn fv-budget-worker thread");

        Self {
            tx: req_tx,
            rx: res_rx,
            handle: Some(handle),
        }
    }

    /// Queue a path for token estimation. The result will arrive on the
    /// channel returned by `try_recv`. Returns false when the worker is gone.
    pub fn enqueue(&self, path: PathBuf) -> bool {
        self.tx.send(BudgetMessage::Estimate(path)).is_ok()
    }

    /// Drain all completed results without blocking.
    pub fn drain(&self) -> Vec<BudgetResult> {
        let mut out = Vec::new();
        while let Ok(r) = self.rx.try_recv() {
            out.push(r);
        }
        out
    }
}

impl Drop for BudgetWorker {
    fn drop(&mut self) {
        let _ = self.tx.send(BudgetMessage::Shutdown);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Format a token count with a `k` suffix for compact rendering.
///
/// Examples: `0` -> `"0"`, `512` -> `"512"`, `1234` -> `"1.2k"`,
/// `42_000` -> `"42k"`, `2_500_000` -> `"2.5M"`.
pub fn humanize_tokens(n: usize) -> String {
    if n < 1_000 {
        return n.to_string();
    }
    if n < 10_000 {
        return format!("{:.1}k", n as f32 / 1_000.0);
    }
    if n < 1_000_000 {
        return format!("{}k", n / 1_000);
    }
    if n < 10_000_000 {
        return format!("{:.1}M", n as f32 / 1_000_000.0);
    }
    format!("{}M", n / 1_000_000)
}

/// Severity bucket for color decisions in the status bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetSeverity {
    Ok,
    Warn,
    Hot,
    Over,
}

impl BudgetSeverity {
    /// Classify (used, window) into a bucket.
    pub fn from_usage(used: usize, window: usize) -> Self {
        if window == 0 || used == 0 {
            return Self::Ok;
        }
        if used > window {
            return Self::Over;
        }
        let ratio = used as f64 / window as f64;
        if ratio < 0.5 {
            Self::Ok
        } else if ratio < 0.8 {
            Self::Warn
        } else {
            Self::Hot
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_visits_every_variant_once() {
        let mut seen = Vec::new();
        let mut cur = BudgetModel::default();
        for _ in 0..5 {
            seen.push(cur);
            cur = cur.next();
        }
        assert_eq!(seen.len(), 5);
        assert_eq!(cur, BudgetModel::default(), "cycle should wrap");
        for v in [
            BudgetModel::Sonnet46_200k,
            BudgetModel::Sonnet46_1M,
            BudgetModel::Opus47_200k,
            BudgetModel::Haiku45_200k,
            BudgetModel::Gpt4o_128k,
        ] {
            assert!(seen.contains(&v), "cycle missed {:?}", v);
        }
    }

    #[test]
    fn config_string_roundtrip() {
        for v in [
            BudgetModel::Sonnet46_200k,
            BudgetModel::Sonnet46_1M,
            BudgetModel::Opus47_200k,
            BudgetModel::Haiku45_200k,
            BudgetModel::Gpt4o_128k,
        ] {
            let s = v.as_config_str();
            let parsed: BudgetModel = s.parse().unwrap();
            assert_eq!(parsed, v, "roundtrip failed for {}", s);
        }
    }

    #[test]
    fn from_str_accepts_friendly_aliases() {
        assert_eq!(
            "sonnet".parse::<BudgetModel>().unwrap(),
            BudgetModel::Sonnet46_200k
        );
        assert_eq!(
            "OPUS".parse::<BudgetModel>().unwrap(),
            BudgetModel::Opus47_200k
        );
        assert_eq!(
            "Sonnet-1M".parse::<BudgetModel>().unwrap(),
            BudgetModel::Sonnet46_1M
        );
        assert!("gpt-3".parse::<BudgetModel>().is_err());
    }

    #[test]
    fn humanize_tokens_examples() {
        assert_eq!(humanize_tokens(0), "0");
        assert_eq!(humanize_tokens(512), "512");
        assert_eq!(humanize_tokens(1234), "1.2k");
        assert_eq!(humanize_tokens(42_000), "42k");
        assert_eq!(humanize_tokens(199_999), "199k");
        assert_eq!(humanize_tokens(2_500_000), "2.5M");
        assert_eq!(humanize_tokens(15_000_000), "15M");
    }

    #[test]
    fn severity_buckets_match_thresholds() {
        let w = 200_000;
        assert_eq!(BudgetSeverity::from_usage(0, w), BudgetSeverity::Ok);
        assert_eq!(BudgetSeverity::from_usage(50_000, w), BudgetSeverity::Ok);
        assert_eq!(BudgetSeverity::from_usage(120_000, w), BudgetSeverity::Warn);
        assert_eq!(BudgetSeverity::from_usage(180_000, w), BudgetSeverity::Hot);
        assert_eq!(BudgetSeverity::from_usage(250_000, w), BudgetSeverity::Over);
    }

    #[test]
    fn worker_round_trips_a_known_file() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hello.txt");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "Hello, world!").unwrap();

        let worker = BudgetWorker::spawn();
        assert!(worker.enqueue(path.clone()));

        let mut got = None;
        for _ in 0..50 {
            let drained = worker.drain();
            if let Some(r) = drained.into_iter().next() {
                got = Some(r);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let r = got.expect("worker must produce a result");
        assert_eq!(r.path, path);
        assert!(r.tokens.unwrap() > 0);
    }
}
