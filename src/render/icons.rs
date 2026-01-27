//! Nerd Fonts icon mappings for files and directories

use std::path::Path;

/// Get the appropriate icon for a file or directory
pub fn get_icon(path: &Path, is_dir: bool, expanded: bool) -> &'static str {
    if is_dir {
        get_directory_icon(path, expanded)
    } else {
        get_file_icon(path)
    }
}

/// Get icon for a directory
fn get_directory_icon(path: &Path, expanded: bool) -> &'static str {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    // Special directories
    match name {
        ".git" => "",
        "node_modules" => "",
        "src" => "",
        "target" | "build" | "dist" => "",
        "test" | "tests" | "__tests__" => "",
        "docs" | "doc" => "",
        ".github" => "",
        ".vscode" => "",
        // Default folder icons (open/closed)
        _ => {
            if expanded {
                "\u{f07c}"
            } else {
                "\u{f07b}"
            }
        }
    }
}

/// Get icon for a file based on extension or filename
fn get_file_icon(path: &Path) -> &'static str {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    // Special filenames
    match name {
        "Cargo.toml" | "Cargo.lock" => "",
        "package.json" | "package-lock.json" => "",
        "Makefile" | "CMakeLists.txt" => "",
        "Dockerfile" | "docker-compose.yml" | "docker-compose.yaml" => "",
        ".gitignore" | ".gitattributes" => "",
        ".env" | ".env.local" | ".env.development" | ".env.production" => "",
        "LICENSE" | "LICENSE.md" | "LICENSE.txt" => "",
        "README.md" | "README" | "README.txt" => "",
        _ => get_icon_by_extension(path),
    }
}

/// Get icon based on file extension
fn get_icon_by_extension(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        // Programming languages
        "rs" => "",
        "py" | "pyw" | "pyi" => "",
        "js" | "mjs" | "cjs" => "",
        "ts" | "mts" | "cts" => "",
        "jsx" => "",
        "tsx" => "",
        "go" => "",
        "java" => "",
        "c" => "",
        "cpp" | "cc" | "cxx" | "c++" => "",
        "h" | "hpp" | "hxx" | "h++" => "",
        "cs" => "",
        "rb" => "",
        "php" => "",
        "swift" => "",
        "kt" | "kts" => "",
        "scala" => "",
        "lua" => "",
        "r" => "",
        "pl" | "pm" => "",
        "sh" | "bash" | "zsh" | "fish" => "",
        "ps1" | "psm1" | "psd1" => "",
        "vim" => "",
        "hs" | "lhs" => "",
        "ex" | "exs" => "",
        "erl" | "hrl" => "",
        "clj" | "cljs" | "cljc" | "edn" => "",
        "elm" => "",
        "zig" => "",
        "nim" => "",
        "v" => "",
        "asm" | "s" => "",

        // Web
        "html" | "htm" => "",
        "css" => "",
        "scss" | "sass" => "",
        "less" => "",
        "vue" => "",
        "svelte" => "",

        // Data & Config
        "json" | "jsonc" => "",
        "yaml" | "yml" => "",
        "toml" => "",
        "xml" => "",
        "ini" | "cfg" | "conf" => "",
        "csv" => "",
        "sql" => "",

        // Documentation
        "md" | "markdown" => "",
        "txt" => "",
        "pdf" => "",
        "doc" | "docx" => "",
        "xls" | "xlsx" => "",
        "ppt" | "pptx" => "",
        "tex" | "latex" => "",
        "org" => "",
        "rst" => "",

        // Images
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "webp" => "",
        "svg" => "",
        "psd" => "",
        "ai" => "",

        // Audio
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" => "",

        // Video
        "mp4" | "avi" | "mkv" | "mov" | "wmv" | "webm" => "",

        // Archives
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => "",

        // Executables & Libraries
        "exe" | "msi" => "",
        "dll" | "so" | "dylib" => "",
        "app" => "",

        // Lock files
        "lock" => "",

        // Git
        "gitignore" | "gitattributes" | "gitmodules" => "",

        // Default
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_rust_file_icon() {
        let path = PathBuf::from("main.rs");
        assert_eq!(get_icon(&path, false, false), "");
    }

    #[test]
    fn test_python_file_icon() {
        let path = PathBuf::from("script.py");
        assert_eq!(get_icon(&path, false, false), "");
    }

    #[test]
    fn test_directory_collapsed() {
        let path = PathBuf::from("src");
        assert_eq!(get_icon(&path, true, false), "");
    }

    #[test]
    fn test_directory_expanded() {
        let path = PathBuf::from("other");
        assert_eq!(get_icon(&path, true, true), "\u{f07c}");
    }

    #[test]
    fn test_special_directory_git() {
        let path = PathBuf::from(".git");
        assert_eq!(get_icon(&path, true, false), "");
    }

    #[test]
    fn test_special_file_cargo() {
        let path = PathBuf::from("Cargo.toml");
        assert_eq!(get_icon(&path, false, false), "");
    }

    #[test]
    fn test_unknown_extension() {
        let path = PathBuf::from("file.xyz");
        assert_eq!(get_icon(&path, false, false), "");
    }

    #[test]
    fn test_image_file() {
        let path = PathBuf::from("photo.png");
        assert_eq!(get_icon(&path, false, false), "");
    }

    #[test]
    fn test_markdown_file() {
        let path = PathBuf::from("README.md");
        // README.md has special handling
        assert_eq!(get_icon(&path, false, false), "");
    }
}
