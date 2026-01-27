//! Nerd Fonts icon mappings for files and directories
//! Based on yazi file manager's icon system with additional customizations

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

    // Special directories (sorted alphabetically for maintainability)
    match name.to_lowercase().as_str() {
        // Hidden/config directories
        ".cache" => "\u{f0a0}",  //
        ".cargo" => "\u{e7a8}",  //  (Rust)
        ".config" => "\u{e5fc}", //
        ".git" => "\u{f1d3}",    //
        ".github" => "\u{f408}", //
        ".idea" => "\u{e7b5}",   //  (IntelliJ)
        ".npm" => "\u{e71e}",    //
        ".rustup" => "\u{e7a8}", //  (Rust)
        ".ssh" => "\u{f084}",    //
        ".vscode" => "\u{e70c}", //

        // Standard directories
        "bin" | "sbin" => "\u{f489}",                         //
        "build" | "dist" | "out" | "output" => "\u{f487}",    //
        "config" | "configs" | "configuration" => "\u{e5fc}", //
        "desktop" => "\u{f108}",                              //
        "development" | "dev" | "projects" => "\u{e5fc}",     //
        "doc" | "docs" | "documentation" => "\u{f02d}",       //
        "documents" => "\u{f02d}",                            //
        "downloads" => "\u{f019}",                            //
        "fonts" => "\u{f031}",                                //
        "images" | "img" | "imgs" => "\u{f03e}",              //
        "lib" | "libs" | "library" => "\u{f121}",             //
        "media" => "\u{f03e}",                                //
        "movies" | "videos" | "video" => "\u{f008}",          //
        "music" | "audio" | "sounds" => "\u{f001}",           //
        "node_modules" => "\u{e718}",                         //
        "packages" => "\u{f487}",                             //
        "photos" | "pictures" => "\u{f03e}",                  //
        "public" => "\u{f0ac}",                               //
        "scripts" | "script" => "\u{f489}",                   //
        "src" | "source" | "sources" => "\u{e5fc}",           //
        "target" => "\u{f487}",                               //  (Rust build)
        "test" | "tests" | "__tests__" | "spec" | "specs" => "\u{f0c3}", //
        "tmp" | "temp" | "temporary" => "\u{f252}",           //
        "vendor" | "vendors" => "\u{f487}",                   //
        "www" | "wwwroot" | "htdocs" => "\u{f0ac}",           //

        // Default folder icons (open/closed)
        _ => {
            if expanded {
                "\u{f07c}" //  (folder open)
            } else {
                "\u{f07b}" //  (folder closed)
            }
        }
    }
}

/// Get icon for a file based on extension or filename
fn get_file_icon(path: &Path) -> &'static str {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let name_lower = name.to_lowercase();

    // Check special filenames first (exact match)
    if let Some(icon) = get_special_file_icon(&name_lower, name) {
        return icon;
    }

    // Then check by extension
    get_icon_by_extension(path)
}

