# FileView - Implementation Roadmap

## Overview

本ドキュメントは FileView の実装計画を定義する。
各フェーズは独立したPRとしてマージ可能な単位で設計されている。

**参考**: 設計の詳細は [DESIGN.md](./DESIGN.md) を参照。
filetreeを参考にしつつ、Command Pattern / 仮想化ツリー / 非同期処理で差別化。

---

## Phase 1: Project Foundation

- [ ] 1.1 プロジェクト初期化
  - Cargo.toml 作成（ratatui, crossterm, tokio, thiserror等）
  - rustfmt.toml / .clippy.toml 設定
  - .gitignore 追加
  - PR: `chore: Initialize Rust project with dependencies`

- [ ] 1.2 CI/CD パイプライン設定
  - .github/workflows/ci.yml（テスト、clippy、rustfmt）
  - PR: `chore: Set up GitHub Actions CI pipeline`

- [ ] 1.3 基本ディレクトリ構造
  - src/main.rs, src/lib.rs
  - 空のモジュール（app/, command/, tree/, view/, input/, preview/, config/）
  - PR: `chore: Set up project directory structure`

---

## Phase 2: Core Application Framework

- [ ] 2.1 アプリケーション状態定義
  - src/app/state.rs（AppState構造体）
  - InputMode, InputState 定義
  - PR: `feat(app): Define core application state`

- [ ] 2.2 Commandトレイト定義 ★
  - src/command/mod.rs（Command trait）
  - execute(), undo(), description(), is_undoable()
  - PR: `feat(command): Define Command trait for undo/redo`

- [ ] 2.3 コマンド履歴管理 ★
  - src/command/history.rs（CommandHistory）
  - Undo/Redoスタック実装
  - PR: `feat(command): Implement command history for undo/redo`

- [ ] 2.4 イベントループ（tokio） ★
  - src/app/runtime.rs（非同期イベントループ）
  - crossterm イベント処理
  - PR: `feat(app): Implement async event loop with tokio`

- [ ] 2.5 TUI初期化
  - ratatui セットアップ
  - ターミナル初期化/復元
  - PR: `feat(view): Initialize TUI framework`

---

## Phase 3: Tree Data Structure

- [ ] 3.1 TreeNode定義
  - src/tree/mod.rs（TreeNode構造体）
  - path, name, is_dir, depth, expanded, children
  - PR: `feat(tree): Define TreeNode data structure`

- [ ] 3.2 ディレクトリ読み込み
  - 非同期ディレクトリ読み込み（tokio::fs）
  - ソート（フォルダ優先、名前順）
  - 隠しファイルフィルタ
  - PR: `feat(tree): Implement async directory loading`

- [ ] 3.3 仮想化ビューポート ★
  - src/tree/virtualizer.rs（Viewport構造体）
  - 表示範囲のみ計算（O(viewport)）
  - PR: `feat(tree): Implement virtualized viewport`

- [ ] 3.4 展開/折りたたみ
  - 遅延読み込み（Lazy Loading）
  - 展開状態管理
  - PR: `feat(tree): Add lazy expand/collapse`

---

## Phase 4: Tree View UI

- [ ] 4.1 基本ツリー描画
  - src/view/tree_view.rs
  - 仮想化ビューポートからの描画
  - PR: `feat(view): Render virtualized file tree`

- [ ] 4.2 カーソル移動
  - j/k, 矢印キーで移動
  - スクロール自動調整
  - NavigationCommand実装
  - PR: `feat(view): Implement cursor navigation`

- [ ] 4.3 アイコン表示
  - Nerd Fonts アイコン
  - ファイルタイプ別マッピング
  - PR: `feat(view): Add file type icons`

- [ ] 4.4 カラーリング
  - ファイルタイプ別色分け
  - 選択行ハイライト
  - PR: `feat(view): Add syntax coloring`

---

## Phase 5: File Operations (Commands)

- [ ] 5.1 ファイル作成コマンド
  - src/command/file_ops.rs（CreateFileCommand）
  - Undo: 作成したファイルを削除
  - PR: `feat(command): Implement CreateFile command`

- [ ] 5.2 フォルダ作成コマンド
  - CreateDirCommand
  - PR: `feat(command): Implement CreateDir command`

- [ ] 5.3 リネームコマンド
  - RenameCommand
  - Undo: 元の名前に戻す
  - PR: `feat(command): Implement Rename command`

- [ ] 5.4 削除コマンド ★
  - DeleteCommand
  - Undo: バックアップから復元
  - 確認ダイアログ
  - PR: `feat(command): Implement Delete command with undo`

- [ ] 5.5 コピー/ペーストコマンド
  - CopyCommand, PasteCommand
  - クリップボード状態管理
  - PR: `feat(command): Implement Copy and Paste commands`

- [ ] 5.6 移動コマンド
  - MoveCommand（Cut + Paste）
  - Undo: 元の場所に戻す
  - PR: `feat(command): Implement Move command`

- [ ] 5.7 Undo/Redoキーバインド ★
  - `u` でUndo、`Ctrl+r` でRedo
  - ステータスバーに履歴表示
  - PR: `feat(command): Add undo/redo keybindings`

---

## Phase 6: Preview Panel (Async)

