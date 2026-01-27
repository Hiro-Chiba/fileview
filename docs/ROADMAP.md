# FileView - Implementation Roadmap

## Overview

モダンターミナル向けのミニマルファイルツリーUIを実装する。

---

## Phase 1: Foundation

- [x] 1.1 プロジェクト初期化
  - Cargo.toml
  - .gitignore
  - PR: `chore: Initialize Rust project`

- [x] 1.2 CI設定
  - .github/workflows/ci.yml
  - PR: `chore: Set up GitHub Actions CI`

- [x] 1.3 モジュール構造作成
  - src/lib.rs + 各モジュールのmod.rs
  - PR: `chore: Set up module structure`

---

## Phase 2: Core Module

- [x] 2.1 core/state.rs
  - AppState構造体
  - PR: `feat(core): Define AppState`

- [x] 2.2 core/mode.rs
  - ViewMode enum（状態内包型）
  - InputPurpose, PendingAction
  - PR: `feat(core): Define ViewMode with embedded state`

---

## Phase 3: Tree Module

- [x] 3.1 tree/node.rs
  - TreeEntry構造体
  - PR: `feat(tree): Define TreeEntry`

- [x] 3.2 tree/navigator.rs
  - TreeNavigator構造体
  - フラット化（flatten / collect_visible）
  - 展開/折りたたみ
  - PR: `feat(tree): Implement TreeNavigator with flatten`

---

## Phase 4: Action Module

- [x] 4.1 action/file.rs
  - create_file / create_dir
  - rename / delete
  - PR: `feat(action): Implement file operations`

- [x] 4.2 action/clipboard.rs
  - copy / cut / paste
  - Clipboard構造体
  - PR: `feat(action): Implement clipboard operations`

---

## Phase 5: Render Module

- [x] 5.1 render/tree.rs
  - ツリー描画
  - PR: `feat(render): Implement tree rendering`

- [x] 5.2 render/preview.rs
  - テキストプレビュー
  - 画像プレビュー（半ブロック）
  - PR: `feat(render): Implement preview rendering`

- [x] 5.3 render/status.rs
  - ステータスバー
  - 入力UI
  - PR: `feat(render): Implement status bar`

---

## Phase 6: Handler Module

- [x] 6.1 handler/key.rs
  - キーイベント処理
  - モード別ハンドラー
  - PR: `feat(handler): Implement key handling`

- [x] 6.2 handler/mouse.rs
  - マウスイベント処理
  - ダブルクリック検出
  - PR: `feat(handler): Implement mouse handling`

- [x] 6.3 DropDetector
  - D&D検出
  - PR: `feat(handler): Implement drag and drop detection`

---

## Phase 7: Integrate Module

- [x] 7.1 integrate/pick.rs
  - --pick オプション
  - stdout出力
  - 終了コード
  - PR: `feat(integrate): Implement --pick mode`

- [x] 7.2 integrate/callback.rs
  - --on-select オプション
  - プレースホルダー展開
  - PR: `feat(integrate): Implement --on-select callback`

---

## Phase 8: Main & Polish

- [x] 8.1 main.rs
  - イベントループ
  - ターミナル初期化/復元
  - PR: `feat: Implement main event loop`

- [x] 8.2 README.md
  - インストール、使用方法
  - PR: `docs: Add README`

- [x] 8.3 テスト
  - tree, action のユニットテスト
  - PR: `test: Add unit tests`

---

## Phase 9: Enhanced Features

### 9.1 Git ステータス表示
**優先度:** 高
**リリース:** v0.2.0

- [x] git/status.rs
  - Gitリポジトリ検出
  - ファイル状態取得（Modified, Added, Untracked, Deleted, Renamed, Ignored）
  - ディレクトリ状態の伝播（子ファイルの状態を親に反映）
  - キャッシュ機構（パフォーマンス最適化）
- [x] render/tree.rs 拡張
  - 状態別カラー表示
    - Modified: Yellow
    - Added/Untracked: Green
    - Deleted: Red
    - Renamed: Cyan
    - Ignored: DarkGray
- [x] render/status.rs 拡張
  - 現在のブランチ名表示
- [x] PR: `feat(git): Add git status display`

