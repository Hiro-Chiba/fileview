# FileView - Design Document

## 1. Overview

FileViewは、ターミナルエミュレーター上で動作するVSCode風のミニマルファイルツリーUIである。
Ghostty等のモダンターミナルでの使用を想定し、**軽量・高速・直感操作**を設計思想の中核とする。

### 1.1 参考プロジェクト

本プロジェクトは [filetree](../../../filetree) を参考にしている。
filetreeの優れた点を学びつつ、以下の観点で独自の設計アプローチを採用する。

| 観点 | filetree | fileview（本プロジェクト） |
|------|----------|---------------------------|
| アーキテクチャ | モード駆動型 State Machine | **Command Pattern + Undo/Redo** |
| ツリー構造 | フラット化リスト同期 | **仮想化ツリー（表示範囲のみ計算）** |
| I/O処理 | 同期中心 | **tokio非同期全面採用** |
| Git統合 | 組み込み | **除外（ミニマル設計）** |
| 画像表示 | 半ブロック文字（▀） | **Sixel/Kittyプロトコル優先** |
| 拡張性 | 単一バイナリ | **トレイトベース拡張** |

### 1.2 設計思想

- **Command Pattern**: 全操作をCommandオブジェクトとして表現し、Undo/Redo対応
- **Virtualized Rendering**: 大規模ディレクトリでも表示範囲のみ計算・描画
- **Non-Blocking UI**: 重い処理をバックグラウンドで実行し、UIをブロックしない
- **Minimal Core**: Git等の追加機能は意図的に除外し、コア機能に集中

### 1.3 「軽量・高速」の定義

| 用語 | 意味 | 実現方法 |
|------|------|----------|
| **軽量** | メモリ使用量が少ない | 仮想化ツリー（表示範囲のみ保持） |
| **高速** | 起動・描画が速い | Rust、最適化ビルド、遅延読み込み |
| **応答性** | UIが固まらない | 非同期I/O（tokio）でプレビュー等をバックグラウンド処理 |

**注意**: 非同期処理は「速度」ではなく「応答性」を向上させる。
ファイル読み込み自体は同期でも非同期でも同じ速度だが、UIがブロックされないため体感が良くなる。

---

## 2. Architecture

### 2.1 全体構成

```
┌──────────────────────────────────────────────────────────────┐
│                        FileView                              │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐   │
│  │   Runtime    │    │   Command    │    │    State     │   │
│  │  (EventLoop) │───►│  (Actions)   │───►│   (Store)    │   │
│  └──────────────┘    └──────────────┘    └──────────────┘   │
│         │                   │                   │            │
│         │                   ▼                   │            │
│         │            ┌──────────────┐           │            │
│         │            │   History    │           │            │
│         │            │ (Undo/Redo)  │           │            │
│         │            └──────────────┘           │            │
│         │                                       │            │
│         ▼                                       ▼            │
│  ┌──────────────┐                      ┌──────────────┐     │
│  │     View     │◄─────────────────────│  Virtualizer │     │
│  │    (TUI)     │                      │   (Viewport) │     │
│  └──────────────┘                      └──────────────┘     │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### 2.2 レイヤー責務

| Layer | Responsibility | filetreeとの違い |
|-------|----------------|-----------------|
| Runtime | イベントループ、入力処理 | tokio非同期ランタイム |
| Command | 操作の抽象化、実行/取消 | Undo/Redo対応 |
| State | アプリケーション状態保持 | イミュータブル更新 |
| History | コマンド履歴管理 | filetreeには無い機能 |
| Virtualizer | 表示範囲の計算 | フラット化不要 |
| View | TUI描画 | 同様 |

### 2.3 Command Pattern の採用理由

filetreeではInputModeによるState Machineパターンを採用している。
これは直感的だが、Undo/Redo実装が困難という課題がある。

fileviewではCommand Patternを採用し、全操作を以下のインターフェースで統一する：

```rust
pub trait Command: Send + Sync {
    /// コマンドを実行し、状態を更新
    fn execute(&self, state: &mut AppState) -> CommandResult;

    /// コマンドを取り消し（Undo）
    fn undo(&self, state: &mut AppState) -> CommandResult;

    /// コマンドの説明（履歴表示用）
    fn description(&self) -> &str;

