//! Application configuration from CLI arguments

use std::env;
use std::io::{self, BufRead, IsTerminal};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use super::config_file::{CommandsConfig, ConfigFile, PreviewConfig};
use crate::integrate::{
    exit_code, Callback, ContextAgent, ContextPackFormat, ContextPackOptions, ContextPackPreset,
    OutputFormat,
};

/// Session action (save, restore, clear)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionAction {
    /// Save current selection to session file
    Save,
    /// Restore selection from session file
    Restore,
    /// Clear session file
    Clear,
}

/// Plugin command action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginAction {
    /// Create plugin template
    Init,
    /// Validate/execute plugin file
    Test,
}

/// Application configuration from CLI args and config file
pub struct Config {
    pub root: PathBuf,
    pub pick_mode: bool,
    pub output_format: OutputFormat,
    pub callback: Option<Callback>,
    pub icons_enabled: Option<bool>,
    /// Shell integration: output directory path on exit (for cd)
    pub choosedir_mode: bool,
    /// Shell integration: file to write directory path on exit
    pub choosedir_file: Option<PathBuf>,
    /// Shell integration: file to write selection on exit
    pub selection_path_file: Option<PathBuf>,
    /// Paths read from stdin (for pipeline integration)
    pub stdin_paths: Option<Vec<PathBuf>>,
    /// Show hidden files by default (from config file)
    pub show_hidden: bool,
    /// Enable mouse support (from config file)
    pub mouse_enabled: bool,
    /// Maximum bytes for hex preview (from config file)
    pub hex_max_bytes: usize,
    /// Maximum entries for archive preview (from config file)
    pub max_archive_entries: usize,
    /// Image protocol setting (from config file)
    pub image_protocol: String,
    /// Git poll interval (from config file)
    pub git_poll_interval: Duration,
    /// Show file size in tree (from config file)
    pub show_size: bool,
    /// Show file permissions in tree (from config file)
    pub show_permissions: bool,
    /// Date format string (from config file)
    pub date_format: String,
    /// Custom commands configuration
    pub commands: CommandsConfig,
    /// Custom preview configuration
    pub preview_custom: PreviewConfig,
    /// Tree output mode (non-interactive, output to stdout)
    pub tree_mode: bool,
    /// Maximum depth for tree output (None = unlimited)
    pub tree_depth: Option<usize>,
    /// Include file content with pick output
    pub with_content: bool,
    /// Select mode (simpler interactive selection)
    pub select_mode: bool,
    /// Allow multiple selection in select mode
    pub multi_select: bool,
    /// MCP server mode
    pub mcp_server: bool,
    /// Context generation mode
    pub context_mode: bool,
    /// AI benchmark mode (non-interactive)
    pub benchmark_ai: bool,
    /// AI benchmark scenario (context-pack, review-pack, related, all)
    pub benchmark_scenario: String,
    /// AI benchmark iterations
    pub benchmark_iterations: usize,
    /// Context pack output mode with preset
    pub context_pack: Option<ContextPackPreset>,
    /// Context pack options
    pub context_pack_options: ContextPackOptions,
    /// Related file selection output mode (non-interactive)
    pub select_related_path: Option<PathBuf>,
    /// Explain related-file selection scoring
    pub explain_selection: bool,
    /// Session action (save/restore/clear) - non-interactive
    pub session_action: Option<SessionAction>,
    /// Plugin command action
    pub plugin_action: Option<PluginAction>,
    /// Plugin path for plugin commands
    pub plugin_path: Option<PathBuf>,
}