**実装詳細:**
```rust
pub struct GitStatus {
    repo_root: PathBuf,
    statuses: HashMap<PathBuf, FileStatus>,
}

pub enum FileStatus {
    Modified,
    Added,
    Untracked,
    Deleted,
    Renamed,
    Ignored,
    Conflict,
    Clean,
}
```

---

### 9.2 ディレクトリ情報表示
**優先度:** 中
**リリース:** v0.3.0

- [x] render/preview.rs 拡張
  - ディレクトリ選択時の情報表示
    - ファイル数
    - サブディレクトリ数
    - 隠しファイル数
    - 合計サイズ（human-readable: KB, MB, GB）
  - 深さ制限付きサイズ計算（depth=3でパフォーマンス確保）
- [x] PR: `feat(preview): Add directory info display`

**表示例:**
```
📁 src/
────────────────────
Files:        42
Directories:   8
Hidden:        2
Total Size:  1.2 MB
```

---

### 9.3 Hex プレビュー
**優先度:** 中
**リリース:** v0.4.0

- [x] render/preview.rs 拡張
  - バイナリファイル検出
  - xxd形式のHexダンプ表示
    - オフセット | Hex (16バイト) | ASCII
  - テキスト/バイナリ自動判定
- [x] PR: `feat(preview): Add hex preview for binary files`

**表示例:**
```
00000000: 7f45 4c46 0201 0100 0000 0000 0000 0000  .ELF............
00000010: 0300 3e00 0100 0000 1010 0000 0000 0000  ..>.............
00000020: 4000 0000 0000 0000 9019 0000 0000 0000  @...............
```

---

## Progress Summary

| Phase | Items | Completed |
|-------|-------|-----------|
| 1. Foundation | 3 | 3 |
| 2. Core | 2 | 2 |
| 3. Tree | 2 | 2 |
| 4. Action | 2 | 2 |
| 5. Render | 3 | 3 |
| 6. Handler | 3 | 3 |
| 7. Integrate | 2 | 2 |
| 8. Main & Polish | 3 | 3 |
| 9. Enhanced Features | 3 | 3 |
| 10. Code Quality | 3 | 3 |
| 11. Nerd Fonts Icons | 3 | 3 |
| 12. Test Improvements | 6 | 6 |
| 13. E2E / Behavioral Tests | 4 | 3 |
| 14. Side Preview Focus | 5 | 5 |
| 15. Image Protocol Support | 8 | 6 |
| **Total** | **52** | **49** |

**注意:** Phase 15.8（main.rs統合）が完了するまでv0.8.0のリリースは行わない。

---

## Release Plan

| Version | Feature | Status |
|---------|---------|--------|
| v0.1.x | Initial release | ✅ Published |
| v0.2.0 | Git status display | ✅ Published |
| v0.3.0 | Directory info | ✅ Published |
| v0.4.0 | Hex preview | ✅ Published |
| v0.4.4 | Ghostty drag-drop fix | ✅ Published |
| v0.4.5 | PathBuffer refactoring | ✅ Published |
| v0.4.6 | DRY improvements | ✅ Published |
| v0.4.7 | Error handling | ✅ Published |
| v0.4.8 | Constants extraction | ✅ Published |
| v0.5.0 | Nerd Fonts icons | ✅ Published |
| v0.6.0 | Test improvements | ✅ Published |
| v0.6.1 | Bug fixes (preview scroll, Enter key) | ✅ Published |

---

## Phase 10: Code Quality & Refactoring

**リリース:** v0.4.6

### 10.1 DRY改善
**優先度:** 高

- [x] ファイルドロップ処理の統合
  - 現状: main.rs内で3箇所に重複
  - 解決: `handle_file_drop()` 関数に抽出
- [x] 宛先ディレクトリ計算の共通化
  - 現状: 6箇所で同じパターン
  - 解決: `get_target_directory()` ヘルパー関数
- [x] プレビュータイトル取得の共通化
  - 現状: 4箇所で重複
  - 解決: `get_filename_str()` ユーティリティ関数
- [x] PR: `refactor: Extract common helper functions (DRY)`

### 10.2 エラーハンドリング強化
**優先度:** 中

