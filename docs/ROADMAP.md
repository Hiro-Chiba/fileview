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

### 9.1 Git ステータス表示 ⭐⭐⭐

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

### 9.2 ディレクトリ情報表示 ⭐⭐

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

### 9.3 Hex プレビュー ⭐⭐

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

## Phase 10: Performance (v0.5.x - v0.6.x) 🚀

**目標スコア:** パフォーマンス 7.5 → 9.0

### 10.1 プレビューキャッシュ改善 ⭐⭐

**優先度:** 高
**リリース:** v0.5.0
**種別:** Minor

- [ ] render/preview.rs 拡張
  - パス単位でのキャッシュ管理
  - キャッシュ有効期限（mtime比較）
  - メモリ上限管理（LRU方式）
- [ ] PR: `perf(preview): Add path-based preview caching`

**実装詳細:**
```rust
pub struct PreviewCache {
    text_cache: HashMap<PathBuf, (SystemTime, TextPreview)>,
    image_cache: HashMap<PathBuf, (SystemTime, ImagePreview)>,
    max_entries: usize,
}
```

---

### 10.2 ツリー展開の遅延読み込み最適化 ⭐⭐

**優先度:** 高
**リリース:** v0.5.1
**種別:** Minor

- [ ] tree/navigator.rs 拡張
  - 展開時のみ子要素を読み込み
  - 読み込み済みフラグ管理
  - 大規模ディレクトリの分割読み込み（100件単位）
- [ ] PR: `perf(tree): Optimize lazy loading for large directories`

---

### 10.3 非同期ファイル読み込み ⭐⭐⭐

**優先度:** 高
**リリース:** v0.6.0
**種別:** **Major**

- [ ] Cargo.toml
  - tokio依存追加
- [ ] render/preview.rs 拡張
  - 非同期プレビュー読み込み
  - ローディング表示
  - キャンセル対応
- [ ] PR: `feat(preview): Add async file loading with tokio`

**実装詳細:**
```rust
pub async fn load_preview_async(path: &Path) -> anyhow::Result<PreviewContent> {
    tokio::fs::read(path).await?
}
```

---

### 10.4 Gitステータスのバックグラウンド更新 ⭐⭐

**優先度:** 中
**リリース:** v0.6.1
**種別:** Minor

- [ ] git/status.rs 拡張
  - バックグラウンドスレッドでの更新
  - 初期表示は即座に、Git情報は後から反映
  - 更新中インジケーター
- [ ] PR: `perf(git): Add background git status refresh`

---

## Phase 11: UX改善 (v0.7.x - v0.8.x) ✨

**目標スコア:** UX 8.0 → 9.0

### 11.1 Nerd Fontアイコン対応 ⭐⭐⭐

**優先度:** 高
**リリース:** v0.7.0
**種別:** **Major**

- [ ] render/icons.rs 新規作成
  - ファイル拡張子→アイコンマッピング
  - ディレクトリアイコン
  - 特殊ファイルアイコン（.git, node_modules等）
- [ ] render/tree.rs 拡張
  - アイコン表示オプション
  - `--no-icons` フラグ
- [ ] PR: `feat(render): Add Nerd Font icon support`

**アイコン例:**
```
 src/
 main.rs
 Cargo.toml
 README.md
 .gitignore
```

---

### 11.2 ブックマーク機能 ⭐⭐

**優先度:** 中
**リリース:** v0.7.1
**種別:** Minor

- [ ] core/bookmark.rs 新規作成
  - ブックマーク保存（`m` + a-z）
  - ブックマーク移動（`'` + a-z）
  - ~/.config/fileview/bookmarks.json に永続化
- [ ] PR: `feat(core): Add bookmark functionality`

**キーバインド:**
| キー | 動作 |
|------|------|
| `ma` | 現在位置をブックマーク 'a' に保存 |
| `'a` | ブックマーク 'a' に移動 |

---

### 11.3 外部コマンド実行 ⭐⭐⭐

**優先度:** 高
**リリース:** v0.8.0
**種別:** **Major**

- [ ] handler/command.rs 新規作成
  - `!` キーでコマンドモード開始
  - プレースホルダー展開（{path}, {dir}, {name}）
  - ターミナル一時解放→コマンド実行→復帰
- [ ] PR: `feat(handler): Add external command execution`

**使用例:**
```
!vim {path}      # 選択ファイルをvimで開く
!code {dir}      # 親ディレクトリをVS Codeで開く
```

