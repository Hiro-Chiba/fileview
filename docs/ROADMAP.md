# FileView - Implementation Roadmap

## Overview

本ドキュメントは FileView の実装計画を定義する。
各フェーズは独立したPRとしてマージ可能な単位で設計されている。

---

## Phase 1: Project Foundation

- [ ] 1.1 プロジェクト初期化
  - Cargo.toml 作成（依存クレート定義）
  - rustfmt.toml / clippy.toml 設定
  - .gitignore 追加
  - PR: `chore: Initialize Rust project with dependencies`

- [ ] 1.2 CI/CD パイプライン設定
  - .github/workflows/ci.yml（テスト、lint、format チェック）
  - PRマージ前の必須チェック設定
  - PR: `chore: Set up GitHub Actions CI pipeline`

- [ ] 1.3 基本ディレクトリ構造
  - src/main.rs（エントリーポイントスタブ）
  - src/lib.rs（モジュール宣言）
  - 空のモジュールファイル作成
  - PR: `chore: Set up project directory structure`

---

## Phase 2: Core Application Framework

- [ ] 2.1 アプリケーション状態定義
  - src/app.rs（AppState構造体）
  - 基本的な状態フィールド定義
  - PR: `feat(app): Define core application state`

- [ ] 2.2 イベントループ基盤
  - src/event/mod.rs
  - src/event/handler.rs（イベントハンドラー骨格）
  - 終了処理（q キー）
  - PR: `feat(event): Implement basic event loop`

- [ ] 2.3 TUI初期化
  - ratatui セットアップ
  - ターミナル初期化/復元
  - 空のレイアウト描画
  - PR: `feat(ui): Initialize TUI framework`

---

## Phase 3: File System Layer

- [ ] 3.1 ファイルエントリ定義
  - src/fs/mod.rs
  - src/fs/entry.rs（FileEntry構造体）
  - ファイル/ディレクトリの判別
  - PR: `feat(fs): Define file entry data structures`

- [ ] 3.2 ディレクトリ読み込み
  - ディレクトリ内容の取得
  - ソート処理（フォルダ優先、名前順）
  - 隠しファイルフィルタリング
  - PR: `feat(fs): Implement directory reading`

- [ ] 3.3 ツリー構造構築
  - 再帰的なツリー構造
  - 遅延読み込み（Lazy Loading）対応
  - PR: `feat(fs): Build lazy-loaded tree structure`

---

## Phase 4: Tree View UI

- [ ] 4.1 基本ツリー表示
  - src/ui/tree.rs
  - 階層インデント表示
  - フォルダ/ファイル識別表示
  - PR: `feat(ui): Render basic file tree`

- [ ] 4.2 フォルダ展開/折りたたみ
  - 展開状態の管理
  - Enter キーでトグル
  - PR: `feat(ui): Add folder expand/collapse`

- [ ] 4.3 カーソル移動
  - j/k または 矢印キーで上下移動
  - スクロール処理
  - PR: `feat(ui): Implement cursor navigation`

- [ ] 4.4 アイコン表示
  - Nerd Fonts アイコン対応
  - ファイルタイプ別アイコン
  - PR: `feat(ui): Add file type icons`

- [ ] 4.5 カラーリング
  - ファイルタイプ別色分け
  - 選択行ハイライト
  - PR: `feat(ui): Add syntax coloring`

---

## Phase 5: File Operations

- [ ] 5.1 ファイル作成
  - `a` キーで新規作成モード
  - ファイル名入力UI
  - PR: `feat(fs): Implement file creation`

- [ ] 5.2 フォルダ作成
  - `A` キーでフォルダ作成
  - PR: `feat(fs): Implement folder creation`

- [ ] 5.3 リネーム
  - `r` キーでリネームモード
  - インライン編集UI
  - PR: `feat(fs): Implement rename operation`

- [ ] 5.4 削除
  - `d` キーで削除
  - 確認ダイアログ
  - PR: `feat(fs): Implement delete with confirmation`

- [ ] 5.5 コピー/ペースト
  - `y` キーでコピー（パス保持）
  - `p` キーでペースト
  - PR: `feat(fs): Implement copy and paste`