    /// Undo可能かどうか
    fn is_undoable(&self) -> bool { true }
}
```

**利点：**
- 操作の記録・再生が容易
- Undo/Redo実装が自然
- テスト可能性の向上
- マクロ機能への拡張が容易

---

## 3. Technology Stack

| Category | Choice | Rationale |
|----------|--------|-----------|
| Language | Rust | メモリ安全性、高速性 |
| TUI Framework | ratatui | filetreeと同様（実績あり） |
| Terminal Backend | crossterm | filetreeと同様（クロスプラットフォーム） |
| Async Runtime | tokio | 非同期I/O（filetreeとの差別化） |
| Clipboard | arboard | filetreeと同様 |
| Image | image | filetreeと同様 |
| Error | thiserror | 型安全なエラー定義（anyhowではなく） |

### 3.1 依存関係の方針

- **共通クレート**: ratatui, crossterm, arboard, image（実績重視）
- **差別化クレート**: tokio（非同期）, thiserror（型安全エラー）
- **除外**: Git関連クレート（ミニマル設計）

---

## 4. Core Data Structures

### 4.1 仮想化ツリー（Virtualized Tree）

filetreeでは`flat_list: Vec<usize>`でツリーをフラット化し、全ノードをメモリに保持する。
fileviewでは**仮想化**により、表示に必要なノードのみを計算する。

```rust
/// ツリーノード（遅延評価）
pub struct TreeNode {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub depth: usize,
    pub expanded: bool,
    /// 子ノードは展開時に初めて読み込む（Option）
    children: Option<Vec<TreeNode>>,
}

/// 仮想化ビューポート
pub struct Viewport {
    /// 表示開始位置（論理インデックス）
    pub offset: usize,
    /// 表示行数
    pub height: usize,
    /// 表示用にレンダリングされたノード
    pub visible_nodes: Vec<VisibleNode>,
}

/// 表示用ノード（計算済み）
pub struct VisibleNode {
    pub index: usize,
    pub node: TreeNode,
    pub indent: String,
    pub icon: &'static str,
}
```

**利点：**
- 10万ファイル超のディレクトリでも軽快
- メモリ使用量が表示行数に比例（O(viewport) vs O(total)）
- 展開状態の変更時に全リスト再構築不要

### 4.2 状態管理

```rust
/// アプリケーション状態（イミュータブル更新）
#[derive(Clone)]
pub struct AppState {
    /// ルートパス
    pub root: PathBuf,
    /// ツリールートノード
    pub tree: TreeNode,
    /// 選択インデックス
    pub cursor: usize,
    /// マーク済みパス
    pub marked: HashSet<PathBuf>,
    /// クリップボード
    pub clipboard: ClipboardState,
    /// 入力モード
    pub input: InputState,
    /// プレビュー
    pub preview: PreviewState,
    /// 設定
    pub config: Config,
}

/// 入力状態（filetreeのInputModeに相当するが、より細分化）
pub struct InputState {
    pub mode: InputMode,
    pub buffer: String,
    pub cursor_pos: usize,
}

pub enum InputMode {
    Normal,
    Search { direction: SearchDirection },
    Input { purpose: InputPurpose },
    Confirm { action: ConfirmAction },
}

pub enum InputPurpose {
    NewFile,
    NewDir,
    Rename { original: PathBuf },
}
```

---

## 5. Command Examples

### 5.1 ファイル削除コマンド

```rust
pub struct DeleteCommand {
    targets: Vec<PathBuf>,
    /// Undo用：削除前のバックアップ先
    backup_dir: Option<PathBuf>,
}

impl Command for DeleteCommand {
    fn execute(&self, state: &mut AppState) -> CommandResult {
        // 1. バックアップディレクトリを作成
        let backup = tempfile::tempdir()?;

        // 2. 各ファイルをバックアップ後に削除
        for path in &self.targets {
            let backup_path = backup.path().join(path.file_name().unwrap());
            fs::rename(path, &backup_path)?;
        }

        // 3. バックアップ先を保存（Undo用）
        self.backup_dir = Some(backup.into_path());

        // 4. ツリーを更新
        state.refresh_tree()?;

        Ok(())
    }

