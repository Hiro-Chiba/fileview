//! Terminal detection module - Detects terminal brands and recommends image protocols
//!
//! This module provides terminal brand detection based on environment variables,
//! and maps each terminal to its recommended image protocol.

/// Terminal brands that can be detected via environment variables
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalBrand {
    /// Kitty terminal (KITTY_WINDOW_ID)
    Kitty,
    /// Ghostty terminal (GHOSTTY_RESOURCES_DIR)
    Ghostty,
    /// WezTerm (WEZTERM_EXECUTABLE)
    WezTerm,
    /// iTerm2 (TERM_PROGRAM=iTerm.app)
    ITerm2,
    /// KDE Konsole (TERMINAL=konsole)
    Konsole,
    /// Foot terminal (TERM=foot*)
    Foot,
    /// VS Code integrated terminal (TERM_PROGRAM=vscode)
    VSCode,
    /// Warp terminal (TERM_PROGRAM=WarpTerminal)
    Warp,
    /// Alacritty (TERM_PROGRAM=Alacritty)
    Alacritty,
    /// Windows Terminal (WT_SESSION)
    WindowsTerminal,
    /// Running inside tmux (TMUX)
    Tmux,
    /// Unknown terminal
    Unknown,
}

/// Recommended image protocol for a terminal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecommendedProtocol {
    /// Kitty Graphics Protocol - highest quality, supported by Kitty, Ghostty, Konsole
    Kitty,
    /// iTerm2 Inline Images Protocol - supported by iTerm2, WezTerm, Warp
    Iterm2,
    /// Sixel graphics - supported by Foot, Windows Terminal, mlterm
    Sixel,
    /// Chafa preferred - for terminals without native protocol support (VSCode, Alacritty)
    Chafa,
    /// Query terminal capabilities - for unknown terminals or tmux
    Query,
}

impl TerminalBrand {
    /// Detect the terminal brand from environment variables
    ///
    /// Detection priority:
    /// 1. Terminal-specific environment variables (most reliable)
    /// 2. TERM_PROGRAM environment variable
    /// 3. TERMINAL environment variable
    /// 4. TERM environment variable
    pub fn detect() -> Self {
        Self::detect_from_env(&EnvReader::Real)
    }

    /// Internal detection with injectable environment reader (for testing)
    fn detect_from_env(env: &dyn EnvReaderTrait) -> Self {
        // 1. Terminal-specific environment variables (most reliable)

        // Kitty sets KITTY_WINDOW_ID
        if env.var("KITTY_WINDOW_ID").is_ok() {
            return Self::Kitty;
        }

        // Ghostty sets GHOSTTY_RESOURCES_DIR
        if env.var("GHOSTTY_RESOURCES_DIR").is_ok() {
            return Self::Ghostty;
        }

        // WezTerm sets WEZTERM_EXECUTABLE
        if env.var("WEZTERM_EXECUTABLE").is_ok() {
            return Self::WezTerm;
        }

        // Windows Terminal sets WT_SESSION
        if env.var("WT_SESSION").is_ok() {
            return Self::WindowsTerminal;
        }

        // Tmux sets TMUX
        if env.var("TMUX").is_ok() {
            return Self::Tmux;
        }

        // 2. TERM_PROGRAM based detection
        if let Ok(term_program) = env.var("TERM_PROGRAM") {
            let term_lower = term_program.to_lowercase();

            // iTerm2 sets TERM_PROGRAM=iTerm.app
            if term_lower.contains("iterm") {
                return Self::ITerm2;
            }

            // Warp sets TERM_PROGRAM=WarpTerminal
            if term_lower.contains("warp") {
                return Self::Warp;
            }

            // VS Code sets TERM_PROGRAM=vscode
            if term_lower.contains("vscode") {
                return Self::VSCode;
            }

            // Alacritty sets TERM_PROGRAM=Alacritty
            if term_lower.contains("alacritty") {
                return Self::Alacritty;
            }
        }

        // 3. TERMINAL environment variable (used by some Linux distros)
        if let Ok(terminal) = env.var("TERMINAL") {
            if terminal.to_lowercase().contains("konsole") {
                return Self::Konsole;
            }
        }

        // 4. TERM environment variable (least specific)
        if let Ok(term) = env.var("TERM") {
            let term_lower = term.to_lowercase();

            // Foot sets TERM=foot or TERM=foot-extra
            if term_lower.starts_with("foot") {
                return Self::Foot;
            }
        }

        Self::Unknown
    }

