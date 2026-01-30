//! Application configuration from CLI arguments

use std::env;
use std::io::{self, BufRead, IsTerminal};
use std::path::PathBuf;
use std::str::FromStr;

use crate::integrate::{exit_code, Callback, OutputFormat};

/// Application configuration from CLI args
pub struct Config {
    pub root: PathBuf,
    pub pick_mode: bool,
    pub output_format: OutputFormat,
    pub callback: Option<Callback>,
    pub icons_enabled: Option<bool>,
    /// Shell integration: output directory path on exit (for cd)
    pub choosedir_mode: bool,
    /// Paths read from stdin (for pipeline integration)
    pub stdin_paths: Option<Vec<PathBuf>>,
}

impl Config {
    pub fn from_args() -> anyhow::Result<Self> {
        let mut args = env::args().skip(1).peekable();
        let mut root = env::current_dir()?;
        let mut pick_mode = false;
        let mut output_format = OutputFormat::default();
        let mut callback: Option<Callback> = None;
        let mut icons_enabled: Option<bool> = None;
        let mut choosedir_mode = false;
        let mut stdin_mode = false;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--pick" | "-p" => pick_mode = true,
                "--choosedir" => choosedir_mode = true,
                "--stdin" => stdin_mode = true,
                "--icons" | "-i" => icons_enabled = Some(true),
                "--no-icons" => icons_enabled = Some(false),
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

        Ok(Self {
            root,
            pick_mode,
            output_format,
            callback,
            icons_enabled,
            choosedir_mode,
            stdin_paths,
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

    let paths: Vec<PathBuf> = stdin
        .lock()
        .lines()
        .map_while(Result::ok)
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let path = PathBuf::from(line.trim());
            // Normalize path (resolve relative paths, handle . and ..)
            if path.is_absolute() {
                path
            } else {
                env::current_dir()
                    .map(|cwd| cwd.join(&path))
                    .unwrap_or(path)
            }
        })
        .collect();

    if paths.is_empty() {
        anyhow::bail!("No paths provided via stdin");
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

OPTIONS:
    -p, --pick          Pick mode: output selected path(s) to stdout
    -f, --format FMT    Output format for pick mode: lines, null, json
    --stdin             Read paths from stdin (one per line)
    --on-select CMD     Run command when file is selected (use {{path}}, {{name}}, etc.)
    --choosedir         Output directory path on exit (press Q to cd there)
    -i, --icons         Enable Nerd Fonts icons (default)
    --no-icons          Disable icons
    -h, --help          Show this help message
    -V, --version       Show version

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
    q/Esc       Quit (or cancel in pick mode)
    Q           Quit and cd to current directory (with --choosedir)
    ?           Show help

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
"#
    );
}