---

### 11.4 コマンド履歴保存 ⭐⭐

**優先度:** 中
**リリース:** v0.8.1
**種別:** Minor

- [ ] integrate/history.rs 新規作成
  - コマンド履歴の永続化（~/.config/fileview/history.txt）
  - 履歴サイズ制限（100件）
  - ↑↓キーで履歴ナビゲーション
- [ ] PR: `feat(integrate): Add command history persistence`

---

### 11.5 ファジー検索 ⭐⭐

**優先度:** 中
**リリース:** v0.8.2
**種別:** Minor

- [ ] handler/search.rs 拡張
  - ファジーマッチングアルゴリズム
  - スコアベースのソート
  - リアルタイム絞り込み
- [ ] PR: `feat(handler): Add fuzzy search`

**アルゴリズム:**
```rust
fn fuzzy_match(pattern: &str, target: &str) -> Option<i32> {
    // スコアベースのファジーマッチング
}
```

---

## Phase 12: テスト強化 (v0.9.x - v1.0.0) 🧪

**目標スコア:** テスト 7.0 → 9.0

### 12.1 レンダリングテスト ⭐⭐

**優先度:** 高
**リリース:** v0.9.0
**種別:** Minor

- [ ] tests/render_test.rs 新規作成
  - ratatui TestBackend使用
  - ツリー描画の検証
  - ステータスバーの検証
  - プレビュー描画の検証
- [ ] PR: `test(render): Add rendering tests with TestBackend`

---

### 12.2 プロパティベーステスト ⭐⭐

**優先度:** 中
**リリース:** v0.9.1
**種別:** Minor

- [ ] Cargo.toml
  - proptest依存追加
- [ ] tests/property_test.rs 新規作成
  - ランダム入力でのクラッシュテスト
  - 状態遷移の整合性検証
- [ ] PR: `test: Add property-based tests with proptest`

---

### 12.3 E2Eシナリオテスト ⭐⭐

**優先度:** 中
**リリース:** v0.9.2
**種別:** Minor

- [ ] tests/e2e_test.rs 新規作成
  - 完全なユーザーシナリオのテスト
  - ファイル作成→編集→削除フロー
  - 検索→選択→コピーフロー
- [ ] PR: `test: Add E2E scenario tests`

---

### 12.4 ベンチマークテスト + v1.0リリース ⭐⭐⭐

**優先度:** 高
**リリース:** v1.0.0
**種別:** **Major**

- [ ] benches/benchmark.rs 新規作成
  - 大規模ディレクトリ読み込み時間
  - プレビュー生成時間
  - Git状態取得時間
- [ ] CI統合
  - パフォーマンス劣化検出
- [ ] PR: `test: Add benchmark tests`
- [ ] v1.0.0安定版リリース

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
| 10. Performance | 4 | 0 |
| 11. UX改善 | 5 | 0 |
| 12. テスト強化 | 4 | 0 |
| **Total** | **36** | **23** |

---

## Release Plan

| Version | Feature | Status |
|---------|---------|--------|
| v0.1.x | Initial release | ✅ Published |
| v0.2.0 | Git status display | ✅ Published |
| v0.3.0 | Directory info | ✅ Published |
| v0.4.0 | Hex preview | ✅ Published |
| v0.5.0 | Preview caching | 📋 Planned |
| v0.5.1 | Lazy loading optimization | 📋 Planned |
| v0.6.0 | Async file loading | 📋 Planned |
| v0.6.1 | Background git refresh | 📋 Planned |
| v0.7.0 | Nerd Font icons | 📋 Planned |
| v0.7.1 | Bookmarks | 📋 Planned |
| v0.8.0 | External command execution | 📋 Planned |
| v0.8.1 | Command history | 📋 Planned |
| v0.8.2 | Fuzzy search | 📋 Planned |
| v0.9.0 | Rendering tests | 📋 Planned |
| v0.9.1 | Property-based tests | 📋 Planned |
| v0.9.2 | E2E tests | 📋 Planned |
| v1.0.0 | Benchmark tests + Stable | 📋 Planned |

---

## Score Projection

| Phase | Version | Expected Score |
|-------|---------|---------------|
| Current | v0.4.2 | 64/80 |
| Phase 10 Complete | v0.6.1 | 68/80 |
| Phase 11 Complete | v0.8.2 | 73/80 |
| Phase 12 Complete | v1.0.0 | 77/80 |

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
