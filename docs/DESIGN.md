# FileView - Design Document

## 1. Overview

FileViewは、ターミナルエミュレーター上で動作するVSCode風のミニマルファイルツリーUIである。
Ghostty等のモダンターミナルでの使用を想定し、**軽量・高速・直感操作**を設計思想の中核とする。

### 1.1 設計思想

- **View-First Architecture**: ファイルシステムの「閲覧」を最優先とし、編集機能は補助的位置づけ
- **Event-Driven Design**: 全操作をイベントとして抽象化し、疎結合な構成を実現
- **Lazy Evaluation**: 必要になるまでデータを読み込まない遅延評価戦略
- **Single Source of Truth**: アプリケーション状態を単一のStateオブジェクトで管理

## 2. Architecture

```
┌─────────────────────────────────────────────────────┐
│                    FileView                         │
├─────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐ │
│  │   View      │  │  Controller │  │    Model    │ │
│  │  (TUI)      │◄─┤  (Events)   │◄─┤  (State)    │ │
│  └─────────────┘  └─────────────┘  └─────────────┘ │
│         │                │                │        │
│         └────────────────┼────────────────┘        │
│                          ▼                         │
│                 ┌─────────────────┐                │
│                 │   FileSystem    │                │
│                 │   Abstraction   │                │
│                 └─────────────────┘                │
└─────────────────────────────────────────────────────┘
```

### 2.1 レイヤー構成

| Layer | Responsibility |
|-------|----------------|
| View | TUI描画、ユーザー入力受付 |
| Controller | イベント処理、ビジネスロジック |
| Model | アプリケーション状態管理 |
| FileSystem | OS抽象化、ファイル操作 |

## 3. Technology Stack

| Category | Choice | Rationale |
|----------|--------|-----------|
| Language | Rust | メモリ安全性、高速性、クロスプラットフォーム |
| TUI Framework | ratatui | アクティブな開発、豊富なウィジェット |
| Async Runtime | tokio | 非同期I/O、ファイル監視 |
| Serialization | serde | 設定ファイル、状態永続化 |

## 4. Core Features

### 4.1 ファイルツリー表示

- 階層構造のインデント表示
- フォルダ展開/折りたたみ（遅延読み込み）
- アイコン表示（Nerd Fonts対応）
- ファイルタイプ別カラーリング

### 4.2 ファイル操作

| Operation | Shortcut | Description |
|-----------|----------|-------------|
| Create | `a` | 新規ファイル/フォルダ作成 |
| Rename | `r` | 名前変更 |
| Delete | `d` | 削除（確認プロンプト付き） |
| Copy | `y` | クリップボードにコピー |
| Paste | `p` | ペースト |
| Move | `m` | 移動モード |

### 4.3 ドラッグ&ドロップ

- OSC 52エスケープシーケンスによるパス取得
- ターミナルからのドロップイベント検知
- コピー先ディレクトリの自動判定

### 4.4 クイックプレビュー

| Type | Preview |
|------|---------|
| Text | 先頭N行のシンタックスハイライト表示 |
| Image | Sixel/Kittyプロトコルによるサムネイル |
| Binary | ファイルサイズ、MIMEタイプ、先頭バイト |

### 4.5 パス連携

- 選択中アイテムのパスを標準出力/クリップボード/環境変数へエクスポート
- 外部コマンド実行時のパス展開（`$FILEVIEW_SELECTED`）

## 5. Directory Structure

```
fileview/
├── src/
│   ├── main.rs              # エントリーポイント
│   ├── app.rs               # アプリケーション状態
│   ├── event/
│   │   ├── mod.rs           # イベントモジュール
│   │   ├── handler.rs       # イベントハンドラー
│   │   └── key.rs           # キーバインド定義
│   ├── ui/
│   │   ├── mod.rs           # UIモジュール
│   │   ├── tree.rs          # ツリービュー
│   │   ├── preview.rs       # プレビューパネル
│   │   └── statusbar.rs     # ステータスバー
│   ├── fs/
│   │   ├── mod.rs           # ファイルシステムモジュール
│   │   ├── entry.rs         # ファイル/ディレクトリエントリ
│   │   ├── operations.rs    # CRUD操作
│   │   └── watcher.rs       # ファイル監視
│   └── config/
│       ├── mod.rs           # 設定モジュール
│       └── theme.rs         # テーマ設定
├── docs/
│   └── DESIGN.md            # 本ドキュメント
├── tests/
│   └── integration/         # 統合テスト
├── Cargo.toml
├── CONTRIBUTING.md
├── LICENSE
└── README.md
```

## 6. State Management

```rust
pub struct AppState {
    // ツリー状態
    tree: TreeState,
    // 選択状態
    selection: SelectionState,
    // プレビュー状態
    preview: PreviewState,
    // 操作モード
    mode: OperationMode,
    // 設定
    config: Config,
}

pub enum OperationMode {
    Normal,
    Search,
    Rename,
    Confirm(ConfirmAction),
}
```

## 7. Event Flow

```
User Input
    │
    ▼
┌─────────────┐
│ EventLoop   │ ─── キー/マウスイベント取得
└─────────────┘
    │
    ▼
┌─────────────┐
│ Dispatcher  │ ─── イベントをActionに変換
└─────────────┘
    │
    ▼
┌─────────────┐
│ Handler     │ ─── Actionに基づき状態更新
└─────────────┘
    │
    ▼
┌─────────────┐
│ Renderer    │ ─── 状態からUIを再描画
└─────────────┘
```

## 8. Preview Strategy

プレビューは**非同期**かつ**キャッシュ付き**で実装する。

```
Selection Changed
    │
    ▼
Debounce (100ms)
    │
    ▼
Check Cache ──── Hit ───► Render from Cache
    │
    Miss
    │
    ▼
Spawn Preview Task
    │
    ▼
Load Content (async)
    │
    ▼
Update Cache & Render
```

## 9. Configuration

`~/.config/fileview/config.toml`:

```toml
[general]
show_hidden = false
follow_symlinks = true

[preview]
enabled = true
max_lines = 50
image_protocol = "auto"  # auto | sixel | kitty | none

[theme]
style = "default"  # default | minimal | colorful

[keybindings]
quit = "q"
up = "k"
down = "j"
```

## 10. Future Considerations

- Git統合（変更ステータス表示）
- ファジーファインダー統合
- マルチペイン対応
- プラグインシステム