- [x] サイレント失敗の修正
  - ファイルコピー失敗時のユーザー通知
  - クリップボード操作失敗時のフィードバック
- [x] パス操作のエラー処理改善
  - 既存の `unwrap_or_else` パターンは適切
- [x] PR: `refactor: Improve error handling and user feedback`

### 10.3 定数化
**優先度:** 低

- [x] preview.rs のマジックナンバー
  - `MAX_DIR_SIZE_DEPTH = 3`
  - `HEX_PREVIEW_MAX_BYTES = 4096`
  - `HEX_BYTES_PER_LINE = 16`
- [x] PR: `refactor: Extract magic numbers to constants`

---

## Phase 11: Nerd Fonts Icons

**リリース:** v0.5.0

### 11.1 アイコンマッピング
**優先度:** 高

- [x] render/icons.rs 新規作成
  - ファイル拡張子→アイコンのマッピング
  - ディレクトリ用アイコン（展開/折りたたみ）
  - 特殊ディレクトリ（.git, node_modules, src等）
- [x] 主要な拡張子サポート
  - プログラミング言語: rs, py, js, ts, go, java, c, cpp, etc.
  - 設定ファイル: json, yaml, toml, xml, etc.
  - ドキュメント: md, txt, pdf, etc.
  - メディア: png, jpg, mp3, mp4, etc.
- [x] PR: `feat(render): Add icon mapping module`

### 11.2 ツリー描画への統合
**優先度:** 高

- [x] render/tree.rs 拡張
  - TreeEntryにアイコン表示を追加
  - Git状態アイコンとの共存
- [x] アイコン表示位置
  - `📁 dirname/` または ` dirname/`
  - ` filename.rs` または ` filename.py`
- [x] PR: `feat(render): Integrate icons into tree view`

### 11.3 設定オプション
**優先度:** 中

- [x] CLIオプション追加
  - `--icons` / `-i`: アイコン表示を有効化
  - `--no-icons`: アイコン表示を無効化（デフォルト）
- [x] 環境変数サポート
  - `FILEVIEW_ICONS=1` でデフォルト有効化
- [ ] Nerd Font未インストール時のフォールバック
  - Unicode絵文字または記号にフォールバック
- [x] PR: `feat(cli): Add icon display options`

**実装詳細:**
```rust
// render/icons.rs
pub fn get_file_icon(path: &Path, is_dir: bool, expanded: bool) -> &'static str {
    if is_dir {
        if expanded { "" } else { "" }
    } else {
        match path.extension().and_then(|e| e.to_str()) {
            Some("rs") => "",
            Some("py") => "",
            Some("js") => "",
            Some("ts") => "",
            Some("json") => "",
            Some("md") => "",
            Some("git") => "",
            _ => "",
        }
    }
}
```

**アイコン一覧（予定）:**

| カテゴリ | 拡張子 | アイコン |
|---------|--------|---------|
| Rust | .rs | `` |
| Python | .py | `` |
| JavaScript | .js | `` |
| TypeScript | .ts | `` |
| Go | .go | `` |
| JSON | .json | `` |
| YAML | .yaml, .yml | `` |
| TOML | .toml | `` |
| Markdown | .md | `` |
| Git | .git/ | `` |
| Directory | (folder) | `` / `` |
| Default | (other) | `` |

---

## Phase 12: Test Improvements

**リリース:** v0.6.0

### 12.1 CLI引数解析テスト
**優先度:** 高

- [x] 基本オプションテスト
  - `--pick`, `--format`, `--on-select`
  - `--icons`, `--no-icons`
- [x] パス解決テスト
  - ディレクトリ指定、ファイル指定、引数なし
- [x] 環境変数テスト
  - `FILEVIEW_ICONS` の動作確認
- [x] PR: `test: Add CLI argument parsing tests`

### 12.2 アイコンテスト拡充
**優先度:** 高

- [x] 主要言語アイコン（JS, TS, Go, Java, C, C++）
- [x] 特殊ディレクトリ（node_modules, target, tests, docs）
- [x] 特殊ファイル（package.json, Dockerfile, .gitignore）
- [x] エッジケース（大文字小文字、拡張子なし）
- [x] PR: `test: Expand icon mapping tests`