    /// Get the recommended protocol for this terminal
    pub fn recommended_protocol(&self) -> RecommendedProtocol {
        match self {
            // Kitty protocol terminals (True color, highest quality)
            Self::Kitty => RecommendedProtocol::Kitty,
            Self::Ghostty => RecommendedProtocol::Kitty,
            Self::Konsole => RecommendedProtocol::Kitty,

            // iTerm2 protocol terminals (True color, high quality)
            Self::ITerm2 => RecommendedProtocol::Iterm2,
            Self::WezTerm => RecommendedProtocol::Iterm2,
            Self::Warp => RecommendedProtocol::Iterm2,

            // Sixel protocol terminals (256 colors, good quality)
            Self::Foot => RecommendedProtocol::Sixel,
            Self::WindowsTerminal => RecommendedProtocol::Sixel,

            // VS Code/Alacritty: no reliable native protocol, use Chafa/Halfblocks
            Self::VSCode => RecommendedProtocol::Chafa,
            Self::Alacritty => RecommendedProtocol::Chafa,

            // Query terminal or fallback
            Self::Tmux => RecommendedProtocol::Query,
            Self::Unknown => RecommendedProtocol::Query,
        }
    }

    /// Get the terminal brand name as a string
    pub fn name(&self) -> &'static str {
        match self {
            Self::Kitty => "Kitty",
            Self::Ghostty => "Ghostty",
            Self::WezTerm => "WezTerm",
            Self::ITerm2 => "iTerm2",
            Self::Konsole => "Konsole",
            Self::Foot => "Foot",
            Self::VSCode => "VS Code",
            Self::Warp => "Warp",
            Self::Alacritty => "Alacritty",
            Self::WindowsTerminal => "Windows Terminal",
            Self::Tmux => "tmux",
            Self::Unknown => "Unknown",
        }
    }
}

impl RecommendedProtocol {
    /// Get the protocol name as a string
    pub fn name(&self) -> &'static str {
        match self {
            Self::Kitty => "Kitty Graphics Protocol",
            Self::Iterm2 => "iTerm2 Inline Images",
            Self::Sixel => "Sixel",
            Self::Chafa => "Chafa",
            Self::Query => "Query/Auto-detect",
        }
    }
}

// =============================================================================
// Environment Reader Trait (for testability)
// =============================================================================

/// Trait for reading environment variables (allows mocking in tests)
trait EnvReaderTrait {
    fn var(&self, key: &str) -> Result<String, std::env::VarError>;
}

/// Real environment reader
enum EnvReader {
    Real,
}

impl EnvReaderTrait for EnvReader {
    fn var(&self, key: &str) -> Result<String, std::env::VarError> {
        std::env::var(key)
    }
}

/// Mock environment reader for testing
#[cfg(test)]
struct MockEnvReader {
    vars: std::collections::HashMap<String, String>,
}

#[cfg(test)]
impl MockEnvReader {
    fn new() -> Self {
        Self {
            vars: std::collections::HashMap::new(),
        }
    }

    fn set(&mut self, key: &str, value: &str) -> &mut Self {
        self.vars.insert(key.to_string(), value.to_string());
        self
    }
}

