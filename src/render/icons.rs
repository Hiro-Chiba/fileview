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

    // ==========================================================================
    // Programming Language Icons
    // ==========================================================================

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
    fn test_javascript_file_icon() {
        assert_eq!(get_icon(&PathBuf::from("app.js"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("module.mjs"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("common.cjs"), false, false), "");
    }

    #[test]
    fn test_typescript_file_icon() {
        assert_eq!(get_icon(&PathBuf::from("app.ts"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("module.mts"), false, false), "");
    }

    #[test]
    fn test_jsx_tsx_file_icons() {
        assert_eq!(get_icon(&PathBuf::from("Component.jsx"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("Component.tsx"), false, false), "");
    }

    #[test]
    fn test_go_file_icon() {
        let path = PathBuf::from("main.go");
        assert_eq!(get_icon(&path, false, false), "");
    }

    #[test]
    fn test_java_file_icon() {
        let path = PathBuf::from("Main.java");
        assert_eq!(get_icon(&path, false, false), "");
    }

    #[test]
    fn test_c_file_icon() {
        let path = PathBuf::from("main.c");
        assert_eq!(get_icon(&path, false, false), "");
    }

    #[test]
    fn test_cpp_file_icon() {
        assert_eq!(get_icon(&PathBuf::from("main.cpp"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("main.cc"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("main.cxx"), false, false), "");
    }

    #[test]
    fn test_header_file_icon() {
        assert_eq!(get_icon(&PathBuf::from("header.h"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("header.hpp"), false, false), "");
    }

    #[test]
    fn test_shell_file_icon() {
        assert_eq!(get_icon(&PathBuf::from("script.sh"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("script.bash"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("script.zsh"), false, false), "");
    }

    // ==========================================================================
    // Web Files
    // ==========================================================================

    #[test]
    fn test_html_file_icon() {
        assert_eq!(get_icon(&PathBuf::from("index.html"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("page.htm"), false, false), "");
    }

    #[test]
    fn test_css_file_icon() {
        assert_eq!(get_icon(&PathBuf::from("style.css"), false, false), "");
    }

    #[test]
    fn test_scss_sass_file_icon() {
        assert_eq!(get_icon(&PathBuf::from("style.scss"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("style.sass"), false, false), "");
    }

    #[test]
    fn test_vue_svelte_file_icons() {
        assert_eq!(get_icon(&PathBuf::from("App.vue"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("App.svelte"), false, false), "");
    }

    // ==========================================================================
    // Config & Data Files
    // ==========================================================================

    #[test]
    fn test_json_file_icon() {
        assert_eq!(get_icon(&PathBuf::from("config.json"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("tsconfig.jsonc"), false, false), "");
    }

    #[test]
    fn test_yaml_file_icon() {
        assert_eq!(get_icon(&PathBuf::from("config.yaml"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("config.yml"), false, false), "");
    }

    #[test]
    fn test_toml_file_icon() {
        assert_eq!(get_icon(&PathBuf::from("config.toml"), false, false), "");
    }

    #[test]
    fn test_xml_file_icon() {
        assert_eq!(get_icon(&PathBuf::from("config.xml"), false, false), "");
    }

    // ==========================================================================
    // Media Files
    // ==========================================================================

    #[test]
    fn test_image_file() {
        assert_eq!(get_icon(&PathBuf::from("photo.png"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("photo.jpg"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("photo.jpeg"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("photo.gif"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("photo.webp"), false, false), "");
    }

    #[test]
    fn test_svg_file_icon() {
        assert_eq!(get_icon(&PathBuf::from("logo.svg"), false, false), "");
    }

    #[test]
    fn test_audio_file_icon() {
        assert_eq!(get_icon(&PathBuf::from("song.mp3"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("song.wav"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("song.flac"), false, false), "");
    }

    #[test]
    fn test_video_file_icon() {
        assert_eq!(get_icon(&PathBuf::from("video.mp4"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("video.mkv"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("video.webm"), false, false), "");
    }

    // ==========================================================================
    // Archive Files
    // ==========================================================================

    #[test]
    fn test_archive_file_icon() {
        assert_eq!(get_icon(&PathBuf::from("archive.zip"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("archive.tar"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("archive.gz"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("archive.7z"), false, false), "");
    }

    // ==========================================================================
    // Special Directories
    // ==========================================================================

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
    fn test_generic_directory_collapsed() {
        let path = PathBuf::from("mydir");
        assert_eq!(get_icon(&path, true, false), "\u{f07b}");
    }

    #[test]
    fn test_special_directory_git() {
        let path = PathBuf::from(".git");
        assert_eq!(get_icon(&path, true, false), "");
    }

    #[test]
    fn test_special_directory_node_modules() {
        let path = PathBuf::from("node_modules");
        assert_eq!(get_icon(&path, true, false), "");
    }

    #[test]
    fn test_special_directory_target() {
        assert_eq!(get_icon(&PathBuf::from("target"), true, false), "");
        assert_eq!(get_icon(&PathBuf::from("build"), true, false), "");
        assert_eq!(get_icon(&PathBuf::from("dist"), true, false), "");
    }

    #[test]
    fn test_special_directory_tests() {
        assert_eq!(get_icon(&PathBuf::from("test"), true, false), "");
        assert_eq!(get_icon(&PathBuf::from("tests"), true, false), "");
        assert_eq!(get_icon(&PathBuf::from("__tests__"), true, false), "");
    }

    #[test]
    fn test_special_directory_docs() {
        assert_eq!(get_icon(&PathBuf::from("docs"), true, false), "");
        assert_eq!(get_icon(&PathBuf::from("doc"), true, false), "");
    }

    #[test]
    fn test_special_directory_github() {
        let path = PathBuf::from(".github");
        assert_eq!(get_icon(&path, true, false), "");
    }

    #[test]
    fn test_special_directory_vscode() {
        let path = PathBuf::from(".vscode");
        assert_eq!(get_icon(&path, true, false), "");
    }

    // ==========================================================================
    // Special Files
    // ==========================================================================

    #[test]
    fn test_special_file_cargo() {
        assert_eq!(get_icon(&PathBuf::from("Cargo.toml"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("Cargo.lock"), false, false), "");
    }

    #[test]
    fn test_special_file_package_json() {
        assert_eq!(get_icon(&PathBuf::from("package.json"), false, false), "");
        assert_eq!(
            get_icon(&PathBuf::from("package-lock.json"), false, false),
            ""
        );
    }

    #[test]
    fn test_special_file_makefile() {
        assert_eq!(get_icon(&PathBuf::from("Makefile"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("CMakeLists.txt"), false, false), "");
    }

    #[test]
    fn test_special_file_dockerfile() {
        assert_eq!(get_icon(&PathBuf::from("Dockerfile"), false, false), "");
        assert_eq!(
            get_icon(&PathBuf::from("docker-compose.yml"), false, false),
            ""
        );
    }

    #[test]
    fn test_special_file_gitignore() {
        assert_eq!(get_icon(&PathBuf::from(".gitignore"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from(".gitattributes"), false, false), "");
    }

    #[test]
    fn test_special_file_env() {
        assert_eq!(get_icon(&PathBuf::from(".env"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from(".env.local"), false, false), "");
        assert_eq!(
            get_icon(&PathBuf::from(".env.development"), false, false),
            ""
        );
    }

    #[test]
    fn test_special_file_license() {
        assert_eq!(get_icon(&PathBuf::from("LICENSE"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("LICENSE.md"), false, false), "");
    }

    #[test]
    fn test_special_file_readme() {
        assert_eq!(get_icon(&PathBuf::from("README.md"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("README"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("README.txt"), false, false), "");
    }

    // ==========================================================================
    // Edge Cases
    // ==========================================================================

    #[test]
    fn test_unknown_extension() {
        let path = PathBuf::from("file.xyz");
        assert_eq!(get_icon(&path, false, false), "");
    }

    #[test]
    fn test_case_insensitive_extension() {
        // Extension matching should be case-insensitive
        assert_eq!(get_icon(&PathBuf::from("file.RS"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("file.Py"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("file.JSON"), false, false), "");
    }

    #[test]
    fn test_no_extension() {
        // File without extension should use default icon
        let path = PathBuf::from("Makefile_backup");
        assert_eq!(get_icon(&path, false, false), "");
    }

    #[test]
    fn test_multiple_dots_in_filename() {
        // Should use the last extension
        assert_eq!(
            get_icon(&PathBuf::from("file.test.js"), false, false),
            ""
        );
        assert_eq!(
            get_icon(&PathBuf::from("app.config.json"), false, false),
            ""
        );
        assert_eq!(
            get_icon(&PathBuf::from("style.module.css"), false, false),
            ""
        );
    }

    #[test]
    fn test_hidden_file_with_extension() {
        assert_eq!(get_icon(&PathBuf::from(".bashrc"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from(".zshrc"), false, false), "");
    }

    #[test]
    fn test_markdown_file() {
        // Non-README markdown file
        assert_eq!(get_icon(&PathBuf::from("CHANGELOG.md"), false, false), "");
        assert_eq!(
            get_icon(&PathBuf::from("CONTRIBUTING.md"), false, false),
            ""
        );
    }

    #[test]
    fn test_lock_file_icon() {
        assert_eq!(get_icon(&PathBuf::from("yarn.lock"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("pnpm-lock.yaml"), false, false), "");
    }

    #[test]
    fn test_executable_file_icon() {
        assert_eq!(get_icon(&PathBuf::from("app.exe"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("installer.msi"), false, false), "");
    }

    #[test]
    fn test_library_file_icon() {
        assert_eq!(get_icon(&PathBuf::from("lib.dll"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("lib.so"), false, false), "");
        assert_eq!(get_icon(&PathBuf::from("lib.dylib"), false, false), "");
    }
}
