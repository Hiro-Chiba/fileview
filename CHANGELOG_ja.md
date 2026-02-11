# 変更履歴

このプロジェクトの主な変更はこのファイルに記録されます。

フォーマットは [Keep a Changelog](https://keepachangelog.com/ja/1.1.0/) に基づいており、
[セマンティックバージョニング](https://semver.org/lang/ja/) に準拠しています。

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