- [ ] 5.6 移動
  - `m` キーで移動モード
  - 移動先選択UI
  - PR: `feat(fs): Implement move operation`

---

## Phase 6: Preview Panel

- [ ] 6.1 プレビューレイアウト
  - src/ui/preview.rs
  - 分割パネルレイアウト
  - PR: `feat(ui): Add preview panel layout`

- [ ] 6.2 テキストプレビュー
  - テキストファイル内容表示
  - 行数制限
  - PR: `feat(preview): Implement text file preview`

- [ ] 6.3 シンタックスハイライト
  - syntect によるハイライト
  - 言語自動検出
  - PR: `feat(preview): Add syntax highlighting`

- [ ] 6.4 画像プレビュー
  - Sixel/Kitty プロトコル検出
  - サムネイル生成・表示
  - PR: `feat(preview): Implement image preview`

- [ ] 6.5 バイナリ情報表示
  - ファイルサイズ、MIMEタイプ
  - 先頭バイトのHEX表示
  - PR: `feat(preview): Show binary file info`

- [ ] 6.6 非同期プレビュー
  - debounce 処理
  - キャッシュ機構
  - PR: `perf(preview): Add async loading with cache`

---

## Phase 7: Status Bar & Path Integration

- [ ] 7.1 ステータスバー
  - src/ui/statusbar.rs
  - 現在パス、ファイル数表示
  - PR: `feat(ui): Add status bar`

- [ ] 7.2 パスコピー
  - `Y` キーでパスをクリップボードへ
  - OSC 52 エスケープシーケンス
  - PR: `feat: Implement path copy to clipboard`

- [ ] 7.3 外部連携
  - 環境変数 `$FILEVIEW_SELECTED` 設定
  - 外部コマンド実行（`!` キー）
  - PR: `feat: Add external command integration`

---

## Phase 8: Drag & Drop

- [ ] 8.1 ドロップイベント検知
  - ターミナルのドロップイベント対応
  - パス文字列のパース
  - PR: `feat: Detect drag and drop events`

- [ ] 8.2 ファイルコピー処理
  - ドロップされたファイルを現在ディレクトリへコピー
  - 進捗表示
  - PR: `feat: Handle file drop and copy`

---

## Phase 9: Configuration

- [ ] 9.1 設定ファイル読み込み
  - src/config/mod.rs
  - ~/.config/fileview/config.toml
  - PR: `feat(config): Load configuration file`

- [ ] 9.2 テーマ設定
  - src/config/theme.rs
  - カラースキーム定義
  - PR: `feat(config): Implement theme system`

- [ ] 9.3 キーバインド設定
  - カスタムキーバインド対応
  - PR: `feat(config): Add customizable keybindings`

---

## Phase 10: Polish & Optimization

- [ ] 10.1 エラーハンドリング強化
  - ユーザーフレンドリーなエラー表示
  - リカバリー処理
  - PR: `fix: Improve error handling and recovery`

- [ ] 10.2 パフォーマンス最適化
  - 大規模ディレクトリ対応
  - メモリ使用量削減
  - PR: `perf: Optimize for large directories`

- [ ] 10.3 テスト追加
  - ユニットテスト
  - 統合テスト
  - PR: `test: Add unit and integration tests`

- [ ] 10.4 ドキュメント整備
  - README.md 作成
  - 使用方法、インストール手順
  - PR: `docs: Add README and usage documentation`

---

## Progress Summary

| Phase | Items | Completed |
|-------|-------|-----------|
| 1. Foundation | 3 | 0 |
| 2. Core Framework | 3 | 0 |
| 3. File System | 3 | 0 |
| 4. Tree View | 5 | 0 |
| 5. File Operations | 6 | 0 |
| 6. Preview | 6 | 0 |
| 7. Status & Path | 3 | 0 |
| 8. Drag & Drop | 2 | 0 |
| 9. Configuration | 3 | 0 |
| 10. Polish | 4 | 0 |
| **Total** | **38** | **0** |

---

## Notes

- 各PRは原則として1つの機能または1つの論理的変更に限定する
- PRは依存関係の順序でマージする（番号順が推奨）
- Phase間の依存: 1 → 2 → 3 → 4 → 5/6/7（並行可能）→ 8 → 9 → 10