    fn undo(&self, state: &mut AppState) -> CommandResult {
        // バックアップから復元
        if let Some(backup_dir) = &self.backup_dir {
            for entry in fs::read_dir(backup_dir)? {
                let entry = entry?;
                let original = self.targets.iter()
                    .find(|p| p.file_name() == entry.path().file_name());
                if let Some(original) = original {
                    fs::rename(entry.path(), original)?;
                }
            }
        }
        state.refresh_tree()?;
        Ok(())
    }

    fn description(&self) -> &str {
        "Delete files"
    }
}
```

### 5.2 履歴管理

```rust
pub struct CommandHistory {
    /// 実行済みコマンド（Undo用）
    undo_stack: Vec<Box<dyn Command>>,
    /// Undo済みコマンド（Redo用）
    redo_stack: Vec<Box<dyn Command>>,
    /// 最大履歴数
    max_size: usize,
}

impl CommandHistory {
    pub fn execute(&mut self, cmd: Box<dyn Command>, state: &mut AppState) -> CommandResult {
        cmd.execute(state)?;
        if cmd.is_undoable() {
            self.undo_stack.push(cmd);
            self.redo_stack.clear(); // 新規操作でRedoスタッククリア
        }
        Ok(())
    }

    pub fn undo(&mut self, state: &mut AppState) -> CommandResult {
        if let Some(cmd) = self.undo_stack.pop() {
            cmd.undo(state)?;
            self.redo_stack.push(cmd);
        }
        Ok(())
    }

    pub fn redo(&mut self, state: &mut AppState) -> CommandResult {
        if let Some(cmd) = self.redo_stack.pop() {
            cmd.execute(state)?;
            self.undo_stack.push(cmd);
        }
        Ok(())
    }
}
```

---

## 6. Directory Structure

filetreeとは異なるモジュール構成を採用する。

```
fileview/
├── src/
│   ├── main.rs                 # エントリーポイント
│   ├── lib.rs                  # ライブラリルート
│   │
│   ├── app/
│   │   ├── mod.rs              # Appモジュール
│   │   ├── state.rs            # AppState定義
│   │   └── runtime.rs          # イベントループ（tokio）
│   │
│   ├── command/                # ★ Command Pattern
│   │   ├── mod.rs              # Commandトレイト
│   │   ├── history.rs          # Undo/Redo履歴
│   │   ├── navigation.rs       # 移動系コマンド
│   │   ├── file_ops.rs         # ファイル操作コマンド
│   │   └── tree_ops.rs         # ツリー操作コマンド
│   │
│   ├── tree/                   # ★ 仮想化ツリー
│   │   ├── mod.rs              # TreeNode定義
│   │   ├── virtualizer.rs      # Viewport計算
│   │   └── loader.rs           # 非同期ローダー
│   │
│   ├── view/
│   │   ├── mod.rs              # Viewモジュール
│   │   ├── tree_view.rs        # ツリー描画
│   │   ├── preview_view.rs     # プレビュー描画
│   │   ├── input_view.rs       # 入力UI描画
│   │   └── status_view.rs      # ステータスバー
│   │
│   ├── input/
│   │   ├── mod.rs              # 入力処理
│   │   ├── key.rs              # キーバインド
│   │   └── mouse.rs            # マウス処理
│   │
│   ├── preview/                # ★ 非同期プレビュー
│   │   ├── mod.rs              # プレビューモジュール
│   │   ├── text.rs             # テキストプレビュー
│   │   ├── image.rs            # 画像プレビュー（Sixel/Kitty）
│   │   └── binary.rs           # バイナリ情報
│   │
│   ├── clipboard/
│   │   ├── mod.rs              # クリップボード
│   │   └── osc52.rs            # OSC 52対応
│   │
│   └── config/
│       ├── mod.rs              # 設定
│       └── theme.rs            # テーマ
│
├── tests/
│   ├── command_tests.rs        # コマンドテスト
│   ├── tree_tests.rs           # ツリーテスト
│   └── integration/            # 統合テスト
│
├── docs/
│   ├── DESIGN.md               # 本ドキュメント
│   └── ROADMAP.md              # 実装計画
│
├── .github/
│   └── workflows/
│       └── ci.yml              # CI設定
│
├── Cargo.toml
├── CONTRIBUTING.md
├── LICENSE
└── README.md
```

---

## 7. Async Preview Strategy

filetreeでは同期的にプレビューを読み込む。
fileviewでは**tokio**を使用した非同期プレビューを実装する。

```
┌─────────────────────────────────────────────────────────┐
│                    Preview Pipeline                     │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  Selection Changed                                      │
│        │                                                │
│        ▼                                                │
│  ┌───────────┐                                         │
│  │ Debouncer │ ─── 150ms待機（高速移動時のキャンセル）  │
│  └───────────┘                                         │
│        │                                                │
│        ▼                                                │
│  ┌───────────┐     ┌─────────┐                        │
│  │   Cache   │────►│  Hit    │───► Render             │
│  │   Check   │     └─────────┘                        │
│  └───────────┘                                         │
│        │ Miss                                           │
│        ▼                                                │
│  ┌───────────┐                                         │
│  │  Spawn    │ ─── tokio::spawn で非同期タスク        │
│  │  Task     │                                         │
│  └───────────┘                                         │
│        │                                                │
│        ▼                                                │
│  ┌───────────┐     ┌───────────┐                      │
│  │   Load    │────►│  Detect   │ ─── MIME判定        │
│  │   File    │     │   Type    │                      │
│  └───────────┘     └───────────┘                      │
│                          │                              │
│        ┌─────────────────┼─────────────────┐           │
│        ▼                 ▼                 ▼           │
│  ┌──────────┐     ┌──────────┐     ┌──────────┐      │
│  │   Text   │     │  Image   │     │  Binary  │      │
│  │ Preview  │     │ Preview  │     │   Info   │      │
│  └──────────┘     └──────────┘     └──────────┘      │
│        │                 │                 │           │
│        └─────────────────┼─────────────────┘           │
│                          ▼                              │
│                   ┌───────────┐                        │
│                   │   Cache   │                        │
│                   │   Store   │                        │
│                   └───────────┘                        │
│                          │                              │
│                          ▼                              │
│                   ┌───────────┐                        │
│                   │  Render   │ ─── チャンネル経由で通知│
│                   └───────────┘                        │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### 7.1 画像プレビューの差別化