/// Get icon for special filenames
fn get_special_file_icon(name_lower: &str, _name: &str) -> Option<&'static str> {
    Some(match name_lower {
        // Build & Package
        "cargo.toml" | "cargo.lock" => "\u{e7a8}", //  (Rust)
        "package.json" | "package-lock.json" => "\u{e71e}", //
        "yarn.lock" => "\u{e6a7}",                 //
        "pnpm-lock.yaml" => "\u{e71e}",            //
        "composer.json" | "composer.lock" => "\u{e608}", //  (PHP)
        "gemfile" | "gemfile.lock" => "\u{e791}",  //  (Ruby)
        "requirements.txt" | "setup.py" | "pyproject.toml" => "\u{e73c}", //  (Python)
        "go.mod" | "go.sum" => "\u{e627}",         //  (Go)
        "mix.exs" | "mix.lock" => "\u{e62d}",      //  (Elixir)

        // Build Tools
        "makefile" | "gnumakefile" => "\u{e673}",     //
        "cmakelists.txt" => "\u{e673}",               //
        "justfile" => "\u{e673}",                     //
        "rakefile" => "\u{e791}",                     //  (Ruby)
        "gulpfile.js" | "gruntfile.js" => "\u{e74e}", //  (JS)

        // Docker
        "dockerfile" => "\u{f308}", // 󰡨
        "docker-compose.yml" | "docker-compose.yaml" => "\u{f308}",
        "compose.yml" | "compose.yaml" => "\u{f308}",
        ".dockerignore" => "\u{f308}",

        // Git
        ".gitignore" => "\u{f1d3}", //
        ".gitattributes" => "\u{f1d3}",
        ".gitmodules" => "\u{f1d3}",
        ".gitconfig" => "\u{f1d3}",
        ".gitkeep" => "\u{f1d3}",

        // CI/CD
        ".travis.yml" => "\u{e77e}",    //
        ".gitlab-ci.yml" => "\u{f296}", //
        "jenkinsfile" => "\u{e767}",    //

        // Config files
        ".env" | ".env.local" | ".env.development" | ".env.production" | ".env.example" => {
            "\u{f462}"
        } //
        ".editorconfig" => "\u{e652}", //
        ".prettierrc" | ".prettierrc.json" | ".prettierrc.yml" | "prettier.config.js" => "\u{e6b4}", //
        ".eslintrc" | ".eslintrc.js" | ".eslintrc.json" | "eslint.config.js" => "\u{e655}", //
        ".stylelintrc" => "\u{e749}",                                                       //
        "tsconfig.json" | "jsconfig.json" => "\u{e628}",                                    //  (TS)
        "vite.config.js" | "vite.config.ts" => "\u{e6b5}",                                  //
        "webpack.config.js" => "\u{f487}",                                                  //
        "rollup.config.js" => "\u{f487}",
        "babel.config.js" | ".babelrc" => "\u{e661}", //

        // Shell
        ".bashrc" | ".bash_profile" | ".bash_history" => "\u{e795}", //
        ".zshrc" | ".zprofile" | ".zsh_history" => "\u{e795}",
        ".profile" => "\u{e795}",

        // Vim/Neovim
        ".vimrc" | ".gvimrc" | "_vimrc" => "\u{e62b}", //
        "init.vim" | "init.lua" => "\u{e62b}",

        // Documentation
        "readme" | "readme.md" | "readme.txt" | "readme.rst" => "\u{f48a}", // 󰂺
        "license" | "license.md" | "license.txt" | "licence" => "\u{f0219}", // 󰈙
        "changelog" | "changelog.md" | "changes" | "history.md" => "\u{f7d9}", //
        "contributing" | "contributing.md" => "\u{f4d4}",                   //
        "authors" | "authors.md" | "contributors" => "\u{f0c0}",            //
        "code_of_conduct.md" => "\u{f4d4}",
        "security.md" => "\u{f084}", //

        // Lock files
        "flake.lock" | "flake.nix" => "\u{f313}", //

        // Kubernetes
        "kubernetes.yml" | "kubernetes.yaml" => "\u{f10fe}", // 󱃾
        "k8s.yml" | "k8s.yaml" => "\u{f10fe}",

        // Terraform
        "main.tf" | "variables.tf" | "outputs.tf" | "providers.tf" => "\u{e69a}", //

        // Other
        ".npmrc" | ".npmignore" => "\u{e71e}",  //
        ".yarnrc" | ".yarnclean" => "\u{e6a7}", //
        ".nvmrc" => "\u{e718}",                 //
        "robots.txt" => "\u{f06a}",             //
        "favicon.ico" => "\u{f245}",            //
        "manifest.json" => "\u{e60b}",          //
        ".htaccess" => "\u{e60b}",
        "netlify.toml" | "vercel.json" => "\u{f233}", //

        _ => return None,
    })
}