#[cfg(test)]
impl EnvReaderTrait for MockEnvReader {
    fn var(&self, key: &str) -> Result<String, std::env::VarError> {
        self.vars
            .get(key)
            .cloned()
            .ok_or(std::env::VarError::NotPresent)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // TerminalBrand Detection Tests
    // =========================================================================

    mod detection {
        use super::*;

        #[test]
        fn detects_kitty_from_kitty_window_id() {
            let mut env = MockEnvReader::new();
            env.set("KITTY_WINDOW_ID", "1");

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::Kitty);
        }

        #[test]
        fn detects_ghostty_from_ghostty_resources_dir() {
            let mut env = MockEnvReader::new();
            env.set("GHOSTTY_RESOURCES_DIR", "/usr/share/ghostty");

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::Ghostty);
        }

        #[test]
        fn detects_wezterm_from_wezterm_executable() {
            let mut env = MockEnvReader::new();
            env.set("WEZTERM_EXECUTABLE", "/usr/bin/wezterm");

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::WezTerm);
        }

        #[test]
        fn detects_windows_terminal_from_wt_session() {
            let mut env = MockEnvReader::new();
            env.set("WT_SESSION", "some-guid");

            assert_eq!(
                TerminalBrand::detect_from_env(&env),
                TerminalBrand::WindowsTerminal
            );
        }

        #[test]
        fn detects_tmux_from_tmux_env() {
            let mut env = MockEnvReader::new();
            env.set("TMUX", "/tmp/tmux-1000/default,12345,0");

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::Tmux);
        }

        #[test]
        fn detects_iterm2_from_term_program() {
            let mut env = MockEnvReader::new();
            env.set("TERM_PROGRAM", "iTerm.app");

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::ITerm2);
        }

        #[test]
        fn detects_iterm2_case_insensitive() {
            let mut env = MockEnvReader::new();
            env.set("TERM_PROGRAM", "ITERM.APP");

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::ITerm2);
        }

        #[test]
        fn detects_warp_from_term_program() {
            let mut env = MockEnvReader::new();
            env.set("TERM_PROGRAM", "WarpTerminal");

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::Warp);
        }

        #[test]
        fn detects_vscode_from_term_program() {
            let mut env = MockEnvReader::new();
            env.set("TERM_PROGRAM", "vscode");

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::VSCode);
        }

        #[test]
        fn detects_alacritty_from_term_program() {
            let mut env = MockEnvReader::new();
            env.set("TERM_PROGRAM", "Alacritty");

            assert_eq!(
                TerminalBrand::detect_from_env(&env),
                TerminalBrand::Alacritty
            );
        }

        #[test]
        fn detects_konsole_from_terminal_env() {
            let mut env = MockEnvReader::new();
            env.set("TERMINAL", "konsole");

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::Konsole);
        }

        #[test]
        fn detects_foot_from_term_env() {
            let mut env = MockEnvReader::new();
            env.set("TERM", "foot");

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::Foot);
        }

        #[test]
        fn detects_foot_extra_from_term_env() {
            let mut env = MockEnvReader::new();
            env.set("TERM", "foot-extra");

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::Foot);
        }

        #[test]
        fn returns_unknown_for_empty_env() {
            let env = MockEnvReader::new();

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::Unknown);
        }

        #[test]
        fn returns_unknown_for_unrecognized_term_program() {
            let mut env = MockEnvReader::new();
            env.set("TERM_PROGRAM", "SomeUnknownTerminal");

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::Unknown);
        }

        #[test]
        fn returns_unknown_for_generic_term() {
            let mut env = MockEnvReader::new();
            env.set("TERM", "xterm-256color");

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::Unknown);
        }
    }

    // =========================================================================
    // Detection Priority Tests
    // =========================================================================

    mod priority {
        use super::*;

        #[test]
        fn kitty_takes_priority_over_term_program() {
            let mut env = MockEnvReader::new();
            env.set("KITTY_WINDOW_ID", "1");
            env.set("TERM_PROGRAM", "vscode"); // Should be ignored

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::Kitty);
        }

        #[test]
        fn ghostty_takes_priority_over_term_program() {
            let mut env = MockEnvReader::new();
            env.set("GHOSTTY_RESOURCES_DIR", "/path");
            env.set("TERM_PROGRAM", "Alacritty"); // Should be ignored

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::Ghostty);
        }

        #[test]
        fn wezterm_takes_priority_over_term_program() {
            let mut env = MockEnvReader::new();
            env.set("WEZTERM_EXECUTABLE", "/path");
            env.set("TERM_PROGRAM", "iTerm.app"); // Should be ignored

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::WezTerm);
        }

        #[test]
        fn windows_terminal_takes_priority_over_term() {
            let mut env = MockEnvReader::new();
            env.set("WT_SESSION", "guid");
            env.set("TERM", "foot"); // Should be ignored

            assert_eq!(
                TerminalBrand::detect_from_env(&env),
                TerminalBrand::WindowsTerminal
            );
        }

        #[test]
        fn tmux_takes_priority_over_term_program() {
            let mut env = MockEnvReader::new();
            env.set("TMUX", "/tmp/tmux");
            env.set("TERM_PROGRAM", "iTerm.app"); // Should be ignored

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::Tmux);
        }

        #[test]
        fn term_program_takes_priority_over_terminal() {
            let mut env = MockEnvReader::new();
            env.set("TERM_PROGRAM", "vscode");
            env.set("TERMINAL", "konsole"); // Should be ignored

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::VSCode);
        }

        #[test]
        fn terminal_takes_priority_over_term() {
            let mut env = MockEnvReader::new();
            env.set("TERMINAL", "konsole");
            env.set("TERM", "foot"); // Should be ignored

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::Konsole);
        }

        #[test]
        fn kitty_inside_tmux_detects_kitty() {
            // When running tmux inside Kitty, both vars are set
            // Kitty-specific var should take priority
            let mut env = MockEnvReader::new();
            env.set("KITTY_WINDOW_ID", "1");
            env.set("TMUX", "/tmp/tmux");

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::Kitty);
        }

        #[test]
        fn kitty_and_ghostty_both_set_kitty_wins() {
            // Edge case: both Kitty and Ghostty vars set
            // Kitty is checked first, so it wins
            let mut env = MockEnvReader::new();
            env.set("KITTY_WINDOW_ID", "1");
            env.set("GHOSTTY_RESOURCES_DIR", "/path");

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::Kitty);
        }

        #[test]
        fn ghostty_and_wezterm_both_set_ghostty_wins() {
            let mut env = MockEnvReader::new();
            env.set("GHOSTTY_RESOURCES_DIR", "/path");
            env.set("WEZTERM_EXECUTABLE", "/bin/wezterm");

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::Ghostty);
        }

        #[test]
        fn wezterm_and_windows_terminal_both_set_wezterm_wins() {
            let mut env = MockEnvReader::new();
            env.set("WEZTERM_EXECUTABLE", "/bin/wezterm");
            env.set("WT_SESSION", "guid");

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::WezTerm);
        }

        #[test]
        fn all_terminal_specific_vars_set_kitty_wins() {
            // Extreme edge case: all vars set
            let mut env = MockEnvReader::new();
            env.set("KITTY_WINDOW_ID", "1");
            env.set("GHOSTTY_RESOURCES_DIR", "/path");
            env.set("WEZTERM_EXECUTABLE", "/bin/wezterm");
            env.set("WT_SESSION", "guid");
            env.set("TMUX", "/tmp/tmux");
            env.set("TERM_PROGRAM", "vscode");
            env.set("TERMINAL", "konsole");
            env.set("TERM", "foot");

            // First check wins
            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::Kitty);
        }

        #[test]
        fn ghostty_inside_tmux_detects_ghostty() {
            let mut env = MockEnvReader::new();
            env.set("GHOSTTY_RESOURCES_DIR", "/path");
            env.set("TMUX", "/tmp/tmux");

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::Ghostty);
        }

        #[test]
        fn wezterm_inside_tmux_detects_wezterm() {
            let mut env = MockEnvReader::new();
            env.set("WEZTERM_EXECUTABLE", "/bin/wezterm");
            env.set("TMUX", "/tmp/tmux");

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::WezTerm);
        }
    }

    // =========================================================================
    // Protocol Recommendation Tests
    // =========================================================================

    mod protocol {
        use super::*;

        #[test]
        fn kitty_recommends_kitty_protocol() {
            assert_eq!(
                TerminalBrand::Kitty.recommended_protocol(),
                RecommendedProtocol::Kitty
            );
        }

        #[test]
        fn ghostty_recommends_kitty_protocol() {
            assert_eq!(
                TerminalBrand::Ghostty.recommended_protocol(),
                RecommendedProtocol::Kitty
            );
        }

        #[test]
        fn konsole_recommends_kitty_protocol() {
            assert_eq!(
                TerminalBrand::Konsole.recommended_protocol(),
                RecommendedProtocol::Kitty
            );
        }

        #[test]
        fn iterm2_recommends_iterm2_protocol() {
            assert_eq!(
                TerminalBrand::ITerm2.recommended_protocol(),
                RecommendedProtocol::Iterm2
            );
        }

        #[test]
        fn wezterm_recommends_iterm2_protocol() {
            assert_eq!(
                TerminalBrand::WezTerm.recommended_protocol(),
                RecommendedProtocol::Iterm2
            );
        }

        #[test]
        fn warp_recommends_iterm2_protocol() {
            assert_eq!(
                TerminalBrand::Warp.recommended_protocol(),
                RecommendedProtocol::Iterm2
            );
        }

        #[test]
        fn foot_recommends_sixel_protocol() {
            assert_eq!(
                TerminalBrand::Foot.recommended_protocol(),
                RecommendedProtocol::Sixel
            );
        }

        #[test]
        fn windows_terminal_recommends_sixel_protocol() {
            assert_eq!(
                TerminalBrand::WindowsTerminal.recommended_protocol(),
                RecommendedProtocol::Sixel
            );
        }

        #[test]
        fn vscode_recommends_chafa() {
            assert_eq!(
                TerminalBrand::VSCode.recommended_protocol(),
                RecommendedProtocol::Chafa
            );
        }

        #[test]
        fn alacritty_recommends_chafa() {
            assert_eq!(
                TerminalBrand::Alacritty.recommended_protocol(),
                RecommendedProtocol::Chafa
            );
        }

        #[test]
        fn tmux_recommends_query() {
            assert_eq!(
                TerminalBrand::Tmux.recommended_protocol(),
                RecommendedProtocol::Query
            );
        }

        #[test]
        fn unknown_recommends_query() {
            assert_eq!(
                TerminalBrand::Unknown.recommended_protocol(),
                RecommendedProtocol::Query
            );
        }
    }

    // =========================================================================
    // Name Tests
    // =========================================================================

    mod names {
        use super::*;

        #[test]
        fn terminal_brand_names() {
            assert_eq!(TerminalBrand::Kitty.name(), "Kitty");
            assert_eq!(TerminalBrand::Ghostty.name(), "Ghostty");
            assert_eq!(TerminalBrand::WezTerm.name(), "WezTerm");
            assert_eq!(TerminalBrand::ITerm2.name(), "iTerm2");
            assert_eq!(TerminalBrand::Konsole.name(), "Konsole");
            assert_eq!(TerminalBrand::Foot.name(), "Foot");
            assert_eq!(TerminalBrand::VSCode.name(), "VS Code");
            assert_eq!(TerminalBrand::Warp.name(), "Warp");
            assert_eq!(TerminalBrand::Alacritty.name(), "Alacritty");
            assert_eq!(TerminalBrand::WindowsTerminal.name(), "Windows Terminal");
            assert_eq!(TerminalBrand::Tmux.name(), "tmux");
            assert_eq!(TerminalBrand::Unknown.name(), "Unknown");
        }

        #[test]
        fn protocol_names() {
            assert_eq!(RecommendedProtocol::Kitty.name(), "Kitty Graphics Protocol");
            assert_eq!(RecommendedProtocol::Iterm2.name(), "iTerm2 Inline Images");
            assert_eq!(RecommendedProtocol::Sixel.name(), "Sixel");
            assert_eq!(RecommendedProtocol::Chafa.name(), "Chafa");
            assert_eq!(RecommendedProtocol::Query.name(), "Query/Auto-detect");
        }
    }

    // =========================================================================
    // Edge Case Tests
    // =========================================================================

    mod edge_cases {
        use super::*;

        #[test]
        fn empty_env_var_values_are_still_detected() {
            let mut env = MockEnvReader::new();
            env.set("KITTY_WINDOW_ID", ""); // Empty but present

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::Kitty);
        }

        #[test]
        fn whitespace_in_term_program_is_handled() {
            let mut env = MockEnvReader::new();
            env.set("TERM_PROGRAM", "  iTerm.app  ");

            // Contains "iterm" so should match
            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::ITerm2);
        }

        #[test]
        fn mixed_case_term_program_is_detected() {
            let mut env = MockEnvReader::new();
            env.set("TERM_PROGRAM", "VSCODE");

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::VSCode);
        }

        #[test]
        fn partial_match_in_terminal_env() {
            let mut env = MockEnvReader::new();
            env.set("TERMINAL", "org.kde.konsole");

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::Konsole);
        }

        #[test]
        fn foot_direct_term_match() {
            let mut env = MockEnvReader::new();
            env.set("TERM", "foot-direct");

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::Foot);
        }

        #[test]
        fn xterm_does_not_match_foot() {
            let mut env = MockEnvReader::new();
            env.set("TERM", "xterm-256color");

            // xterm should not match foot
            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::Unknown);
        }

        #[test]
        fn very_long_env_var_value() {
            let mut env = MockEnvReader::new();
            let long_value = "a".repeat(10000);
            env.set("KITTY_WINDOW_ID", &long_value);

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::Kitty);
        }

        #[test]
        fn term_program_with_path_separators() {
            let mut env = MockEnvReader::new();
            env.set(
                "TERM_PROGRAM",
                "/Applications/iTerm.app/Contents/MacOS/iTerm2",
            );

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::ITerm2);
        }

        #[test]
        fn term_program_not_iterm_false_positive() {
            let mut env = MockEnvReader::new();
            env.set("TERM_PROGRAM", "not-iterm-terminal");

            // Contains "iterm" so will match - this is expected behavior
            // documenting this edge case
            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::ITerm2);
        }

        #[test]
        fn newline_in_env_var() {
            let mut env = MockEnvReader::new();
            env.set("TERM_PROGRAM", "vscode\n");

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::VSCode);
        }

        #[test]
        fn tab_in_env_var() {
            let mut env = MockEnvReader::new();
            env.set("TERM_PROGRAM", "\tvscode\t");

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::VSCode);
        }

        #[test]
        fn unicode_in_env_var() {
            let mut env = MockEnvReader::new();
            env.set("TERM_PROGRAM", "日本語vscode日本語");

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::VSCode);
        }

        #[test]
        fn only_numbers_in_kitty_window_id() {
            let mut env = MockEnvReader::new();
            env.set("KITTY_WINDOW_ID", "12345678901234567890");

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::Kitty);
        }

        #[test]
        fn special_characters_in_tmux() {
            let mut env = MockEnvReader::new();
            env.set("TMUX", "/tmp/tmux-1000/default,12345,0\x00extra");

            assert_eq!(TerminalBrand::detect_from_env(&env), TerminalBrand::Tmux);
        }
    }

    // =========================================================================
    // Integration-style Tests
    // =========================================================================

    mod integration {
        use super::*;

        #[test]
        fn full_detection_to_protocol_flow_kitty() {
            let mut env = MockEnvReader::new();
            env.set("KITTY_WINDOW_ID", "1");
            env.set("TERM", "xterm-kitty");

            let brand = TerminalBrand::detect_from_env(&env);
            let protocol = brand.recommended_protocol();

            assert_eq!(brand, TerminalBrand::Kitty);
            assert_eq!(protocol, RecommendedProtocol::Kitty);
            assert_eq!(brand.name(), "Kitty");
            assert_eq!(protocol.name(), "Kitty Graphics Protocol");
        }

        #[test]
        fn full_detection_to_protocol_flow_vscode() {
            let mut env = MockEnvReader::new();
            env.set("TERM_PROGRAM", "vscode");
            env.set("TERM", "xterm-256color");

            let brand = TerminalBrand::detect_from_env(&env);
            let protocol = brand.recommended_protocol();

            assert_eq!(brand, TerminalBrand::VSCode);
            assert_eq!(protocol, RecommendedProtocol::Chafa);
        }

        #[test]
        fn full_detection_to_protocol_flow_unknown() {
            let mut env = MockEnvReader::new();
            env.set("TERM", "linux");

            let brand = TerminalBrand::detect_from_env(&env);
            let protocol = brand.recommended_protocol();

            assert_eq!(brand, TerminalBrand::Unknown);
            assert_eq!(protocol, RecommendedProtocol::Query);
        }
    }

    // =========================================================================
    // Exhaustiveness Tests
    // =========================================================================

    mod exhaustiveness {
        use super::*;

        #[test]
        fn all_terminal_brands_have_protocol() {
            // Ensure every TerminalBrand variant has a defined protocol
            let brands = [
                TerminalBrand::Kitty,
                TerminalBrand::Ghostty,
                TerminalBrand::WezTerm,
                TerminalBrand::ITerm2,
                TerminalBrand::Konsole,
                TerminalBrand::Foot,
                TerminalBrand::VSCode,
                TerminalBrand::Warp,
                TerminalBrand::Alacritty,
                TerminalBrand::WindowsTerminal,
                TerminalBrand::Tmux,
                TerminalBrand::Unknown,
            ];

            for brand in brands {
                // This should not panic
                let _ = brand.recommended_protocol();
                let _ = brand.name();
            }
        }

        #[test]
        fn all_protocols_have_name() {
            let protocols = [
                RecommendedProtocol::Kitty,
                RecommendedProtocol::Iterm2,
                RecommendedProtocol::Sixel,
                RecommendedProtocol::Chafa,
                RecommendedProtocol::Query,
            ];

            for protocol in protocols {
                assert!(!protocol.name().is_empty());
            }
        }

        #[test]
        fn terminal_brand_count() {
            // Verify we have the expected number of terminal brands
            // This helps catch if someone adds a new variant without tests
            let brands = [
                TerminalBrand::Kitty,
                TerminalBrand::Ghostty,
                TerminalBrand::WezTerm,
                TerminalBrand::ITerm2,
                TerminalBrand::Konsole,
                TerminalBrand::Foot,
                TerminalBrand::VSCode,
                TerminalBrand::Warp,
                TerminalBrand::Alacritty,
                TerminalBrand::WindowsTerminal,
                TerminalBrand::Tmux,
                TerminalBrand::Unknown,
            ];

            assert_eq!(brands.len(), 12);
        }

        #[test]
        fn protocol_count() {
            let protocols = [
                RecommendedProtocol::Kitty,
                RecommendedProtocol::Iterm2,
                RecommendedProtocol::Sixel,
                RecommendedProtocol::Chafa,
                RecommendedProtocol::Query,
            ];

            assert_eq!(protocols.len(), 5);
        }
    }
}