filetreeは**半ブロック文字（▀）**でRGB表示する。
これは互換性が高いが、解像度が低い。

fileviewでは**Sixel/Kittyプロトコル**を優先し、非対応ターミナルのみ半ブロックにフォールバック：

```rust
pub enum ImageProtocol {
    Kitty,      // Kitty Graphics Protocol
    Sixel,      // Sixel Graphics
    HalfBlock,  // 半ブロック文字（フォールバック）
    None,       // 画像非対応
}

impl ImageProtocol {
    pub fn detect() -> Self {
        // 1. TERM_PROGRAM / KITTY_WINDOW_ID をチェック
        if std::env::var("KITTY_WINDOW_ID").is_ok() {
            return Self::Kitty;
        }

        // 2. Sixel対応チェック（DA1レスポンス）
        if check_sixel_support() {
            return Self::Sixel;
        }

        // 3. フォールバック
        Self::HalfBlock
    }
}
```

---

## 8. Key Bindings

filetreeと同様のVim風キーバインドを採用するが、Undo/Redo用のキーを追加。

| Key | Action | filetreeとの違い |
|-----|--------|-----------------|
| `j` / `↓` | 下移動 | 同様 |
| `k` / `↑` | 上移動 | 同様 |
| `l` / `→` / `Enter` | 展開 / 開く | 同様 |
| `h` / `←` | 折りたたみ / 親へ | 同様 |
| `g` | 先頭へ | 同様 |
| `G` | 末尾へ | 同様 |
| `Space` | マーク切替 | 同様 |
| `y` | コピー | 同様 |
| `d` | カット | 同様 |
| `p` | ペースト | 同様 |
| `D` | 削除 | 同様 |
| `r` | リネーム | 同様 |
| `a` | 新規ファイル | 同様 |
| `A` | 新規フォルダ | 同様 |
| `/` | 検索 | 同様 |
| `u` | **Undo** | ★ 追加 |
| `Ctrl+r` | **Redo** | ★ 追加 |
| `P` | プレビュー切替 | 同様 |
| `.` | 隠しファイル切替 | 同様 |
| `q` | 終了 | 同様 |