### 12.3 ファイル操作エッジケース
**優先度:** 中

- [x] ユニークパス生成（競合時の連番）
- [x] 特殊文字（スペース、Unicode）
- [x] エラーハンドリング（存在しないファイル削除等）
- [x] PR: `test: Add file operation edge case tests`

### 12.4 Pick出力フォーマット
**優先度:** 中

- [x] lines/null/json 各形式の出力テスト
- [x] 複数ファイル選択時の出力
- [x] 特殊文字エスケープ
- [x] PR: `test: Add pick output format tests`

### 12.5 Gitエラーハンドリング
**優先度:** 中

- [x] 非Gitディレクトリの処理
- [x] ブランチ名の特殊ケース（スラッシュ含む等）
- [x] PR: `test: Add git error handling tests`

### 12.6 ツリーレンダリング
**優先度:** 低

- [x] パス省略ロジック
- [x] 可視高さ計算
- [x] PR: `test: Add tree rendering tests`

**結果:**
- テスト数: 125 → 201（+76テスト）
- カバレッジ: 45% → 70%以上

---

## Phase 13: E2E / Behavioral Tests

**リリース:** v0.7.0

### 背景
v0.6.1で修正したバグ（プレビュースクロール、Enterキー動作）は、既存のユニットテストでは検出できなかった。
これは、KeyAction生成とその実行ロジックの統合部分がテストされていなかったため。

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  キー入力       │ -> │  KeyAction      │ -> │  状態変更       │
│  (テスト済)     │    │  (テスト済)     │    │  (テスト不足)   │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### 13.1 アクションハンドラーの分離
**優先度:** 高

- [x] main.rsからアクション実行ロジックを分離
  - `handler/action.rs` 新規作成
  - `handle_action(state, navigator, action, ...)` 関数
- [x] テスタブルな構造に変更
  - 副作用（ターミナルI/O）を分離
  - 状態変更ロジックをテスト可能に