impl Config {
    pub fn from_args() -> anyhow::Result<Self> {
        // Load config file first (provides defaults)
        let config_file = ConfigFile::load();

        let mut args = env::args().skip(1).peekable();
        let mut root = env::current_dir()?;
        let mut pick_mode = false;
        let mut output_format = OutputFormat::default();
        let mut callback: Option<Callback> = None;
        let mut icons_enabled: Option<bool> = None;
        let mut choosedir_mode = false;
        let mut choosedir_file: Option<PathBuf> = None;
        let mut selection_path_file: Option<PathBuf> = None;
        let mut stdin_mode = false;
        let mut show_hidden: Option<bool> = None;
        let mut tree_mode = false;
        let mut tree_depth: Option<usize> = None;
        let mut with_content = false;
        let mut select_mode = false;
        let mut multi_select = false;
        let mut mcp_server = false;
        let mut context_mode = false;
        let mut benchmark_ai = false;
        let mut benchmark_scenario = "context-pack".to_string();
        let mut benchmark_iterations = 5usize;
        let mut context_pack: Option<ContextPackPreset> = None;
        let mut context_pack_format = ContextPackFormat::AiMarkdown;
        let mut context_pack_options = ContextPackOptions::default();
        let mut select_related_path: Option<PathBuf> = None;
        let mut explain_selection = false;
        let mut session_action: Option<SessionAction> = None;
        let mut plugin_action: Option<PluginAction> = None;
        let mut plugin_path: Option<PathBuf> = None;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--pick" | "-p" => pick_mode = true,
                "--choosedir" => {
                    choosedir_mode = true;
                    // Check if next arg is a file path (not starting with -)
                    if let Some(next) = args.peek() {
                        if !next.starts_with('-') {
                            choosedir_file = Some(PathBuf::from(args.next().unwrap()));
                        }
                    }
                }
                "--selection-path" => {
                    if let Some(file) = args.next() {
                        selection_path_file = Some(PathBuf::from(file));
                    } else {
                        anyhow::bail!("--selection-path requires a file path");
                    }
                }
                "--stdin" => stdin_mode = true,
                "--tree" | "-t" => tree_mode = true,
                "--depth" => {
                    if let Some(depth_str) = args.next() {
                        tree_depth = Some(depth_str.parse().map_err(|_| {
                            anyhow::anyhow!(
                                "--depth requires a positive integer, got '{}'",
                                depth_str
                            )
                        })?);
                    } else {
                        anyhow::bail!("--depth requires a value");
                    }
                }
                "--with-content" => with_content = true,
                "--select-mode" => select_mode = true,
                "--multi" => multi_select = true,
                "--mcp-server" => mcp_server = true,
                "--context" => context_mode = true,
                "benchmark" => {
                    let mode = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("benchmark requires a mode (e.g. ai)"))?;
                    if mode != "ai" {
                        anyhow::bail!("unsupported benchmark mode: {} (expected: ai)", mode);
                    }
                    benchmark_ai = true;

                    while let Some(next) = args.peek().cloned() {
                        match next.as_str() {
                            "--scenario" => {
                                args.next();
                                benchmark_scenario = args.next().ok_or_else(|| {
                                    anyhow::anyhow!("--scenario requires a value")
                                })?;
                            }
                            "--iterations" => {
                                args.next();
                                let value = args.next().ok_or_else(|| {
                                    anyhow::anyhow!("--iterations requires a value")
                                })?;
                                benchmark_iterations = value.parse().map_err(|_| {
                                    anyhow::anyhow!("--iterations requires a positive integer")
                                })?;
                            }
                            token if token.starts_with('-') => {
                                anyhow::bail!("unknown benchmark option: {}", token);
                            }
                            path => {
                                let p = PathBuf::from(path);
                                if p.is_dir() {
                                    root = p.canonicalize()?;
                                    args.next();
                                }
                                break;
                            }
                        }
                    }
                }
                "--context-pack" => {
                    if let Some(preset) = args.next() {
                        context_pack =
                            Some(ContextPackPreset::from_str(&preset).map_err(|_| {
                                anyhow::anyhow!(
                                    "--context-pack must be one of: minimal, review, debug, refactor, incident, onboarding"
                                )
                            })?);
                    } else {
                        anyhow::bail!("--context-pack requires a preset");
                    }
                }
                "--context-format" => {
                    if let Some(fmt) = args.next() {
                        context_pack_format = ContextPackFormat::from_str(&fmt).map_err(|_| {
                            anyhow::anyhow!("--context-format must be one of: ai-md, jsonl")
                        })?;
                    } else {
                        anyhow::bail!("--context-format requires a value");
                    }
                }
                "--agent" => {
                    if let Some(agent) = args.next() {
                        context_pack_options.agent =
                            ContextAgent::from_str(&agent).map_err(|_| {
                                anyhow::anyhow!("--agent must be one of: claude, codex, cursor")
                            })?;
                    } else {
                        anyhow::bail!("--agent requires a value");
                    }
                }
                "--token-budget" => {
                    if let Some(value) = args.next() {
                        context_pack_options.token_budget = value.parse().map_err(|_| {
                            anyhow::anyhow!("--token-budget requires a positive integer")
                        })?;
                    } else {
                        anyhow::bail!("--token-budget requires a value");
                    }
                }
                "--include-git-diff" => context_pack_options.include_git_diff = true,
                "--include-tests" => context_pack_options.include_tests = true,
                "--context-depth" => {
                    if let Some(value) = args.next() {
                        context_pack_options.depth = value.parse().map_err(|_| {
                            anyhow::anyhow!("--context-depth requires a positive integer")
                        })?;
                    } else {
                        anyhow::bail!("--context-depth requires a value");
                    }
                }
                "--select-related" => {
                    if let Some(path) = args.next() {
                        select_related_path = Some(PathBuf::from(path));
                    } else {
                        anyhow::bail!("--select-related requires a file path");
                    }
                }
                "--explain-selection" => explain_selection = true,
                "--session" => {
                    if let Some(action) = args.next() {
                        session_action = Some(match action.as_str() {
                            "save" => SessionAction::Save,
                            "restore" => SessionAction::Restore,
                            "clear" => SessionAction::Clear,
                            _ => {
                                anyhow::bail!(
                                    "--session requires 'save', 'restore', or 'clear', got '{}'",
                                    action
                                );
                            }
                        });
                    } else {
                        anyhow::bail!("--session requires 'save', 'restore', or 'clear'");
                    }
                }
                "plugin" => {
                    let sub = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("plugin requires 'init' or 'test'"))?;
                    match sub.as_str() {
                        "init" => {
                            plugin_action = Some(PluginAction::Init);
                            if let Some(next) = args.peek() {
                                if !next.starts_with('-') {
                                    plugin_path = Some(PathBuf::from(args.next().unwrap()));
                                }
                            }
                        }
                        "test" => {
                            plugin_action = Some(PluginAction::Test);
                            let path = args.next().ok_or_else(|| {
                                anyhow::anyhow!("plugin test requires a path to .lua file")
                            })?;
                            plugin_path = Some(PathBuf::from(path));
                        }
                        _ => anyhow::bail!("Unknown plugin command: {}", sub),
                    }
                }
                "--icons" | "-i" => icons_enabled = Some(true),
                "--no-icons" => icons_enabled = Some(false),
                "--hidden" | "-a" => show_hidden = Some(true),
                "--no-hidden" => show_hidden = Some(false),
                "--format" | "-f" => {
                    if let Some(fmt) = args.next() {
                        output_format = OutputFormat::from_str(&fmt).map_err(|_| {
                            anyhow::anyhow!(
                                "Invalid format '{}'. Valid formats: lines, null, json",
                                fmt
                            )
                        })?;
                    } else {
                        anyhow::bail!("--format requires a value (lines, null, or json)");
                    }
                }
                "--on-select" => {
                    if let Some(cmd) = args.next() {
                        callback = Some(Callback::new(cmd));
                    } else {
                        anyhow::bail!("--on-select requires a command");
                    }
                }
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(exit_code::SUCCESS);
                }
                "--version" | "-V" => {
                    println!("fv {}", env!("CARGO_PKG_VERSION"));
                    std::process::exit(exit_code::SUCCESS);
                }
                path if !path.starts_with('-') => {
                    let p = PathBuf::from(path);
                    if p.is_dir() {
                        root = p.canonicalize()?;
                    } else if p.is_file() {
                        if let Some(parent) = p.parent() {
                            root = parent.canonicalize()?;
                        }
                    } else {
                        anyhow::bail!("Path does not exist: {}", path);
                    }
                }
                unknown => {
                    anyhow::bail!(
                        "Unknown option: {}. Use --help for usage information.",
                        unknown
                    );
                }
            }
        }

        // Handle stdin mode
        let stdin_paths = if stdin_mode {
            Some(read_stdin_paths()?)
        } else {
            None
        };

        // If stdin paths are provided, determine root from common ancestor
        let root = if let Some(ref paths) = stdin_paths {
            find_common_ancestor(paths)?
        } else {
            root
        };

        context_pack_options.format = context_pack_format;

        // Merge config file settings with CLI overrides
        // CLI arguments take precedence over config file
        Ok(Self {
            root,
            pick_mode,
            output_format,
            callback,
            icons_enabled,
            choosedir_mode,
            choosedir_file,
            selection_path_file,
            stdin_paths,
            // Settings from config file (CLI can override some)
            show_hidden: show_hidden.unwrap_or(config_file.general.show_hidden),
            mouse_enabled: config_file.general.mouse_enabled,
            hex_max_bytes: config_file.preview.hex_max_bytes,
            max_archive_entries: config_file.preview.max_archive_entries,
            image_protocol: config_file.preview.image_protocol.clone(),
            git_poll_interval: Duration::from_secs(config_file.performance.git_poll_interval_secs),
            show_size: config_file.ui.show_size,
            show_permissions: config_file.ui.show_permissions,
            date_format: config_file.ui.date_format,
            commands: config_file.commands,
            preview_custom: config_file.preview,
            tree_mode,
            tree_depth,
            with_content,
            select_mode,
            multi_select,
            mcp_server,
            context_mode,
            benchmark_ai,
            benchmark_scenario,
            benchmark_iterations,
            context_pack,
            context_pack_options,
            select_related_path,
            explain_selection,
            session_action,
            plugin_action,
            plugin_path,
        })
    }
}

