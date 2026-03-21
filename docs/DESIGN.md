# FileView - Design Document

## 1. Overview

FileViewは、ターミナルエミュレーター上で動作するシンプルなファイルビューワーである。

```
ls より便利、yazi より軽い
```

### 1.1 設計目標

- **シンプル**: 必要最小限の機能に絞る
- **高速**: 起動 <50ms、大きなディレクトリでもスムーズに動作
- **直感的**: Vimライクなキーバインドで効率的な操作
- **ゼロ設定**: インストールしてすぐ使える

### 1.2 やらないこと

- yaziとの機能競争（勝てない、勝つ必要もない）
- 汎用AI統合の乱立（FileViewは AI 駆動開発向けの中核導線に集中）

---

## 2. Architecture（アーキテクチャ）

### 2.1 ディレクトリ構成

```
src/
├── main.rs             # エントリポイント
├── lib.rs
├── app/                # アプリケーションモジュール (v1.5.0+)
│   ├── mod.rs
│   ├── config.rs       # Config - CLI引数とアプリケーション設定
│   ├── preview.rs      # PreviewState - プレビュー状態管理
│   ├── event_loop.rs   # run_app - メインイベントループ
│   └── render.rs       # RenderContext - 描画ヘルパー
├── core/
│   ├── mod.rs
│   ├── state.rs        # AppState - アプリケーション状態
│   └── mode.rs         # ViewMode - ビューモード定義
├── tree/
│   ├── mod.rs
│   ├── node.rs         # TreeEntry - ツリーノード
│   └── navigator.rs    # TreeNavigator - ツリー操作
├── action/
│   ├── mod.rs
│   ├── file.rs         # ファイル操作
│   └── clipboard.rs    # クリップボード
├── render/
│   ├── mod.rs
│   ├── tree.rs         # ツリー描画
│   ├── preview.rs      # プレビュー描画
│   ├── status.rs       # ステータスバー・ヘルプ
│   ├── icons.rs        # Nerd Fontsアイコン
│   ├── fuzzy.rs        # ファジーファインダーUI
│   └── terminal.rs     # ターミナル検出・画像プロトコル
├── handler/
│   ├── mod.rs
│   ├── key.rs          # キーイベント・KeyAction定義
│   ├── mouse.rs        # マウスイベント
│   └── action/         # アクション実行 (v1.5.0+)
│       ├── mod.rs      # dispatch関数とActionResult
│       ├── navigation.rs   # 移動アクション
│       ├── tree_ops.rs     # ツリー操作
│       ├── selection.rs    # 選択・クリップボード
│       ├── file_ops.rs     # ファイル操作
│       ├── search.rs       # 検索・ファジーファインダー
│       ├── input.rs        # 入力確認
│       ├── display.rs      # 表示・プレビュー
│       ├── bookmark.rs     # ブックマーク (v1.6.0+)
│       └── tests.rs        # アクションテスト
├── integrate/
│   ├── mod.rs
│   ├── pick.rs         # --pick モード
│   └── callback.rs     # --on-select
├── watcher/            # ファイル監視 (v1.3.0+)
│   ├── mod.rs
│   └── smart.rs        # スマートファイルウォッチャー
└── git/
    ├── mod.rs
    └── status.rs       # Git状態管理
```

### 2.2 モジュール責務

| モジュール | 責務 |
|-----------|------|
| `app` | アプリケーション設定、イベントループ、プレビュー状態管理 |
| `core` | アプリケーション状態とモード管理 |
| `tree` | ファイルツリーのデータ構造と操作 |
| `action` | ファイル操作とクリップボード |
| `render` | UI描画（ツリー、プレビュー、ファジーファインダー、画像） |
| `handler` | イベント処理（キーボード、マウス）とアクション実行 |
| `handler/action` | アクション実行の分割モジュール群 |
| `integrate` | 外部ツール連携（--pick, --on-select, --choosedir） |
| `watcher` | ファイル変更監視（展開ディレクトリの自動更新） |
| `git` | Gitリポジトリ状態の検出と表示 |

### 2.3 モード定義

```rust
pub enum ViewMode {
    Browse,                            // 通常ブラウズ
    Search { query: String },          // インクリメンタル検索
    Input { purpose: InputPurpose, buffer: String, cursor: usize },
    Confirm { action: PendingAction }, // 確認ダイアログ
    Preview { scroll: usize },         // フルスクリーンプレビュー
    FuzzyFinder { query: String, selected: usize }, // ファジーファインダー
    Help,                              // ヘルプポップアップ
    BookmarkSet,                       // ブックマーク設定待ち (v1.6.0+)
    BookmarkJump,                      // ブックマークジャンプ待ち (v1.6.0+)
    Filter { query: String },          // ファイルフィルター入力 (v1.6.0+)
}

pub enum InputPurpose {
    CreateFile,
    CreateDir,
    Rename { original: PathBuf },
}

pub enum PendingAction {
    Delete { targets: Vec<PathBuf> },
}
```

**設計ポイント:**
- 状態をenum variantに内包することで、状態管理を型安全に
- モードごとに必要なデータを明示
- 不正な状態遷移をコンパイル時に防止

---

## 3. Core Algorithms（コアアルゴリズム）

### 3.1 ツリーのフラット化

ツリー構造を画面表示用のフラットリストに変換する。

```rust
impl TreeNavigator {
    /// ツリーを可視エントリのリストに変換
    pub fn flatten(&self) -> Vec<&TreeEntry> {
        let mut entries = Vec::new();
        self.collect_visible(&self.root, &mut entries);
        entries
    }

    fn collect_visible<'a>(&'a self, entry: &'a TreeEntry, out: &mut Vec<&'a TreeEntry>) {
        out.push(entry);
        if entry.is_expanded() {
            for child in entry.children() {
                self.collect_visible(child, out);
            }
        }
    }
}
```

