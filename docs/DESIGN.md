# FileView - Design Document

## 1. Overview

FileViewは、ターミナルエミュレーター上で動作するVSCode風のミニマルファイルツリーUIである。
Ghostty等のモダンターミナルでの使用を想定し、**軽量・高速・直感操作**を設計思想の中核とする。

### 1.1 ベースプロジェクト

本プロジェクトは [myfile](../myfile) の設計・実装を参考にしている。
myfileの優れたアーキテクチャをベースに、書き方を変えつつ同等の機能を実現する。

### 1.2 myfileから継承する設計

| 要素 | 内容 |
|------|------|
| アーキテクチャ | **モード駆動型 State Machine** |
| ツリー構造 | **フラット化リスト（flat_list）同期** |
| I/O処理 | **同期処理**（シンプルさ優先） |
| 画像表示 | **半ブロック文字（▀）** |
| キーバインド | **Vim風**（j/k/h/l） |

### 1.3 fileviewでの追加・変更

| 要素 | 変更内容 |
|------|----------|
| Git統合 | **除外**（ミニマル設計） |
| 外部コマンド | **除外** → `--pick`/`--on-select`で代替 |
| パス連携 | **追加**（stdout出力、コールバック） |
| 書き方 | 変数名・関数名を一部変更 |

---

## 2. Architecture（myfileベース）

### 2.1 モード駆動型 State Machine

myfileと同様に、`InputMode` enumで状態を管理する。

```rust
pub enum InputMode {
    Normal,           // 通常操作
    Search,           // インクリメンタル検索
    Rename,           // ファイル/フォルダ名変更
    NewFile,          // ファイル作成
    NewDir,           // フォルダ作成
    Confirm(ConfirmAction),  // 削除確認
    Preview,          // フルスクリーンプレビュー
}
```

### 2.2 フラット化ツリー

myfileと同様に、ツリーをフラット化リストとして管理する。

```rust
pub struct FileTree {
    pub root: FileNode,           // ツリー構造
    pub flat_list: Vec<usize>,    // フラット化インデックス
    nodes: Vec<FileNode>,         // フラット化ノード
    pub show_hidden: bool,
}
```

**利点：**
- O(1) でインデックスアクセス
- スクロール・選択が高速
- 実装がシンプル

### 2.3 全体構成

```
┌─────────────────────────────────────────────────────┐
│                    FileView                         │
├─────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐ │
│  │    App      │  │  FileTree   │  │     UI      │ │
│  │   (State)   │◄─┤  (Data)     │◄─┤  (Render)   │ │
│  └─────────────┘  └─────────────┘  └─────────────┘ │
│         │                │                │        │
│         ▼                ▼                ▼        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐ │
│  │   Input     │  │  FileOps    │  │  Preview    │ │
│  │  (Events)   │  │  (CRUD)     │  │  (Display)  │ │
│  └─────────────┘  └─────────────┘  └─────────────┘ │
└─────────────────────────────────────────────────────┘
```

---

## 3. Technology Stack

myfileと同じクレートを使用する。

| Category | Choice | Rationale |
|----------|--------|-----------|
| Language | Rust | メモリ安全性、高速性 |
| TUI Framework | ratatui | myfileと同様 |
| Terminal Backend | crossterm | myfileと同様 |
| Clipboard | arboard | myfileと同様 |
| Image | image | myfileと同様 |
| Error | anyhow | myfileと同様 |

---

## 4. Core Data Structures

### 4.1 FileNode（myfileベース）

```rust
pub struct FileNode {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub expanded: bool,
    pub depth: usize,
    pub children: Vec<FileNode>,
}
```

### 4.2 App State（myfileベース）

```rust
pub struct App {
    // ツリー・ナビゲーション
    pub tree: FileTree,
    pub selected: usize,
    pub scroll_offset: usize,
    pub marked: HashSet<PathBuf>,

    // クリップボード
    pub clipboard: Clipboard,

    // 入力・モード
    pub input_mode: InputMode,
    pub input_buffer: String,

    // UI状態
    pub message: Option<String>,
    pub quick_preview_enabled: bool,
    pub preview_scroll: usize,

    // 終了フラグ
    pub should_quit: bool,

    // ★ 追加: パス連携
    pub pick_mode: bool,
    pub selected_path: Option<PathBuf>,
}
```

---

## 5. Directory Structure

myfileに近い構造を採用する。

```
fileview/
├── src/
│   ├── main.rs           # エントリーポイント・イベントループ
│   ├── app.rs            # App状態・ビジネスロジック
│   ├── file_tree.rs      # ツリーデータ構造・操作
│   ├── file_ops.rs       # ファイル操作・クリップボード
│   ├── ui.rs             # UI描画・レイアウト
│   ├── input.rs          # キーボード・マウス入力処理
│   └── preview.rs        # ★ 追加: プレビュー処理を分離
├── Cargo.toml
├── LICENSE
└── README.md
```

**myfileとの違い：**
- `git_status.rs` を削除（Git統合なし）
- `preview.rs` を追加（プレビュー処理を分離）