/// Read paths from stdin (one path per line)
fn read_stdin_paths() -> anyhow::Result<Vec<PathBuf>> {
    let stdin = io::stdin();

    // Check if stdin is a TTY (not piped)
    if stdin.is_terminal() {
        anyhow::bail!("--stdin requires piped input");
    }

    let cwd = env::current_dir()?;
    let paths: Vec<PathBuf> = stdin
        .lock()
        .lines()
        .map_while(Result::ok)
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let path = PathBuf::from(line.trim());
            let resolved = if path.is_absolute() {
                path
            } else {
                cwd.join(&path)
            };

            // canonicalize() resolves ".." components and verifies path exists
            // This prevents path traversal attacks like "../../../etc/passwd"
            resolved.canonicalize().ok()
        })
        .collect();

    if paths.is_empty() {
        anyhow::bail!("No valid paths provided via stdin");
    }

    Ok(paths)
}

/// Find the common ancestor directory of all paths
fn find_common_ancestor(paths: &[PathBuf]) -> anyhow::Result<PathBuf> {
    if paths.is_empty() {
        return env::current_dir().map_err(Into::into);
    }

    // Start with the first path's parent (or itself if it's a directory)
    let first = &paths[0];
    let mut ancestor = if first.is_dir() {
        first.clone()
    } else {
        first
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from("/")))
    };

    // Find common prefix with all other paths
    for path in paths.iter().skip(1) {
        let other = if path.is_dir() {
            path.clone()
        } else {
            path.parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| ancestor.clone())
        };

        // Walk up until we find a common ancestor
        while !other.starts_with(&ancestor) {
            if let Some(parent) = ancestor.parent() {
                ancestor = parent.to_path_buf();
            } else {
                // Reached root
                break;
            }
        }
    }

    Ok(ancestor)
}