/// Get icon based on file extension
fn get_icon_by_extension(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        // ==========================================================================
        // Programming Languages
        // ==========================================================================

        // Rust
        "rs" => "\u{e7a8}", //

        // Python
        "py" | "pyw" | "pyi" | "pyx" | "pxd" => "\u{e73c}", //
        "pyc" | "pyo" => "\u{e73c}",
        "ipynb" => "\u{e678}", //  (Jupyter)

        // JavaScript & TypeScript
        "js" | "mjs" | "cjs" => "\u{e74e}", //
        "ts" | "mts" | "cts" => "\u{e628}", //
        "jsx" => "\u{e7ba}",                //  (React)
        "tsx" => "\u{e7ba}",                //  (React + TS)

        // Go
        "go" => "\u{e627}",  //
        "mod" => "\u{e627}", // go.mod

        // Java & JVM
        "java" => "\u{e738}", //
        "jar" | "war" | "ear" => "\u{e738}",
        "kt" | "kts" => "\u{e634}",                    //  (Kotlin)
        "scala" | "sc" => "\u{e737}",                  //  (Scala)
        "groovy" | "gvy" | "gy" | "gsh" => "\u{e775}", //
        "clj" | "cljs" | "cljc" | "edn" => "\u{e76a}", //  (Clojure)

        // C Family
        "c" => "\u{e61e}",                          //
        "cpp" | "cc" | "cxx" | "c++" => "\u{e61d}", //
        "h" | "hpp" | "hxx" | "h++" | "hh" => "\u{e61d}",
        "cs" => "\u{f031b}",      // 󰌛 (C#)
        "m" | "mm" => "\u{e61e}", //  (Objective-C)

        // Swift
        "swift" => "\u{e755}", //

        // Ruby
        "rb" | "ruby" | "rake" | "erb" | "slim" | "haml" => "\u{e791}", //

        // PHP
        "php" | "phtml" | "php3" | "php4" | "php5" | "php7" | "php8" => "\u{e608}", //
        "blade.php" => "\u{e608}",

        // Perl
        "pl" | "pm" | "pod" | "t" | "perl" => "\u{e769}", //

        // Shell
        "sh" | "bash" | "zsh" | "fish" | "ksh" | "csh" | "tcsh" => "\u{e795}", //
        "ps1" | "psm1" | "psd1" => "\u{e70f}",                                 //  (PowerShell)
        "bat" | "cmd" => "\u{e629}",                                           //

        // Lua
        "lua" => "\u{e620}", //

        // R
        "r" | "rmd" | "rproj" => "\u{e68a}", //

        // Haskell
        "hs" | "lhs" => "\u{e777}", //

        // Elixir & Erlang
        "ex" | "exs" => "\u{e62d}",  //  (Elixir)
        "erl" | "hrl" => "\u{e7b1}", //  (Erlang)

        // Elm
        "elm" => "\u{e62c}", //

        // Zig
        "zig" => "\u{e6a9}", //

        // Nim
        "nim" | "nims" | "nimble" => "\u{e677}", //

        // V
        "v" => "\u{e6ac}", //

        // OCaml & F#
        "ml" | "mli" => "\u{e67a}",         //  (OCaml)
        "fs" | "fsi" | "fsx" => "\u{e7a7}", //  (F#)

        // D
        "d" => "\u{e7af}", //

        // Crystal
        "cr" => "\u{e7a3}", //

        // Julia
        "jl" => "\u{e624}", //

        // Dart & Flutter
        "dart" => "\u{e798}", //

        // Assembly
        "asm" | "s" | "S" => "\u{e6ab}", //

        // WASM
        "wasm" | "wat" => "\u{e6a1}", //

        // ==========================================================================
        // Web Technologies
        // ==========================================================================

        // HTML
        "html" | "htm" | "xhtml" => "\u{e736}", //

        // CSS
        "css" => "\u{e749}",             //
        "scss" | "sass" => "\u{e603}",   //
        "less" => "\u{e758}",            //
        "styl" | "stylus" => "\u{e600}", //

        // Vue.js
        "vue" => "\u{e6a0}", //

        // Svelte
        "svelte" => "\u{e697}", //

        // Astro
        "astro" => "\u{e6b4}", //

        // Angular
        "angular" | "component.ts" => "\u{e753}", //

        // ==========================================================================
        // Data & Config
        // ==========================================================================

        // JSON
        "json" | "jsonc" | "json5" => "\u{e60b}", //

        // YAML
        "yaml" | "yml" => "\u{e6a8}", //

        // TOML
        "toml" => "\u{e6b2}", //

        // XML
        "xml" | "plist" | "xsd" | "xsl" | "xslt" | "rss" | "atom" => "\u{e796}", //

        // INI
        "ini" | "cfg" | "conf" | "config" => "\u{e615}", //

        // CSV & Data
        "csv" | "tsv" => "\u{f0ce}", //

        // SQL
        "sql" | "pgsql" | "mysql" => "\u{e706}", //

        // GraphQL
        "graphql" | "gql" => "\u{e662}", //

        // Protocol Buffers
        "proto" => "\u{e6a5}", //

        // ==========================================================================
        // Documentation
        // ==========================================================================

        // Markdown
        "md" | "markdown" | "mdown" | "mkd" | "mdx" => "\u{e73e}", //

        // Plain text
        "txt" | "text" => "\u{f0f6}", //

        // reStructuredText
        "rst" => "\u{e73e}", //

        // Org mode
        "org" => "\u{e633}", //

        // AsciiDoc
        "adoc" | "asciidoc" => "\u{e606}", //

        // LaTeX
        "tex" | "latex" | "bib" | "sty" | "cls" => "\u{e69b}", //

        // PDF
        "pdf" => "\u{f1c1}", //

        // Office documents
        "doc" | "docx" | "odt" => "\u{f1c2}", //
        "xls" | "xlsx" | "ods" => "\u{f1c3}", //
        "ppt" | "pptx" | "odp" => "\u{f1c4}", //

        // eBooks
        "epub" | "mobi" | "azw" | "azw3" => "\u{f02d}", //

        // ==========================================================================
        // Images
        // ==========================================================================

        // Raster images
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "webp" | "avif" | "heic" | "heif" => {
            "\u{f03e}"
        } //
        "tiff" | "tif" | "raw" | "cr2" | "nef" | "arw" | "dng" => "\u{f03e}",

        // Vector images
        "svg" => "\u{f1c5}",        //
        "eps" | "ai" => "\u{e7b4}", //  (Adobe Illustrator)

        // Design files
        "psd" => "\u{e7b8}",           //  (Photoshop)
        "xd" => "\u{e7b5}",            //  (Adobe XD)
        "sketch" => "\u{e7b6}",        //
        "fig" | "figma" => "\u{e7b5}", //

        // 3D
        "obj" | "stl" | "fbx" | "blend" | "3ds" | "dae" | "gltf" | "glb" => "\u{f1b2}", //

        // ==========================================================================
        // Audio
        // ==========================================================================
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" | "m4a" | "opus" | "aiff" => "\u{f001}", //
        "mid" | "midi" => "\u{f001}",

        // ==========================================================================
        // Video
        // ==========================================================================
        "mp4" | "avi" | "mkv" | "mov" | "wmv" | "webm" | "flv" | "m4v" | "mpeg" | "mpg" => {
            "\u{f008}"
        } //
        "3gp" | "3g2" | "ogv" | "vob" => "\u{f008}",

        // ==========================================================================
        // Archives
        // ==========================================================================
        "zip" | "tar" | "gz" | "bz2" | "xz" | "lz" | "lzma" | "lzo" => "\u{f1c6}", //
        "7z" | "rar" | "cab" | "iso" | "dmg" | "pkg" | "deb" | "rpm" => "\u{f1c6}",
        "tgz" | "tbz2" | "txz" | "zst" | "zstd" => "\u{f1c6}",

        // ==========================================================================
        // Executables & Libraries
        // ==========================================================================
        "exe" | "msi" | "com" | "scr" => "\u{f085}", //  (Windows)
        "app" | "bundle" => "\u{f0ac}",              //  (Mac)
        "dll" | "so" | "dylib" | "a" | "lib" => "\u{f121}", //
        "apk" => "\u{e70e}",                         //  (Android)
        "ipa" => "\u{e711}",                         //  (iOS)

        // ==========================================================================
        // Security & Certificates
        // ==========================================================================
        "pem" | "crt" | "cer" | "key" | "pub" => "\u{f084}", //
        "p12" | "pfx" | "jks" | "keystore" => "\u{f084}",
        "gpg" | "pgp" | "asc" => "\u{f084}",

        // ==========================================================================
        // Misc
        // ==========================================================================

        // Lock files (generic)
        "lock" => "\u{f023}", //

        // Git
        "gitignore" | "gitattributes" | "gitmodules" => "\u{f1d3}", //

        // Log files
        "log" => "\u{f0f6}", //

        // Backup files
        "bak" | "backup" | "old" | "orig" | "swp" => "\u{f0c5}", //

        // Diff & Patch
        "diff" | "patch" => "\u{f440}", //

        // Fonts
        "ttf" | "otf" | "woff" | "woff2" | "eot" => "\u{f031}", //

        // Database
        "db" | "sqlite" | "sqlite3" | "mdb" | "accdb" => "\u{f1c0}", //

        // Virtual machines
        "vmdk" | "vdi" | "vhd" | "qcow2" | "ova" | "ovf" => "\u{f233}", //

        // Nix
        "nix" => "\u{f313}", //

        // Default file icon
        _ => "\u{f15b}", //
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ==========================================================================
    // Directory Tests
    // ==========================================================================

    #[test]
    fn test_special_directories() {
        assert_eq!(get_icon(&PathBuf::from(".git"), true, false), "\u{f1d3}");
        assert_eq!(get_icon(&PathBuf::from(".config"), true, false), "\u{e5fc}");
        assert_eq!(
            get_icon(&PathBuf::from("node_modules"), true, false),
            "\u{e718}"
        );
        assert_eq!(get_icon(&PathBuf::from("src"), true, false), "\u{e5fc}");
        assert_eq!(
            get_icon(&PathBuf::from("Downloads"), true, false),
            "\u{f019}"
        );
    }

    #[test]
    fn test_directory_open_close() {
        assert_eq!(get_icon(&PathBuf::from("mydir"), true, false), "\u{f07b}");
        assert_eq!(get_icon(&PathBuf::from("mydir"), true, true), "\u{f07c}");
    }

    // ==========================================================================
    // File Extension Tests
    // ==========================================================================

    #[test]
    fn test_programming_languages() {
        assert_eq!(
            get_icon(&PathBuf::from("main.rs"), false, false),
            "\u{e7a8}"
        );
        assert_eq!(
            get_icon(&PathBuf::from("script.py"), false, false),
            "\u{e73c}"
        );
        assert_eq!(get_icon(&PathBuf::from("app.js"), false, false), "\u{e74e}");
        assert_eq!(get_icon(&PathBuf::from("app.ts"), false, false), "\u{e628}");
        assert_eq!(
            get_icon(&PathBuf::from("main.go"), false, false),
            "\u{e627}"
        );
    }

    #[test]
    fn test_react_files() {
        assert_eq!(
            get_icon(&PathBuf::from("Component.jsx"), false, false),
            "\u{e7ba}"
        );
        assert_eq!(
            get_icon(&PathBuf::from("Component.tsx"), false, false),
            "\u{e7ba}"
        );
    }

    #[test]
    fn test_web_files() {
        assert_eq!(
            get_icon(&PathBuf::from("index.html"), false, false),
            "\u{e736}"
        );
        assert_eq!(
            get_icon(&PathBuf::from("style.css"), false, false),
            "\u{e749}"
        );
        assert_eq!(
            get_icon(&PathBuf::from("style.scss"), false, false),
            "\u{e603}"
        );
    }

    #[test]
    fn test_config_files() {
        assert_eq!(
            get_icon(&PathBuf::from("config.json"), false, false),
            "\u{e60b}"
        );
        assert_eq!(
            get_icon(&PathBuf::from("config.yaml"), false, false),
            "\u{e6a8}"
        );
        assert_eq!(
            get_icon(&PathBuf::from("config.toml"), false, false),
            "\u{e6b2}"
        );
    }

    // ==========================================================================
    // Special File Tests
    // ==========================================================================

    #[test]
    fn test_special_files() {
        assert_eq!(
            get_icon(&PathBuf::from("Cargo.toml"), false, false),
            "\u{e7a8}"
        );
        assert_eq!(
            get_icon(&PathBuf::from("package.json"), false, false),
            "\u{e71e}"
        );
        assert_eq!(
            get_icon(&PathBuf::from("Dockerfile"), false, false),
            "\u{f308}"
        );
        assert_eq!(
            get_icon(&PathBuf::from(".gitignore"), false, false),
            "\u{f1d3}"
        );
        assert_eq!(
            get_icon(&PathBuf::from("README.md"), false, false),
            "\u{f48a}"
        );
    }

    #[test]
    fn test_shell_config_files() {
        assert_eq!(
            get_icon(&PathBuf::from(".bashrc"), false, false),
            "\u{e795}"
        );
        assert_eq!(get_icon(&PathBuf::from(".zshrc"), false, false), "\u{e795}");
    }

    // ==========================================================================
    // Media File Tests
    // ==========================================================================

    #[test]
    fn test_image_files() {
        assert_eq!(
            get_icon(&PathBuf::from("photo.png"), false, false),
            "\u{f03e}"
        );
        assert_eq!(
            get_icon(&PathBuf::from("photo.jpg"), false, false),
            "\u{f03e}"
        );
        assert_eq!(
            get_icon(&PathBuf::from("logo.svg"), false, false),
            "\u{f1c5}"
        );
    }

    #[test]
    fn test_audio_video_files() {
        assert_eq!(
            get_icon(&PathBuf::from("song.mp3"), false, false),
            "\u{f001}"
        );
        assert_eq!(
            get_icon(&PathBuf::from("video.mp4"), false, false),
            "\u{f008}"
        );
    }

    #[test]
    fn test_archive_files() {
        assert_eq!(
            get_icon(&PathBuf::from("archive.zip"), false, false),
            "\u{f1c6}"
        );
        assert_eq!(
            get_icon(&PathBuf::from("archive.tar.gz"), false, false),
            "\u{f1c6}"
        );
    }

    // ==========================================================================
    // Edge Cases
    // ==========================================================================

    #[test]
    fn test_case_insensitivity() {
        assert_eq!(
            get_icon(&PathBuf::from("FILE.RS"), false, false),
            "\u{e7a8}"
        );
        assert_eq!(
            get_icon(&PathBuf::from("README.MD"), false, false),
            "\u{f48a}"
        );
        assert_eq!(
            get_icon(&PathBuf::from("DOCKERFILE"), false, false),
            "\u{f308}"
        );
    }

    #[test]
    fn test_unknown_extension() {
        assert_eq!(
            get_icon(&PathBuf::from("file.xyz"), false, false),
            "\u{f15b}"
        );
    }

    #[test]
    fn test_default_file() {
        assert_eq!(
            get_icon(&PathBuf::from("noextension"), false, false),
            "\u{f15b}"
        );
    }
}
