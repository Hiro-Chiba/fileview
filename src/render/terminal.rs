//! Terminal detection and image protocol selection
//!
//! This module detects the terminal emulator and selects the best
//! image rendering protocol available.

use std::env;

/// Known terminal emulators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalKind {
    /// Ghostty terminal
    Ghostty,
    /// Kitty terminal
    Kitty,
    /// iTerm2 terminal
    ITerm2,
    /// WezTerm terminal
    WezTerm,
    /// VS Code integrated terminal
    VSCode,
    /// macOS Terminal.app
    TerminalApp,
    /// Windows Terminal
    WindowsTerminal,
    /// Alacritty terminal
    Alacritty,
    /// foot terminal (Wayland)
    Foot,
    /// mlterm terminal
    Mlterm,
    /// xterm
    Xterm,
    /// Unknown terminal
    Unknown,
}

/// Image rendering protocols
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageProtocol {
    /// Sixel graphics (wide support)
    Sixel,
    /// Kitty graphics protocol (Kitty only)
    Kitty,
    /// iTerm2 inline images (iTerm2, WezTerm)
    ITerm2,
    /// Half-block character rendering (universal fallback)
    #[default]
    HalfBlock,
}

impl std::fmt::Display for ImageProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageProtocol::Sixel => write!(f, "sixel"),
            ImageProtocol::Kitty => write!(f, "kitty"),
            ImageProtocol::ITerm2 => write!(f, "iterm2"),
            ImageProtocol::HalfBlock => write!(f, "halfblock"),
        }
    }
}

impl std::str::FromStr for ImageProtocol {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "sixel" => Ok(ImageProtocol::Sixel),
            "kitty" => Ok(ImageProtocol::Kitty),
            "iterm2" => Ok(ImageProtocol::ITerm2),
            "halfblock" | "half-block" | "block" => Ok(ImageProtocol::HalfBlock),
            // "auto" defaults to HalfBlock here, but callers should use detect_best_protocol()
            // for actual auto-detection at runtime
            "auto" => Ok(ImageProtocol::HalfBlock),
            _ => Err(format!("Unknown image protocol: {}", s)),
        }
    }
}

/// Detect the current terminal emulator
pub fn detect_terminal() -> TerminalKind {
    // Check specific environment variables first (most reliable)

    // Ghostty
    if env::var("GHOSTTY_RESOURCES_DIR").is_ok() {
        return TerminalKind::Ghostty;
    }

    // Kitty
    if env::var("KITTY_WINDOW_ID").is_ok() {
        return TerminalKind::Kitty;
    }

    // WezTerm
    if env::var("WEZTERM_EXECUTABLE").is_ok() || env::var("WEZTERM_PANE").is_ok() {
        return TerminalKind::WezTerm;
    }

    // Check TERM_PROGRAM
    if let Ok(term_program) = env::var("TERM_PROGRAM") {
        match term_program.to_lowercase().as_str() {
            "iterm.app" => return TerminalKind::ITerm2,
            "vscode" => return TerminalKind::VSCode,
            "apple_terminal" => return TerminalKind::TerminalApp,
            "alacritty" => return TerminalKind::Alacritty,
            "wezterm" => return TerminalKind::WezTerm,
            _ => {}
        }
    }

    // Check LC_TERMINAL (used by some terminals)
    if let Ok(lc_terminal) = env::var("LC_TERMINAL") {
        if lc_terminal.to_lowercase().as_str() == "iterm2" {
            return TerminalKind::ITerm2;
        }
    }

    // Check WT_SESSION for Windows Terminal
    if env::var("WT_SESSION").is_ok() {
        return TerminalKind::WindowsTerminal;
    }

    // Check TERM for xterm variants
    if let Ok(term) = env::var("TERM") {
        let term_lower = term.to_lowercase();
        if term_lower.contains("foot") {
            return TerminalKind::Foot;
        }
        if term_lower.contains("mlterm") {
            return TerminalKind::Mlterm;
        }
        if term_lower.starts_with("xterm") {
            return TerminalKind::Xterm;
        }
    }

    TerminalKind::Unknown
}

