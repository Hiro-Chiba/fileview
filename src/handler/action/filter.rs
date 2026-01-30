//! Filter action handlers
//!
//! Handles file filter operations

use crate::core::{AppState, ViewMode};
use crate::handler::key::KeyAction;

/// Handle filter-related actions
pub fn handle(action: KeyAction, state: &mut AppState) {
    match action {
        KeyAction::StartFilter => {
            state.mode = ViewMode::Filter {
                query: state.filter_pattern.clone().unwrap_or_default(),
            };
        }
        KeyAction::ApplyFilter { pattern } => {
            if pattern.is_empty() {
                state.filter_pattern = None;
                state.set_message("Filter cleared");
            } else {
                state.filter_pattern = Some(pattern.clone());
                state.set_message(format!("Filter: {}", pattern));
            }
            state.mode = ViewMode::Browse;
            // Reset focus to top since visible entries may change
            state.focus_index = 0;
        }
        KeyAction::ClearFilter => {
            state.filter_pattern = None;
            state.set_message("Filter cleared");
            state.mode = ViewMode::Browse;
        }
        _ => {}
    }
}

/// Check if a filename matches the filter pattern
/// Supports simple glob patterns: * (any chars), ? (single char)
pub fn matches_filter(filename: &str, pattern: &str) -> bool {
    glob_match(pattern, filename)
}

/// Simple glob matching implementation
fn glob_match(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let text: Vec<char> = text.chars().collect();
    glob_match_impl(&pattern, &text)
}

fn glob_match_impl(pattern: &[char], text: &[char]) -> bool {
    let mut p_idx = 0;
    let mut t_idx = 0;
    let mut star_idx: Option<usize> = None;
    let mut match_idx = 0;

    while t_idx < text.len() {
        if p_idx < pattern.len() && (pattern[p_idx] == '?' || pattern[p_idx] == text[t_idx]) {
            // Characters match or pattern has ?
            p_idx += 1;
            t_idx += 1;
        } else if p_idx < pattern.len() && pattern[p_idx] == '*' {
            // Star found, remember position
            star_idx = Some(p_idx);
            match_idx = t_idx;
            p_idx += 1;
        } else if let Some(star) = star_idx {
            // Mismatch after star, backtrack
            p_idx = star + 1;
            match_idx += 1;
            t_idx = match_idx;
        } else {
            // No match
            return false;
        }
    }

    // Check remaining pattern characters (should all be stars)
    while p_idx < pattern.len() && pattern[p_idx] == '*' {
        p_idx += 1;
    }

    p_idx == pattern.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        assert!(matches_filter("test.rs", "test.rs"));
        assert!(!matches_filter("test.rs", "test.txt"));
    }

    #[test]
    fn test_star_wildcard() {
        assert!(matches_filter("test.rs", "*.rs"));
        assert!(matches_filter("main.rs", "*.rs"));
        assert!(!matches_filter("test.txt", "*.rs"));
        assert!(matches_filter("test.rs", "test*"));
        assert!(matches_filter("test_foo.rs", "test*"));
        assert!(matches_filter("test.rs", "*"));
    }

    #[test]
    fn test_question_wildcard() {
        assert!(matches_filter("test.rs", "test.?s"));
        assert!(matches_filter("test.ts", "test.?s"));
        assert!(!matches_filter("test.css", "test.?s"));
    }

    #[test]
    fn test_combined_wildcards() {
        assert!(matches_filter("test_foo.rs", "*_*.rs"));
        assert!(matches_filter("a_b.rs", "*_*.rs"));
        assert!(!matches_filter("test.rs", "*_*.rs"));
    }
}