fn print_help() {
    println!(
        r#"fv - FileView: A minimal file tree UI

USAGE:
    fv [OPTIONS] [PATH]
    command | fv --stdin [OPTIONS]
    fv benchmark ai [--scenario NAME] [--iterations N] [PATH]

OPTIONS:
    -p, --pick          Pick mode: output selected path(s) to stdout
    -f, --format FMT    Output format for pick mode: lines, null, json
    --stdin             Read paths from stdin (one per line)
    --on-select CMD     Run command when file is selected (use {{path}}, {{name}}, etc.)
    --choosedir [FILE]  Write directory path to FILE on exit (for shell cd integration)
    --selection-path F  Write selected file paths to FILE on exit
    -i, --icons         Enable Nerd Fonts icons (default)
    --no-icons          Disable icons
    -a, --hidden        Show hidden files
    --no-hidden         Hide hidden files (default)
    -h, --help          Show this help message
    -V, --version       Show version

CLAUDE CODE INTEGRATION:
    -t, --tree          Output directory tree to stdout (non-interactive)
    --depth N           Limit tree depth to N levels
    --with-content      Include file contents in pick output (Claude format)
    --select-mode       Simple selection mode: Enter to select, output to stdout
    --multi             Allow multiple selection in select mode
    --mcp-server        Run as MCP server (JSON-RPC over stdin/stdout)
    --context           Output project context in AI-friendly markdown format
    --context-pack P    Output AI context pack preset: minimal, review, debug, refactor, incident, onboarding
    --context-format F  Context output format: ai-md, jsonl
    --agent A           Target agent profile: claude, codex, cursor
    --token-budget N    Token budget for context pack (default: 4000)
    --include-git-diff  Force include git diff summary in context pack
    --include-tests     Include inferred test files in context pack
    --context-depth N   Max recursive scan depth for fallback file discovery
    --select-related F  Output related file paths for file F
    --explain-selection Include score/reasons for --select-related output
    --session ACTION    Session management: save, restore, or clear
    benchmark ai        Run AI benchmark scenarios (context-pack/review-pack/related/all)
    plugin init [PATH]  Create plugin template file (default: ~/.config/fileview/plugins/init.lua)
    plugin test PATH    Execute plugin file in sandbox and report status

CONFIG FILE:
    ~/.config/fileview/config.toml    Main configuration file
    ~/.config/fileview/keymap.toml    Key bindings (customizable)
    ~/.config/fileview/theme.toml     Color theme

ENVIRONMENT:
    FILEVIEW_ICONS=0            Disable icons
    FILEVIEW_IMAGE_PROTOCOL     Force image protocol: auto, halfblocks, chafa, sixel, kitty, iterm2

KEYBINDINGS:
    j/↓         Move down
    k/↑         Move up
    g           Go to top
    G           Go to bottom
    l/→/Tab     Expand / Toggle
    h/←/BS      Collapse
    H           Collapse all
    L           Expand all
    Space       Toggle mark
    Enter       Toggle expand (or select in pick mode)
    y           Copy to clipboard
    d           Cut to clipboard
    D/Del       Delete (with confirmation)
    p           Paste
    a           New file
    A           New directory
    r           Rename
    /           Search
    n           Next search result
    Ctrl+P      Fuzzy finder
    .           Toggle hidden files
    R/F5        Refresh
    o           Open preview
    P           Toggle quick preview panel
    c           Copy path to system clipboard
    C           Copy filename to system clipboard
    Alt+S       Open subshell in current directory
    q/Esc       Quit (or cancel in pick mode)
    Q           Quit and cd to current directory (with --choosedir)
    ?           Show help

SMART SELECTION:
    Ctrl+G      Select all git changed files
    Ctrl+T      Select test pair for focused file
    Ctrl+R      Select related files
    Ctrl+E      Select error-context files

AI WORKFLOW:
    Ctrl+A      Toggle AI focus mode (ultra compact + peek preview)
    Ctrl+Shift+Y Copy context pack to clipboard
    Ctrl+Shift+P Open AI history

TABS:
    Ctrl+T      New tab
    Ctrl+W      Close tab
    Alt+t       Next tab
    Alt+T       Previous tab

PLACEHOLDERS for --on-select:
    {{path}}    Full path
    {{dir}}     Parent directory
    {{name}}    Filename with extension
    {{stem}}    Filename without extension
    {{ext}}     Extension only

FEATURES:
    Auto-refresh    Automatically refreshes on file changes (disabled in stdin mode)

EXIT CODES:
    0           Success (normal exit or file selected)
    1           Cancelled (user cancelled selection in pick mode)
    2           Error (runtime error)
    3           Invalid arguments (unknown option or invalid value)

SHELL INTEGRATION:
    Add to ~/.bashrc or ~/.zshrc:

    fv() {{
        local tmp=$(mktemp)
        command fv --choosedir "$tmp" "$@"
        local dir=$(cat "$tmp")
        rm -f "$tmp"
        [ -n "$dir" ] && cd "$dir"
    }}
"#
    );
}