### 3.2 スクロール自動調整

フォーカスが画面外に出た場合、自動的にスクロール位置を調整する。

```rust
impl AppState {
    pub fn adjust_viewport(&mut self, visible_height: usize) {
        if self.focus_index < self.viewport_top {
            self.viewport_top = self.focus_index;
        } else if self.focus_index >= self.viewport_top + visible_height {
            self.viewport_top = self.focus_index - visible_height + 1;
        }
    }
}
```

### 3.3 ドラッグ&ドロップ検出

一部のターミナル（Ghostty等）はドロップされたファイルパスを高速なキー入力として送信する。`PathBuffer` がこれを検出する。

```rust
// src/handler/mouse.rs
pub struct PathBuffer {
    data: String,
    last_input: Option<Instant>,
}

impl PathBuffer {
    /// 文字をバッファに追加。入力が遅い場合はリセット
    pub fn push(&mut self, c: char);

    /// 入力が一時停止し、処理可能か判定
    pub fn is_ready(&self) -> bool;

    /// バッファから有効なファイルパスを抽出
    pub fn take_paths(&mut self) -> Vec<PathBuf>;
}
```

---

## 4. Integration Features（連携機能）

### 4.1 --pick モード

選択したパスを標準出力に出力し、シェルスクリプトから利用可能にする。

```bash
# 選択したパスを取得
selected=$(fv --pick)

# ディレクトリに移動
cd "$(fv --pick)"
```

### 4.2 --on-select コールバック

ファイル選択時に指定したコマンドを実行する。

```bash
# エディタで開く
fv --on-select "nvim {path}"

# ファイル情報を表示
fv --on-select "file {path}"
```

### 4.3 終了コード

| Code | 意味 |
|------|------|
| 0 | パス選択あり |
| 1 | キャンセル |
| 2 | エラー |

---

## 5. Git Integration (v0.2.0+)

### 5.1 ファイル状態表示

Gitリポジトリ内のファイル状態をカラーコードで表示する。

| Status | Color | 説明 |
|--------|-------|------|
| Modified | Yellow | 変更あり |
| Added | Green | ステージ済み追加 |
| Untracked | Green | 未追跡 |
| Deleted | Red | 削除 |
| Renamed | Cyan | リネーム |
| Ignored | DarkGray | .gitignore対象 |
| Conflict | Magenta | コンフリクト |

### 5.2 ブランチ表示

ステータスバーに現在のブランチ名を表示する。

```
📁 src/main.rs | 🌿 main | 42 items
```

### 5.3 設計

```rust
pub struct GitStatus {
    repo_root: PathBuf,
    statuses: HashMap<PathBuf, FileStatus>,
    branch: Option<String>,
}

impl GitStatus {
    /// リポジトリを検出し状態をキャッシュ
    pub fn detect(path: &Path) -> Option<Self>;

    /// ファイルの状態を取得
    pub fn get_status(&self, path: &Path) -> FileStatus;

    /// 状態を更新（ファイル操作後）
    pub fn refresh(&mut self);
}
```

---

## 6. Key Bindings

| Key | Action |
|-----|--------|
| `j` / `↓` | 下移動 |
| `k` / `↑` | 上移動 |
| `l` / `→` / `Enter` | 展開 / 確定 |
| `h` / `←` | 折りたたみ / 親へ |
| `g` | 先頭へ |
| `G` | 末尾へ |
| `Space` | 選択切替（マルチセレクト） |
| `y` | コピー |
| `d` | カット |
| `p` | ペースト |
| `D` | 削除 |
| `r` | リネーム |
| `a` | 新規ファイル |
| `A` | 新規フォルダ |
| `/` | インクリメンタル検索 |
| `Ctrl+P` | ファジーファインダー |
| `c` | パスをクリップボードへ |
| `C` | ファイル名をクリップボードへ |
| `P` | サイドプレビュー切替 |
| `o` | フルスクリーンプレビュー |
| `Tab` | フォーカス切替（ツリー/プレビュー） |
| `.` | 隠しファイル切替 |
| `m1-9` | ブックマーク設定 (v1.6.0+) |
| `'1-9` | ブックマークジャンプ (v1.6.0+) |
| `F` | ファイルフィルター (v1.6.0+) |
| `?` | ヘルプ |
| `q` | 終了 |
| `Q` | 終了してcd（--choosedir時） |

---

## 7. Technology Stack

| Category | Choice |
|----------|--------|
| Language | Rust |
| TUI | ratatui |
| Terminal | crossterm |
| Clipboard | arboard |
| Image | image, ratatui-image |
| Fuzzy Match | nucleo-matcher |
| Error | anyhow |

---

## 8. Design Principles

1. **シンプルさを保つ**: 機能追加より安定性を優先
2. **型安全性**: Rustの型システムを活用した安全な設計
3. **モジュール性**: 責務を明確に分離し、テスト容易性を確保
4. **外部連携**: --pick, --on-select, --choosedirでシェルと連携
5. **ゼロ設定**: デフォルトでそのまま動作、必要に応じて設定ファイルでカスタマイズ可能

---

## 9. Non-Goals（やらないこと）

以下は意図的にスコープ外とする:

| 機能 | 理由 |
|------|------|
| タブ/分割ウィンドウ | tmux/ターミナルの仕事 |
| 組み込みエディタ | vim/nvimの仕事 |
| リモートファイル | スコープ外 |
| アーカイブ操作 | スコープ外 |