---

## 6. Key Features（myfileベース）

### 6.1 ファイルツリー管理

myfileと同様のフラット化メカニズム：

```rust
fn flatten_node(&mut self, node: &FileNode) {
    self.nodes.push(node.clone());
    if node.expanded {
        for child in &node.children {
            self.flatten_node(child);
        }
    }
}
```

### 6.2 ファイル操作

myfileと同様のCRUD操作：

```rust
pub fn create_file(parent_dir: &Path, name: &str) -> anyhow::Result<PathBuf>
pub fn create_dir(parent_dir: &Path, name: &str) -> anyhow::Result<PathBuf>
pub fn rename_file(path: &Path, new_name: &str) -> anyhow::Result<PathBuf>
pub fn delete_file(path: &Path) -> anyhow::Result<()>
pub fn copy_file(src: &Path, dest_dir: &Path) -> anyhow::Result<PathBuf>
```

### 6.3 クイックプレビュー

myfileと同様：

| Type | Preview |
|------|---------|
| Text | 先頭N行表示 |
| Image | 半ブロック文字（▀）でRGB表示 |
| Binary | ファイルサイズ表示 |

### 6.4 ドラッグ&ドロップ

myfileと同様のバッファ検出方式：

```rust
pub fn buffer_char(&mut self, c: char) {
    let now = Instant::now();
    let elapsed = now.duration_since(self.last_char_time).as_millis();
    if elapsed > 50 {
        self.drop_buffer.clear();
    }
    self.drop_buffer.push(c);
    self.last_char_time = now;
}
```

---

## 7. Key Bindings（myfileベース）

| Key | Action |
|-----|--------|
| `j` / `↓` | 下移動 |
| `k` / `↑` | 上移動 |
| `l` / `→` / `Enter` | 展開 / 確定 |
| `h` / `←` | 折りたたみ / 親へ |
| `g` | 先頭へ |
| `G` | 末尾へ |
| `Space` | マーク切替 |
| `y` | コピー |
| `d` | カット |
| `p` | ペースト |
| `D` | 削除 |
| `r` | リネーム |
| `a` | 新規ファイル |
| `A` | 新規フォルダ |
| `/` | 検索 |
| `P` | クイックプレビュー切替 |
| `o` | フルスクリーンプレビュー |
| `.` | 隠しファイル切替 |
| `q` | 終了 |

---

## 8. Path Integration（fileview追加機能）

myfileにはない機能として、外部連携を追加する。

### 8.1 標準出力モード

```bash
# 選択したパスを取得
selected=$(fileview --pick)
cd "$selected"

# パイプで渡す
fileview --pick | xargs cat
```

### 8.2 コールバック実行

```bash
# エディタで開く
fileview --on-select "nvim {path}"

# 複数選択
fileview --on-select "tar -cvf archive.tar {paths}"
```

### 8.3 終了コード

| Code | 意味 |
|------|------|
| 0 | 正常終了（パス選択あり） |
| 1 | キャンセル（q で終了） |
| 2 | エラー |

---

## 9. What FileView Does NOT Include

myfileから意図的に除外する機能：

| 機能 | 理由 |
|------|------|
| Git統合 | コア機能ではない。lazygit等を使用 |
| 外部コマンド実行（:） | `--on-select`で代替 |
| コマンド履歴ファイル | シンプルさ優先 |

---

## 10. Naming Conventions（書き方の違い）

myfileとは一部異なる命名規則を使用する。

| myfile | fileview | 理由 |
|--------|----------|------|
| `FileTree` | `FileTree` | 同じ |
| `FileNode` | `TreeNode` | より汎用的 |
| `App` | `AppState` | 役割を明確化 |
| `flat_list` | `visible_indices` | 意味を明確化 |
| `selected` | `cursor` | VSCode用語に合わせる |
| `quick_preview_enabled` | `preview_visible` | 短縮 |

---

## 11. Implementation Notes

### 11.1 myfileから流用するロジック

以下のロジックはmyfileを参考に実装する（書き方を変える）：

- フラット化ツリーの構築・更新
- 展開/折りたたみ処理
- スクロール自動調整
- ドラッグ&ドロップ検出
- 画像の半ブロック描画
- ファイル操作（TOCTOU対策）

### 11.2 独自実装

以下は新規実装：

- `--pick` モード
- `--on-select` コールバック
- プレビュー処理の分離（preview.rs）

---

## 12. Summary

| Aspect | myfile | fileview |
|--------|--------|----------|
| ベース | - | myfileをベース |
| アーキテクチャ | State Machine | 同じ |
| ツリー構造 | flat_list | 同じ |
| Git統合 | あり | **なし** |
| 外部コマンド | あり（:） | **なし**（--on-selectで代替） |
| パス連携 | なし | **--pick, --on-select** |
| 命名規則 | 独自 | 一部変更 |

**fileviewはmyfileの設計をベースに、ミニマル化とパス連携機能を追加したツールである。**