- [x] PR: `refactor: Extract action execution logic` (#37)

### 13.2 状態遷移テスト
**優先度:** 高

- [x] ToggleExpandの動作テスト
  - ファイル + サイドプレビュー非表示 → フルスクリーンプレビュー
  - ファイル + サイドプレビュー表示 → サイドプレビュー閉じる
  - ディレクトリ → 展開/折りたたみ
- [x] プレビュースクロール状態テスト
  - スクロール増減の動作確認
  - 境界値（0でのサチュレーション）
- [x] PR: `test: Add state transition tests`

### 13.3 シーケンステスト
**優先度:** 中

- [x] 操作シーケンスのテスト
  - `j` → `j` → `o` → `j` → `j` → `q` (ナビゲーション+プレビュー)
  - `P` → `Enter` (サイドプレビュー→閉じる)
  - `/` → `test` → `Enter` → `n` (検索シーケンス)
- [x] 複合操作のテスト
  - 選択 → コピー → 移動 → ペースト
  - リネーム → キャンセル → リネーム → 確定
  - 作成 → 削除
  - 展開 → ナビゲート → 全折りたたみ
  - カット → ペースト（移動）
- [x] PR: `test: Add operation sequence tests`

### 13.4 エッジケーステスト
**優先度:** 中

- [x] 空ディレクトリでの操作
- [ ] 権限のないファイル/ディレクトリ（CI環境での制約により保留）
- [x] シンボリックリンク
- [x] 非常に深いディレクトリ構造
- [x] 境界ナビゲーション（先頭/末尾での移動）
- [x] 特殊文字・Unicode文字のファイル名
- [x] クリップボードエッジケース
- [x] PR: `test: Add edge case behavioral tests` (#40)

**期待される効果:**
- v0.6.1で発見されたようなバグの早期検出
- リファクタリング時の回帰テスト
- 新機能追加時の安全性確保

---

## Phase 14: Side Preview Focus Management

**リリース:** v0.7.0

### 背景
サイドプレビュー表示中に矢印キーを押すと、プレビューのスクロールではなくファイルナビゲーションが発生する。
ユーザーがプレビュー内容を読みたい場合、直感的でない操作が必要になる。

### 14.1 フォーカス状態の管理
**優先度:** 高

- [x] AppStateにフォーカス状態を追加
  - `FocusTarget` enum: `Tree`, `Preview`
  - サイドプレビュー表示時のみ有効
- [x] フォーカス切り替えロジック
  - `Tab`キーでトグル
  - プレビュー非表示時は常にTree

### 14.2 キーボード操作のフォーカス対応
**優先度:** 高

- [x] フォーカスに応じたキー動作
  - Tree: `j/k/↑/↓` でファイルナビゲーション
  - Preview: `j/k/↑/↓` でプレビュースクロール
- [x] その他のキー動作
  - `g/G`: 先頭/末尾へ移動（フォーカス対応）
  - `b/f`: ページスクロール（プレビューフォーカス時）
  - `Esc`: プレビューフォーカス → Treeに戻る

### 14.3 マウス操作のフォーカス対応
**優先度:** 高

- [x] クリックでフォーカス切り替え
  - プレビュー領域クリック → Previewにフォーカス
  - ツリー領域クリック → Treeにフォーカス
- [x] スクロールもフォーカス対応
  - プレビュー領域でスクロール → プレビューをスクロール
  - ツリー領域でスクロール → ファイルリストをスクロール

### 14.4 視覚的フィードバック
**優先度:** 中

- [x] フォーカスインジケーター
  - フォーカスのあるパネルのボーダーをシアン色でハイライト
  - サイドプレビュー表示時のみ表示

### 14.5 テスト
**優先度:** 中

- [x] フォーカス切り替えテスト（8件）
- [x] キー操作のフォーカス対応テスト（8件）
- [x] PR: `feat: Add side preview focus management` (#41)

---

## Phase 15: Terminal Image Protocol Support

**リリース:** v0.8.0

### 背景
現在の半ブロック文字による画像プレビューは解像度が低く、スクリーンショットなどの
細かいディテールがある画像がモザイク状に表示される。モダンなターミナルは高品質な
画像プロトコルをサポートしており、これらに対応することで大幅な品質向上が可能。

### 対応プロトコル

| プロトコル | 対応ターミナル | 優先度 |
|-----------|---------------|--------|
| **Sixel** | Ghostty, iTerm2, WezTerm, mlterm, foot, xterm | 高 |
| **Kitty** | Kitty | 中 |
| **iTerm2** | iTerm2, WezTerm | 中 |
| **半ブロック** | 全ターミナル（フォールバック） | 必須 |

### 15.1 ターミナル検出システム
**優先度:** 高

- [x] ターミナル検出モジュール作成
  - `render/terminal.rs` 新規作成
  - `TerminalKind` enum: Ghostty, Kitty, ITerm2, WezTerm, VSCode, WindowsTerminal, TerminalApp, Alacritty, Foot, Mlterm, Xterm, Unknown
  - `ImageProtocol` enum: Sixel, Kitty, ITerm2, HalfBlock
- [x] 環境変数による検出
  - `TERM_PROGRAM`: iTerm.app, vscode, etc.
  - `GHOSTTY_RESOURCES_DIR`: Ghostty検出
  - `KITTY_WINDOW_ID`: Kitty検出
  - `WEZTERM_EXECUTABLE`: WezTerm検出
  - `WT_SESSION`: Windows Terminal検出
  - `TERM`: xterm-256color, foot, mlterm, etc.
- [x] 最適なプロトコル選択ロジック
  - ターミナルごとに最適なプロトコルを返す
  - 未知のターミナルは半ブロックにフォールバック
- [x] PR: `feat: Add terminal detection system` (#43)

### 15.2 Sixelプロトコル実装
**優先度:** 高

- [x] Sixelエンコーダー実装
  - 依存: なし（手動実装）
  - 画像をSixelシーケンスに変換
  - パレット最適化（256色制限）
  - RLEエンコーディング
- [x] Sixelレンダリング関数
  - `encode_sixel()`, `render_sixel_image()`, `write_sixel()`
- [x] サイズ調整・アスペクト比維持
- [x] PR: `feat: Implement Sixel image protocol` (#43)

### 15.3 Kittyプロトコル実装
**優先度:** 中

- [x] Kittyグラフィックスプロトコル実装
  - Base64エンコード
  - チャンク分割送信
  - 画像配置制御
  - RGB/RGBA/PNG形式サポート
- [x] Kittyレンダリング関数
  - `encode_kitty()`, `render_kitty_image()`, `write_kitty()`
  - `delete_kitty_image()`, `clear_kitty_images()`
- [x] PR: `feat: Implement Kitty image protocol` (#43)

### 15.4 iTerm2プロトコル実装
**優先度:** 中

- [x] iTerm2インラインイメージ実装
  - Base64エンコード
  - OSCエスケープシーケンス
  - PNG/JPEG形式サポート
- [x] iTerm2レンダリング関数
  - `encode_iterm2()`, `render_iterm2_image()`, `write_iterm2()`
- [x] PR: `feat: Implement iTerm2 image protocol` (#43)

### 15.5 統合・自動切り替え
**優先度:** 高

- [x] 統合レンダラー実装
  - `render/image.rs` - 統合画像レンダリングモジュール
  - `ImageRenderResult` enum: Widget (HalfBlock), EscapeSequence (Sixel/Kitty/iTerm2)
  - `render_image()` - プロトコル自動選択
- [x] プロトコル自動選択
  - アプリ起動時にターミナル検出
  - 最適なレンダラーを選択
  - `force_protocol` オプションで上書き可能
- [x] PR: `feat: Integrate image protocol auto-switching` (#43)

### 15.6 テスト
**優先度:** 高

- [x] ターミナル検出テスト
  - 各環境変数パターンのテスト
  - フォールバック動作テスト
- [x] Sixelエンコードテスト
  - 基本的なエンコードテスト
  - パレット生成テスト
  - 境界値テスト（極小/極大画像）
  - マルチカラーテスト
- [x] Kittyプロトコルテスト
  - エンコードテスト
  - チャンク分割テスト
  - RGB/RGBA/PNG形式テスト
- [x] iTerm2プロトコルテスト
  - エンコードテスト
  - Base64データ検証
- [x] 統合テスト
  - プロトコル選択テスト
  - フォールバックテスト
  - エッジケース（1x1画像、大画像スケーリング）
- [x] PR: `test: Add image protocol tests` (#43)

### 15.7 ドキュメント・CLI
**優先度:** 中

- [ ] README更新
  - 対応ターミナル一覧
  - 画像プロトコル説明
- [ ] CLIオプション追加
  - `--image-protocol <auto|sixel|kitty|iterm2|halfblock>`
  - デフォルト: auto
- [ ] PR: `docs: Add image protocol documentation`

### 15.8 main.rs統合（リリースブロッカー）
**優先度:** 最高
**リリース:** v0.8.0

現在、画像プロトコル実装（Sixel/Kitty/iTerm2）はライブラリとして存在するが、
main.rsでは使用されていない。この統合が完了するまでcrates.ioへのリリースは行わない。

#### 技術的課題

ratauiはエスケープシーケンスの直接出力に対応していないため、
画像プロトコルの出力には特別な対応が必要。

**アプローチ案:**
1. **ratauiバイパス方式** - 画像領域のみstdoutに直接出力
2. **カスタムWidget方式** - ratauiのWidgetトレイトを実装
3. **レンダリング後出力方式** - frame.render()後に画像を重ねて出力

#### タスク

- [ ] 15.8.1 出力方式の調査・決定
  - 各方式のPros/Consを検証
  - Ghostty/Kitty/iTerm2での実機テスト
  - PR: `research: Image protocol output strategy`

- [ ] 15.8.2 プロトコル自動選択の統合
  - 起動時にターミナル検出を実行
  - AppStateにImageProtocolを保持
  - PR: `feat: Integrate terminal detection on startup`

- [ ] 15.8.3 画像プレビューの置き換え
  - `render_image_preview()` → 新実装に切り替え
  - フルスクリーンプレビュー対応
  - サイドプレビュー対応
  - PR: `feat: Replace image preview with protocol-aware renderer`

- [ ] 15.8.4 CLIオプション実装
  - `--image-protocol` オプション追加
  - 環境変数 `FILEVIEW_IMAGE_PROTOCOL` サポート
  - PR: `feat: Add --image-protocol CLI option`

- [ ] 15.8.5 統合テスト・E2Eテスト
  - 各ターミナルでの動作確認
  - フォールバック動作テスト
  - PR: `test: Add image protocol integration tests`

- [ ] 15.8.6 リリース準備
  - CHANGELOG更新
  - README更新（対応ターミナル一覧）
  - バージョンをv0.8.0に更新
  - PR: `release: v0.8.0 with image protocol support`

#### 依存関係

```
15.8.1 (調査)
    ↓
15.8.2 (検出統合) → 15.8.4 (CLIオプション)
    ↓
15.8.3 (プレビュー置き換え)
    ↓
15.8.5 (テスト)
    ↓
15.8.6 (リリース)
```

#### リリース判定基準

以下がすべて満たされた場合にv0.8.0をリリース:
- [ ] Ghosttyで高品質画像プレビューが表示される
- [ ] Kittyで高品質画像プレビューが表示される
- [ ] Terminal.app/Alacrittyで半ブロックフォールバックが動作する
- [ ] `--image-protocol halfblock` で強制的に半ブロックに切り替えられる
- [ ] 全テストがパス

---

### 対応ターミナル一覧

| ターミナル | OS | Sixel | Kitty | iTerm2 | フォールバック | 備考 |
|-----------|-----|-------|-------|--------|--------------|------|
| Ghostty | macOS/Linux | ✅ | ❌ | ❌ | ✅ | |
| Kitty | macOS/Linux | ❌ | ✅ | ❌ | ✅ | |
| iTerm2 | macOS | ✅ | ❌ | ✅ | ✅ | |
| WezTerm | 全OS | ✅ | ❌ | ✅ | ✅ | |
| VS Code | 全OS | ✅ | ❌ | ❌ | ✅ | `terminal.integrated.enableImages: true` 必要 |
| Terminal.app | macOS | ❌ | ❌ | ❌ | ✅ | Sixel非対応 |
| Windows Terminal | Windows | ✅ | ❌ | ❌ | ✅ | v1.22+（2025年2月〜） |
| Alacritty | 全OS | ❌ | ❌ | ❌ | ✅ | Sixel対応予定なし |
| foot | Linux | ✅ | ❌ | ❌ | ✅ | |
| mlterm | 全OS | ✅ | ❌ | ❌ | ✅ | |
| xterm | 全OS | ✅* | ❌ | ❌ | ✅ | *コンパイルオプションによる |

### 期待される効果
- Sixel対応ターミナルで高品質な画像プレビュー
- Kittyユーザーへの最高品質プレビュー提供
- 非対応ターミナルでも動作保証（フォールバック）
- ユーザーが手動でプロトコル選択可能

---

## Module Structure

```
src/
├── main.rs
├── lib.rs
├── core/
│   ├── state.rs     # AppState
│   └── mode.rs      # ViewMode
├── tree/
│   ├── node.rs      # TreeEntry
│   └── navigator.rs # TreeNavigator
├── action/
│   ├── file.rs      # ファイル操作
│   └── clipboard.rs # クリップボード
├── render/
│   ├── tree.rs      # ツリー描画
│   ├── preview.rs   # プレビュー
│   ├── status.rs    # ステータスバー
│   ├── icons.rs     # Nerd Fontsアイコン (v0.5.0)
│   ├── terminal.rs  # ターミナル検出 (v0.8.0)
│   ├── sixel.rs     # Sixelプロトコル (v0.8.0)
│   ├── kitty.rs     # Kittyプロトコル (v0.8.0)
│   ├── iterm2.rs    # iTerm2プロトコル (v0.8.0)
│   └── image.rs     # 統合画像レンダラー (v0.8.0)
├── handler/
│   ├── key.rs       # キーイベント
│   ├── mouse.rs     # マウスイベント
│   └── action.rs    # アクション実行 (v0.7.0)
├── integrate/
│   ├── pick.rs      # --pick モード
│   └── callback.rs  # --on-select
└── git/
    └── status.rs    # Git状態管理 (v0.2.0)
```
