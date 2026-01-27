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
| 12. Test Improvements | 6 | 0 |
| **Total** | **35** | **29** |

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
| v0.6.0 | Test improvements | 🚧 Planned |

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

- [ ] 基本オプションテスト
  - `--pick`, `--format`, `--on-select`
  - `--icons`, `--no-icons`
- [ ] パス解決テスト
  - ディレクトリ指定、ファイル指定、引数なし
- [ ] 環境変数テスト
  - `FILEVIEW_ICONS` の動作確認
- [ ] PR: `test: Add CLI argument parsing tests`

### 12.2 アイコンテスト拡充
**優先度:** 高

- [ ] 主要言語アイコン（JS, TS, Go, Java, C, C++）
- [ ] 特殊ディレクトリ（node_modules, target, tests, docs）
- [ ] 特殊ファイル（package.json, Dockerfile, .gitignore）
- [ ] エッジケース（大文字小文字、拡張子なし）
- [ ] PR: `test: Expand icon mapping tests`

### 12.3 ファイル操作エッジケース
**優先度:** 中

- [ ] ユニークパス生成（競合時の連番）
- [ ] 特殊文字（スペース、Unicode）
- [ ] エラーハンドリング（存在しないファイル削除等）
- [ ] PR: `test: Add file operation edge case tests`

### 12.4 Pick出力フォーマット
**優先度:** 中

- [ ] lines/null/json 各形式の出力テスト
- [ ] 複数ファイル選択時の出力
- [ ] 特殊文字エスケープ
- [ ] PR: `test: Add pick output format tests`

### 12.5 Gitエラーハンドリング
**優先度:** 中

- [ ] 非Gitディレクトリの処理
- [ ] ブランチ名の特殊ケース（スラッシュ含む等）
- [ ] PR: `test: Add git error handling tests`

### 12.6 ツリーレンダリング
**優先度:** 低

- [ ] パス省略ロジック
- [ ] 可視高さ計算
- [ ] PR: `test: Add tree rendering tests`

**目標:**
- テスト数: 64 → 127（+63テスト）
- カバレッジ: 45% → 70%以上

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
│   └── status.rs    # ステータスバー
├── handler/
│   ├── key.rs       # キーイベント
│   └── mouse.rs     # マウスイベント
├── integrate/
│   ├── pick.rs      # --pick モード
│   └── callback.rs  # --on-select
└── git/
    └── status.rs    # Git状態管理 (v0.2.0)
```
