# 変更履歴

このプロジェクトの主な変更はこのファイルに記録されます。

フォーマットは [Keep a Changelog](https://keepachangelog.com/ja/1.1.0/) に基づいており、
[セマンティックバージョニング](https://semver.org/lang/ja/) に準拠しています。

## [2.7.1] - 2026-05-16

純粋なリファクタとリリースツール修正のみ。挙動・API・依存・バイナリサイズ・
feature の変更はなし。`CHANGELOG.md` の英語版が一次情報。

### 内部
- `src/handler/action/tests.rs` (2928 LOC) を `tests/` ディレクトリに
  6 ファイル分割 (basic / state_transition / sequence / edge_cases /
  focus / scroll_bounds + 共有ヘルパー入りの mod.rs)
- `src/render/status.rs` (1644 LOC) を `status/` ディレクトリに
  6 ファイル分割 (bar / format / popup / help / todos / mod)

### 修正
- `release.yml` の awk が range pattern の重複で section 抽出に失敗していた
  問題を state-machine 方式に書き直して修正

## [2.7.0] - 2026-05-16

AI agent や shell スクリプトから TUI/MCP server を介さず fileview を呼べる
非対話型フラグを 3 つ追加。`CHANGELOG.md` の英語版が一次情報。

### 追加
- `fv --tokens <path>` — ファイルの cl100k_base token 推定値を stdout に出力
- `fv --snapshot-create <name>` / `fv --snapshot-diff <name>` — 作業ツリーの
  manifest (path/size/mtime) を `.fileview/snapshots/<name>.json` に保存し、
  後から `+ added / - removed / M modified` の差分を表示
- `fv --watch <path> [--watch-timeout-secs N]` — 指定ファイルが変更されるまで
  block、変更時に path を print して exit 0、timeout 時 exit 1

### 内部
- `integrate::snapshot` / `integrate::watch` モジュール新設、ユニットテスト
  9 件 + e2e テスト 12 件、合計 21 件追加

## [2.6.0] - 2026-05-11

AI 駆動ワークフローと slim ビルドに焦点を当てたリリース。8 つの新機能と
4 つのオプショナル Cargo feature を追加。`CHANGELOG.md` の英語版が一次情報
であり、ここでは概要のみ記載する。

### 追加
- AI 用 ignore ファイル合成 (`fv init aiignore`)
- マーク中ファイルの context budget bar
- 起動時のリポジトリ fingerprint
- diff-aware tree (`fv --diff [REVSPEC]`)
- TODO/FIXME aggregator (`\` キー)
- AI session replay (`fv --replay [ID]`)

### 変更
- syntect / tiktoken のバックグラウンド warmup で初回ラグを除去
- GitHub Releases の形式統一（英語、Keep a Changelog 準拠）

### ビルド
- 4 つの Cargo feature 化: `ai`, `archive`, `clipboard`, `lua`
- `cargo install fileview --no-default-features` で slim ビルドが可能

詳細は `CHANGELOG.md` を参照。

## [2.3.2] - 2026-02-12

### 変更

- READMEに不足していたCLIオプションを追加（`--hidden`, `--session`, `--selection-path`, `plugin`コマンド）
- READMEに環境変数セクションを追加
- 安定性バージョンを2.3.2に更新
- 日本語READMEのスクリーンショットをリサイズして中央揃え

## [2.3.1] - 2026-02-12

### 変更

- Dependabotによる依存関係の更新（mlua, tiktoken-rs, zip, dirs, toml, notify-debouncer-miniなど）
- CIアクションの更新（checkout v6, upload-artifact v6, download-artifact v7, codecov-action v5）
- READMEのスクリーンショットをリサイズして中央揃え

### 修正

- Windowsテストの互換性改善

## [2.2.3] - 2026-02-04

### 変更

- 開発/運用ルールを整理し、`main` を最終スナップショット向けのシンプル構成にクリーンアップ

### 注記

- このリリースを区切りとして、以後の開発は一時停止

## [2.2.2] - 2026-02-04

### 変更

- Narrow UI (`25-39` 列) でも Nerd Fonts のファイル/フォルダアイコンを表示するよう改善
- ドキュメントの密度テーブルを実装仕様に合わせて更新

## [2.2.1] - 2026-02-04

### 追加

- `init claude`: Claude設定への `fileview` MCPエントリ自動初期化
- `--resume-ai-session [NAME]`: 名前付きAIセッション復元（既定: `ai`）
- `Ctrl+Shift+Enter`: review context pack のクイックコピー
- `docs/CLAUDE_CODE_ja.md`: Claude連携ガイド（日本語）を追加

### 変更

- `docs/DEVELOPMENT_HISTORY.md` を最新リリース履歴（`v2.1.0` まで）に同期
- `docs/ROADMAP.md` を管理対象として整備
- `README.md` / `README_ja.md` のAI導線とリンクを更新

### 修正

- `notify-types` ロック整合を修正し、公開フローを安定化

## [2.2.0] - 2026-02-04

### 注記

- このタグは公開時のロック不整合対応のため、実運用上は `2.2.1` を最新安定版として利用してください。

## [2.1.0] - 2026-02-04

### 追加

- stable昇格基準の文書化（`docs/STABILITY.md`）

### 変更

- バージョン表記を `2.1.0`（stable）へ移行

## [2.0.0-alpha] - 2026-02-03

### 追加

- **Ultra-Narrow UI (20文字幅対応)**
- **MCP 2.0: AI-Native Development Tools** - 21ツール、6カテゴリ
- **統一エラーハンドリング** (`src/error.rs`)

### 変更

- **MCPハンドラー再構成**: モノリシックな `handlers.rs` を分割
- **プレビューモジュール分割**: 1880行の `preview.rs` を11モジュールに

## 以前のバージョン

詳細な履歴は英語版 [CHANGELOG.md](CHANGELOG.md) を参照してください。