- [ ] 6.1 プレビューレイアウト
  - src/view/preview_view.rs
  - 分割パネル
  - PR: `feat(view): Add preview panel layout`

- [ ] 6.2 非同期プレビュー基盤 ★
  - src/preview/mod.rs
  - tokio::spawn でバックグラウンド読み込み
  - debounce（150ms）
  - PR: `feat(preview): Implement async preview infrastructure`

- [ ] 6.3 テキストプレビュー
  - src/preview/text.rs
  - 行数制限、スクロール
  - PR: `feat(preview): Implement text preview`

- [ ] 6.4 画像プレビュー ★
  - src/preview/image.rs
  - Sixel/Kittyプロトコル検出
  - 半ブロックフォールバック
  - PR: `feat(preview): Implement image preview with protocol detection`

- [ ] 6.5 バイナリ情報
  - src/preview/binary.rs
  - サイズ、MIMEタイプ、HEX表示
  - PR: `feat(preview): Show binary file info`

- [ ] 6.6 プレビューキャッシュ
  - LRUキャッシュ実装
  - PR: `perf(preview): Add preview cache`

---

## Phase 7: Status Bar & Path Integration

- [ ] 7.1 ステータスバー
  - src/view/status_view.rs
  - パス、カーソル位置、マーク数
  - Undo/Redo可否表示
  - PR: `feat(view): Add status bar`

- [ ] 7.2 パスコピー（クリップボード）
  - `Y` キーでクリップボードへ
  - OSC 52 エスケープシーケンス
  - PR: `feat(clipboard): Implement path copy`

- [ ] 7.3 マルチセレクト
  - Space でマーク切替
  - マーク済みファイルへのバッチ操作
  - PR: `feat: Implement multi-select with marks`

- [ ] 7.4 標準出力モード ★
  - `--pick` オプション
  - Enter確定時にパスを stdout 出力
  - 終了コード（0=選択, 1=キャンセル, 2=エラー）
  - PR: `feat: Add --pick mode for stdout integration`

- [ ] 7.5 コールバック実行 ★
  - `--on-select` オプション
  - プレースホルダー展開（{path}, {paths}, {dir}）
  - PR: `feat: Add --on-select callback option`

---

## Phase 8: Drag & Drop

- [ ] 8.1 ドロップイベント検知
  - Event::Paste 処理
  - パス正規化
  - PR: `feat: Detect drag and drop events`

- [ ] 8.2 ドロップファイルコピー
  - CopyFromDropCommand
  - Undo対応
  - PR: `feat: Handle file drop with undo support`

---

## Phase 9: Configuration

- [ ] 9.1 設定ファイル読み込み
  - src/config/mod.rs
  - ~/.config/fileview/config.toml
  - serde + toml
  - PR: `feat(config): Load configuration file`

- [ ] 9.2 テーマ設定
  - src/config/theme.rs
  - default / minimal
  - PR: `feat(config): Implement theme system`

- [ ] 9.3 キーバインド設定
  - カスタムキーバインド
  - PR: `feat(config): Add customizable keybindings`

---

## Phase 10: Polish & Release

- [ ] 10.1 エラーハンドリング
  - thiserror によるエラー型定義
  - ユーザーフレンドリーなメッセージ
  - PR: `fix: Improve error handling`

- [ ] 10.2 パフォーマンス最適化
  - 仮想化ツリーのチューニング
  - メモリプロファイリング
  - PR: `perf: Optimize for large directories`

- [ ] 10.3 テスト追加
  - Commandのユニットテスト
  - TreeNodeのテスト
  - PR: `test: Add comprehensive tests`

- [ ] 10.4 ドキュメント整備
  - README.md
  - インストール手順、使用方法
  - PR: `docs: Add README and documentation`

---

## Progress Summary

| Phase | Items | Completed | Key Features |
|-------|-------|-----------|--------------|
| 1. Foundation | 3 | 0 | CI/CD |
| 2. Core Framework | 5 | 0 | Command Pattern |
| 3. Tree Structure | 4 | 0 | Virtualized Tree |
| 4. Tree View | 4 | 0 | Navigation |
| 5. File Operations | 7 | 0 | Undo/Redo |
| 6. Preview | 6 | 0 | Non-blocking, Sixel/Kitty |
| 7. Status & Path | 5 | 0 | --pick, --on-select |
| 8. Drag & Drop | 2 | 0 | - |
| 9. Configuration | 3 | 0 | - |
| 10. Polish | 4 | 0 | Tests |
| **Total** | **43** | **0** | |

---

## Differentiation from filetree

| Phase | filetreeとの違い |
|-------|-----------------|
| 2 | Command Pattern採用（filetreeはState Machine） |
| 3 | 仮想化ツリー（filetreeはflat_list同期） |
| 5 | Undo/Redo対応（filetreeにはない） |
| 6 | ノンブロッキングUI + Sixel/Kitty優先 |
| 7 | --pick/--on-select で外部連携（filetreeは組み込み外部コマンド） |

---

## Notes

- 各PRは1つの機能または1つの論理的変更に限定
- ★マークは filetree との主要な差別化ポイント
- Phase間の依存: 1 → 2 → 3 → 4 → 5/6/7（並行可能）→ 8 → 9 → 10
- Git統合は意図的に除外（ミニマル設計）
