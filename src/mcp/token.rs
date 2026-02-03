//! Token estimation for AI context optimization
//!
//! Provides token counting and estimation for optimizing context sent to AI models.

use std::path::Path;
use std::sync::OnceLock;

use tiktoken_rs::{cl100k_base, CoreBPE};

use crate::error::{FileviewError, Result};

/// Lazy-initialized tokenizer (cl100k_base - used by GPT-4 and Claude)
static TOKENIZER: OnceLock<CoreBPE> = OnceLock::new();

/// Get the shared tokenizer instance
fn get_tokenizer() -> &'static CoreBPE {
    TOKENIZER.get_or_init(|| cl100k_base().expect("Failed to initialize tokenizer"))
}

/// Estimate the number of tokens in a string.
///
/// Uses cl100k_base encoding which is compatible with GPT-4 and provides
/// a reasonable approximation for Claude models.
pub fn estimate_tokens(text: &str) -> usize {
    let bpe = get_tokenizer();
    bpe.encode_with_special_tokens(text).len()
}

/// Estimate tokens for a file by reading its content.
pub fn estimate_file_tokens(path: &Path) -> Result<usize> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        FileviewError::token(format!("failed to read file {}: {}", path.display(), e))
    })?;
    Ok(estimate_tokens(&content))
}

/// Estimate tokens for multiple files.
///
/// Returns a vector of (path, token_count) pairs and the total.
pub fn estimate_files_tokens(paths: &[&Path]) -> Result<(Vec<(String, usize)>, usize)> {
    let mut results = Vec::with_capacity(paths.len());
    let mut total = 0;

    for path in paths {
        match estimate_file_tokens(path) {
            Ok(count) => {
                results.push((path.display().to_string(), count));
                total += count;
            }
            Err(e) => {
                // Include error info but continue with other files
                results.push((format!("{} (error: {})", path.display(), e), 0));
            }
        }
    }

    Ok((results, total))
}

/// Token budget configuration for context generation.
#[derive(Debug, Clone)]
pub struct TokenBudget {
    /// Maximum total tokens
    pub max_tokens: usize,
    /// Reserved tokens for the main file
    pub main_file_reserve: usize,
    /// Reserved tokens for imports/dependencies
    pub imports_reserve: usize,
    /// Reserved tokens for related tests
    pub tests_reserve: usize,
    /// Reserved tokens for other context
    pub other_reserve: usize,
}

impl Default for TokenBudget {
    fn default() -> Self {
        Self {
            max_tokens: 8000,
            main_file_reserve: 4000,
            imports_reserve: 2000,
            tests_reserve: 1000,
            other_reserve: 1000,
        }
    }
}

impl TokenBudget {
    /// Create a new token budget with custom max tokens.
    pub fn new(max_tokens: usize) -> Self {
        // Distribute tokens proportionally
        let main_file_reserve = max_tokens / 2;
        let imports_reserve = max_tokens / 4;
        let tests_reserve = max_tokens / 8;
        let other_reserve = max_tokens - main_file_reserve - imports_reserve - tests_reserve;

        Self {
            max_tokens,
            main_file_reserve,
            imports_reserve,
            tests_reserve,
            other_reserve,
        }
    }

    /// Check if content fits within the budget.
    pub fn fits(&self, current: usize) -> bool {
        current <= self.max_tokens
    }

    /// Get remaining tokens.
    pub fn remaining(&self, used: usize) -> usize {
        self.max_tokens.saturating_sub(used)
    }
}

/// Truncate content to fit within a token limit.
///
/// Truncates at line boundaries when possible to keep code readable.
pub fn truncate_to_tokens(content: &str, max_tokens: usize) -> String {
    let current_tokens = estimate_tokens(content);
    if current_tokens <= max_tokens {
        return content.to_string();
    }

    // Binary search for the right length
    let lines: Vec<&str> = content.lines().collect();
    let mut low = 0;
    let mut high = lines.len();

    while low < high {
        let mid = (low + high).div_ceil(2);
        let test_content = lines[..mid].join("\n");
        if estimate_tokens(&test_content) <= max_tokens {
            low = mid;
        } else {
            high = mid - 1;
        }
    }

    if low == 0 {
        // Even first line is too long, truncate characters
        let chars: Vec<char> = content.chars().collect();
        let mut end = chars.len();
        while end > 0 && estimate_tokens(&chars[..end].iter().collect::<String>()) > max_tokens {
            end = end * 3 / 4; // Reduce by 25% each iteration
        }
        format!("{}...", chars[..end].iter().collect::<String>())
    } else {
        format!("{}\\n... (truncated)", lines[..low].join("\n"))
    }
}

/// Compress content by removing comments and extra whitespace.
///
/// This is a simple compression that works for most languages.
pub fn compress_content(content: &str) -> String {
    let mut result = Vec::new();
    let mut in_multiline_comment = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }

        // Handle multiline comments (/* ... */)
        if in_multiline_comment {
            if trimmed.contains("*/") {
                in_multiline_comment = false;
            }
            continue;
        }

        if trimmed.starts_with("/*") && !trimmed.contains("*/") {
            in_multiline_comment = true;
            continue;
        }

        // Skip single-line comments
        if trimmed.starts_with("//") || trimmed.starts_with('#') {
            continue;
        }

        // Skip doc comments but keep code
        if trimmed.starts_with("///") || trimmed.starts_with("//!") {
            continue;
        }

        // Compress multiple spaces to single space
        let compressed: String = trimmed.split_whitespace().collect::<Vec<_>>().join(" ");

        if !compressed.is_empty() {
            result.push(compressed);
        }
    }

    result.join("\n")
}

/// Format file content with path header for AI context.
pub fn format_file_context(path: &Path, content: &str) -> String {
    format!("--- {} ---\n{}\n", path.display(), content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens() {
        // Simple text should have predictable token count
        let text = "Hello, world!";
        let tokens = estimate_tokens(text);
        assert!(tokens > 0);
        assert!(tokens < 10);

        // Longer text should have more tokens
        let long_text = "Hello, world! ".repeat(100);
        let long_tokens = estimate_tokens(&long_text);
        assert!(long_tokens > tokens);
    }

    #[test]
    fn test_token_budget() {
        let budget = TokenBudget::new(10000);
        assert_eq!(budget.max_tokens, 10000);
        assert!(budget.fits(5000));
        assert!(!budget.fits(15000));
        assert_eq!(budget.remaining(3000), 7000);
    }

    #[test]
    fn test_truncate_to_tokens() {
        let content = "line1\nline2\nline3\nline4\nline5";
        let tokens = estimate_tokens(content);

        // Truncating to more than current should return same content
        let truncated = truncate_to_tokens(content, tokens + 100);
        assert_eq!(truncated, content);

        // Truncating to less should result in shorter content
        let truncated = truncate_to_tokens(content, 3);
        assert!(truncated.len() < content.len());
    }

    #[test]
    fn test_compress_content() {
        let content = r#"
// This is a comment
fn main() {
    // Another comment
    println!("Hello");
}
"#;
        let compressed = compress_content(content);
        assert!(!compressed.contains("// This is a comment"));
        assert!(compressed.contains("println"));
    }

    #[test]
    fn test_format_file_context() {
        let path = Path::new("src/main.rs");
        let content = "fn main() {}";
        let formatted = format_file_context(path, content);
        assert!(formatted.starts_with("--- src/main.rs ---"));
        assert!(formatted.contains("fn main()"));
    }
}