/// Get the best image protocol for a terminal
///
/// Note: Some terminals require specific versions or settings:
/// - Windows Terminal: Requires v1.22+ (stable Feb 2025)
/// - VS Code: Requires `terminal.integrated.enableImages: true`
/// - xterm: Must be compiled with Sixel support
pub fn best_protocol_for_terminal(terminal: TerminalKind) -> ImageProtocol {
    match terminal {
        // Sixel-capable terminals
        TerminalKind::Ghostty => ImageProtocol::Sixel,
        TerminalKind::ITerm2 => ImageProtocol::Sixel, // iTerm2 supports both, Sixel is simpler
        TerminalKind::WezTerm => ImageProtocol::Sixel,
        TerminalKind::Foot => ImageProtocol::Sixel,
        TerminalKind::Mlterm => ImageProtocol::Sixel,
        TerminalKind::Xterm => ImageProtocol::Sixel, // May not work if not compiled with Sixel
        TerminalKind::VSCode => ImageProtocol::Sixel, // Requires terminal.integrated.enableImages
        TerminalKind::WindowsTerminal => ImageProtocol::Sixel, // Requires v1.22+ (Feb 2025)

        // Kitty protocol
        TerminalKind::Kitty => ImageProtocol::Kitty,

        // No native image protocol support - use half-block fallback
        TerminalKind::TerminalApp => ImageProtocol::HalfBlock, // macOS Terminal.app - no Sixel
        TerminalKind::Alacritty => ImageProtocol::HalfBlock,   // No plans to add Sixel support
        TerminalKind::Unknown => ImageProtocol::HalfBlock,
    }
}

/// Detect terminal and return the best image protocol
pub fn detect_best_protocol() -> ImageProtocol {
    let terminal = detect_terminal();
    best_protocol_for_terminal(terminal)
}