---

## 9. Configuration

`~/.config/fileview/config.toml`:

```toml
[general]
# 隠しファイルを表示
show_hidden = false
# シンボリックリンクを追跡
follow_symlinks = true
# Undo履歴の最大数
max_undo_history = 100

[preview]
# プレビュー有効化
enabled = true
# テキストプレビューの最大行数
max_lines = 50
# 画像プロトコル（auto | kitty | sixel | halfblock | none）
image_protocol = "auto"
# プレビューのdebounce時間（ms）
debounce_ms = 150

[theme]
# テーマ（default | minimal）
style = "default"

[keybindings]
# カスタムキーバインド（オプション）
# quit = "q"
# undo = "u"
# redo = "C-r"
```

---

## 10. Path Integration（パス連携）

選択中のパスを外部と連携するための複数の方法を提供する。

### 10.1 クリップボード連携

```
Y キー → OSC 52 エスケープシーケンス → システムクリップボード
```

ターミナル経由でクリップボードにコピー。SSH越しでも動作。

### 10.2 標準出力モード

```bash
# 選択したパスを変数に格納
selected=$(fileview --pick)

# パイプで他のコマンドに渡す
fileview --pick | xargs cat
```

`--pick` オプションで起動すると、Enter確定時に選択パスを stdout に出力して終了。

### 10.3 コールバック実行

```bash
# 選択確定時にコマンドを実行
fileview --on-select "code {path}"

# 複数ファイル選択時
fileview --on-select "tar -cvf archive.tar {paths}"
```

プレースホルダー:
- `{path}`: 選択中のパス（単一）
- `{paths}`: マーク済み全パス（スペース区切り）
- `{dir}`: 選択中アイテムの親ディレクトリ

### 10.4 終了コード

| Code | 意味 |
|------|------|
| 0 | 正常終了（パス選択あり） |
| 1 | キャンセル（q で終了） |
| 2 | エラー |

### 10.5 使用例

```bash
# fzf的な使い方
cd "$(fileview --pick)"

# エディタとの連携
fileview --on-select "nvim {path}"

# ファイルピッカーとして
attachment=$(fileview ~/Documents --pick)
mail --attach "$attachment" user@example.com
```

---

## 11. What FileView Does NOT Include

ミニマル設計として、以下の機能は**意図的に除外**する：

| 機能 | 理由 |
|------|------|
| Git統合 | コア機能ではない。必要なら専用ツール（lazygit等）を使用 |
| 外部コマンド実行 | スコープ外。シェルに戻って実行すればよい |
| コマンド履歴ファイル | 永続化は不要。セッション内Undo/Redoで十分 |
| 複雑なフィルタリング | fd/rg等の専用ツールに任せる |

---

## 12. Testing Strategy

### 11.1 ユニットテスト

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_delete_command_undo() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("test.txt");
        fs::write(&file, "content").unwrap();

        let mut state = AppState::new(temp.path()).unwrap();
        let mut history = CommandHistory::new(100);

        // 削除実行
        let cmd = DeleteCommand::new(vec![file.clone()]);
        history.execute(Box::new(cmd), &mut state).unwrap();
        assert!(!file.exists());

        // Undo
        history.undo(&mut state).unwrap();
        assert!(file.exists());
        assert_eq!(fs::read_to_string(&file).unwrap(), "content");
    }
}
```

### 11.2 統合テスト

- CI（GitHub Actions）で自動実行
- 仮想ターミナル環境でのE2Eテスト

---

## 13. Summary: filetree vs fileview

| Aspect | filetree | fileview |
|--------|----------|----------|
| 設計思想 | 機能豊富なファイルマネージャ | ミニマルなファイルビューア |
| アーキテクチャ | State Machine | Command Pattern |
| Undo/Redo | なし | **あり** |
| Git統合 | あり | なし |
| 非同期 | 部分的 | 全面採用 |
| 画像表示 | 半ブロック | Sixel/Kitty優先 |
| 外部コマンド | あり | なし |
| ターゲット | 汎用 | サイドバー用途 |

**fileviewは「見る」ことに特化した軽量ツールである。**