/// Check if a protocol is supported by the detected terminal
pub fn is_protocol_supported(protocol: ImageProtocol) -> bool {
    let terminal = detect_terminal();
    match protocol {
        ImageProtocol::Sixel => matches!(
            terminal,
            TerminalKind::Ghostty
                | TerminalKind::ITerm2
                | TerminalKind::WezTerm
                | TerminalKind::Foot
                | TerminalKind::Mlterm
                | TerminalKind::Xterm
                | TerminalKind::VSCode
                | TerminalKind::WindowsTerminal
        ),
        ImageProtocol::Kitty => matches!(terminal, TerminalKind::Kitty),
        ImageProtocol::ITerm2 => {
            matches!(terminal, TerminalKind::ITerm2 | TerminalKind::WezTerm)
        }
        ImageProtocol::HalfBlock => true, // Always supported
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_protocol_display() {
        assert_eq!(format!("{}", ImageProtocol::Sixel), "sixel");
        assert_eq!(format!("{}", ImageProtocol::Kitty), "kitty");
        assert_eq!(format!("{}", ImageProtocol::ITerm2), "iterm2");
        assert_eq!(format!("{}", ImageProtocol::HalfBlock), "halfblock");
    }

    #[test]
    fn test_image_protocol_from_str() {
        assert_eq!(
            "sixel".parse::<ImageProtocol>().unwrap(),
            ImageProtocol::Sixel
        );
        assert_eq!(
            "SIXEL".parse::<ImageProtocol>().unwrap(),
            ImageProtocol::Sixel
        );
        assert_eq!(
            "kitty".parse::<ImageProtocol>().unwrap(),
            ImageProtocol::Kitty
        );
        assert_eq!(
            "iterm2".parse::<ImageProtocol>().unwrap(),
            ImageProtocol::ITerm2
        );
        assert_eq!(
            "halfblock".parse::<ImageProtocol>().unwrap(),
            ImageProtocol::HalfBlock
        );
        assert_eq!(
            "half-block".parse::<ImageProtocol>().unwrap(),
            ImageProtocol::HalfBlock
        );
        assert_eq!(
            "block".parse::<ImageProtocol>().unwrap(),
            ImageProtocol::HalfBlock
        );
        assert_eq!(
            "auto".parse::<ImageProtocol>().unwrap(),
            ImageProtocol::HalfBlock
        );
        assert!("invalid".parse::<ImageProtocol>().is_err());
    }

    #[test]
    fn test_best_protocol_for_terminal() {
        // Sixel-capable terminals
        assert_eq!(
            best_protocol_for_terminal(TerminalKind::Ghostty),
            ImageProtocol::Sixel
        );
        assert_eq!(
            best_protocol_for_terminal(TerminalKind::ITerm2),
            ImageProtocol::Sixel
        );
        assert_eq!(
            best_protocol_for_terminal(TerminalKind::WezTerm),
            ImageProtocol::Sixel
        );
        assert_eq!(
            best_protocol_for_terminal(TerminalKind::Foot),
            ImageProtocol::Sixel
        );
        assert_eq!(
            best_protocol_for_terminal(TerminalKind::Mlterm),
            ImageProtocol::Sixel
        );
        assert_eq!(
            best_protocol_for_terminal(TerminalKind::Xterm),
            ImageProtocol::Sixel
        );
        assert_eq!(
            best_protocol_for_terminal(TerminalKind::VSCode),
            ImageProtocol::Sixel
        );
        assert_eq!(
            best_protocol_for_terminal(TerminalKind::WindowsTerminal),
            ImageProtocol::Sixel
        );

        // Kitty protocol
        assert_eq!(
            best_protocol_for_terminal(TerminalKind::Kitty),
            ImageProtocol::Kitty
        );

        // HalfBlock fallback
        assert_eq!(
            best_protocol_for_terminal(TerminalKind::TerminalApp),
            ImageProtocol::HalfBlock
        );
        assert_eq!(
            best_protocol_for_terminal(TerminalKind::Alacritty),
            ImageProtocol::HalfBlock
        );
        assert_eq!(
            best_protocol_for_terminal(TerminalKind::Unknown),
            ImageProtocol::HalfBlock
        );
    }

    #[test]
    fn test_halfblock_always_supported() {
        assert!(is_protocol_supported(ImageProtocol::HalfBlock));
    }

    #[test]
    fn test_sixel_capable_terminals() {
        // All Sixel-capable terminals should return Sixel
        let sixel_terminals = [
            TerminalKind::Ghostty,
            TerminalKind::ITerm2,
            TerminalKind::WezTerm,
            TerminalKind::Foot,
            TerminalKind::Mlterm,
            TerminalKind::Xterm,
            TerminalKind::VSCode,
            TerminalKind::WindowsTerminal,
        ];

        for terminal in sixel_terminals {
            assert_eq!(
                best_protocol_for_terminal(terminal),
                ImageProtocol::Sixel,
                "Terminal {:?} should support Sixel",
                terminal
            );
        }
    }

    #[test]
    fn test_fallback_terminals() {
        // Terminals without native image support
        let fallback_terminals = [
            TerminalKind::TerminalApp,
            TerminalKind::Alacritty,
            TerminalKind::Unknown,
        ];

        for terminal in fallback_terminals {
            assert_eq!(
                best_protocol_for_terminal(terminal),
                ImageProtocol::HalfBlock,
                "Terminal {:?} should use HalfBlock fallback",
                terminal
            );
        }
    }

    #[test]
    fn test_terminal_kind_variants() {
        // Ensure all variants are distinct
        let kinds = [
            TerminalKind::Ghostty,
            TerminalKind::Kitty,
            TerminalKind::ITerm2,
            TerminalKind::WezTerm,
            TerminalKind::VSCode,
            TerminalKind::TerminalApp,
            TerminalKind::WindowsTerminal,
            TerminalKind::Alacritty,
            TerminalKind::Foot,
            TerminalKind::Mlterm,
            TerminalKind::Xterm,
            TerminalKind::Unknown,
        ];

        for (i, k1) in kinds.iter().enumerate() {
            for (j, k2) in kinds.iter().enumerate() {
                if i == j {
                    assert_eq!(k1, k2);
                } else {
                    assert_ne!(k1, k2);
                }
            }
        }
    }
}
