# FileView (fv)

[![Crates.io](https://img.shields.io/crates/v/fileview.svg)](https://crates.io/crates/fileview)
[![Downloads](https://img.shields.io/crates/d/fileview.svg)](https://crates.io/crates/fileview)
[![CI](https://github.com/Hiro-Chiba/fileview/actions/workflows/ci.yml/badge.svg)](https://github.com/Hiro-Chiba/fileview/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![MSRV](https://img.shields.io/badge/MSRV-1.90-blue.svg)](https://www.rust-lang.org)

> 設定不要のターミナルファイルブラウザ（画像プレビュー自動検出）

[English](README.md) | 日本語

## なぜ fv？

```
軽量 ◄───────────────────────────────► 高機能

  nnn    lf    fv    ranger    yazi
 3.4MB  12MB  8MB    28MB     38MB
```

- **設定不要** - インストールして即使える
- **画像自動プレビュー** - Kitty/iTerm2/Sixel/Halfblocks を自動検出
- **高速** - 起動 2.3ms、メモリ 8MB（ranger: 400ms/28MB）
- **バッテリー同梱** - Git連携、シンタックスハイライト、PDFプレビュー
- **Vimキーバインド** - j/k/h/l で操作

## クイックスタート

```bash
cargo install fileview
fv
```

## 3分で始めるAIワークフロー（推奨）

```bash
# 1) Claude MCP連携を一度だけ設定
fv init claude

# 2) AI向けコンテキストを生成
fv --context-pack review --agent claude

# 3) 前回セッションを復元
fv --resume-ai-session
```

関連ドキュメント:
- [Claude Codeガイド](docs/CLAUDE_CODE_ja.md)
- [競合スコアカード（週次更新）](docs/COMPETITIVE_SCORECARD.md)
- [リリース方針](docs/RELEASE_POLICY.md)

## 機能

| 機能 | 説明 |
|------|------|
| ツリーナビゲーション | Vimキーで展開/折りたたみ |
| 複数選択 | バッチ操作対応 |
| プレビューパネル | テキスト、画像、アーカイブ、PDF、Hex |
| シンタックスハイライト | 100+言語対応 |
| ファイル操作 | 作成、リネーム、削除、コピー/ペースト |
| ファジーファインダー | `Ctrl+P` で素早く検索 |
| マウス対応 | クリック、スクロール、ドラッグ |
| Nerd Fonts | アイコンはデフォルト有効 |

## 画像プレビュー

ターミナルを自動検出:

| ターミナル | プロトコル |
|-----------|-----------|
| Kitty / Ghostty / Konsole | Kitty Graphics |
| iTerm2 / WezTerm / Warp | iTerm2 Inline |
| Foot / Windows Terminal | Sixel |
| VS Code / Alacritty | Halfblocks |

## キーバインド（クイックリファレンス）

| キー | 動作 |
|------|------|
| `j/k` | 上下移動 |
| `h/l` | 折りたたみ/展開 |
| `g/G` | 先頭/末尾 |
| `Space` | マーク切り替え |
| `/` | 検索 |
| `Ctrl+P` | ファジーファインダー |
| `P` | プレビューパネル |
| `Ctrl+Shift+Enter` | レビュー用コンテキストパックをコピー |
| `q` | 終了 |

**[完全なキーバインド一覧](docs/KEYBINDINGS_ja.md)**

## Claude Code 連携

FileView は Claude Code との AI ペアプログラミングに最適化されています:

```bash
# AI向けプロジェクトコンテキスト
fv --context

# ディレクトリツリーをコンテキストとして出力
fv --tree --depth 2 ./src

# ファイルをすばやく選択
selected=$(fv --select-mode --multi)

# Claude 向けフォーマットでファイル内容をコピー
# fileview 内で Ctrl+Y を押すとシンタックスヒント付きでコピー
```

### スマート選択

| キー | 動作 |
|------|------|
| `Ctrl+G` | Git変更ファイルを一括選択 |
| `Ctrl+T` | テストペアファイルを選択 |

### MCP サーバー

FileView を Claude Code の MCP サーバーとして使用:

```json
{
  "mcpServers": {
    "fileview": {
      "command": "fv",
      "args": ["--mcp-server"]
    }
  }
}
```

**MCP 2.0 ツール (21ツール):**

| カテゴリ | ツール |
|----------|-------|
| ファイル | `list_directory`, `get_tree`, `read_file`, `read_files`, `write_file`, `delete_file`, `search_code` |
| Git | `get_git_status`, `get_git_diff`, `git_log`, `stage_files`, `create_commit` |
| 解析 | `get_file_symbols`, `get_definitions`, `get_references`, `get_diagnostics` |
| 依存関係 | `get_dependency_graph`, `get_import_tree`, `find_circular_deps` |
| コンテキスト | `get_smart_context`, `estimate_tokens`, `compress_context` |
| プロジェクト | `run_build`, `run_test`, `run_lint`, `get_project_stats` |

**[MCPドキュメント詳細](docs/CLAUDE_CODE.md)**

## CLIオプション

```bash
fv [OPTIONS] [PATH]

オプション:
  -p, --pick          Pickモード: 選択パスを出力
  -f, --format FMT    出力形式: lines, null, json
  --stdin             stdinからパスを読み込み
  --on-select CMD     選択時にコマンド実行
  --choosedir         終了時にディレクトリを出力
  --no-icons          Nerd Fontsアイコンを無効化

Claude Code連携:
  -t, --tree          ディレクトリツリーをstdoutに出力
  --depth N           ツリー深度を制限
  --context           AI向けプロジェクトコンテキストを出力
  --context-pack P    コンテキストパック preset 出力 (minimal/review/debug/refactor/incident/onboarding)
  --context-format F  コンテキスト形式: ai-md, jsonl
  --agent A           エージェントプロファイル: claude, codex, cursor
  --token-budget N    コンテキストのトークン予算
  --include-git-diff  コンテキストにgit diff要約を含める
  --include-tests     推定テストファイルを含める
  --context-depth N   フォールバックスキャン深度
  --with-content      出力にファイル内容を含める
  --select-mode       シンプル選択モード
  --multi             複数選択を許可
  --select-related F  対象ファイルに関連するファイルを出力
  --explain-selection 関連ファイル選定理由を出力
  --resume-ai-session [NAME]
                      名前付きAIセッションを復元（省略時: ai）
  --mcp-server        MCPサーバーとして起動
  benchmark ai        AI向けベンチマークを実行
  init claude         Claude設定にfileview MCPエントリを自動追加
```

### 終了コード

| コード | 意味 |
|--------|------|
| 0 | 成功 |
| 1 | キャンセル（Pickモード） |
| 2 | 実行時エラー |
| 3 | 不正な引数 |

## シェル連携

```bash
# .bashrc または .zshrc に追加
fvcd() {
  local dir=$(fv --choosedir "$@")
  [ -n "$dir" ] && [ -d "$dir" ] && cd "$dir"
}
```

## インストールオプション

```bash
# 標準インストール
cargo install fileview

# Chafaサポート付き（基本ターミナルでの高品質画像）
brew install chafa  # または apt install libchafa-dev
cargo install fileview --features chafa
```

## 安定性

- 現在のチャネル: `stable`（`2.1.0`）
- stable移行条件は `docs/STABILITY.md` に明記しています。
- 2026-02-04 時点で条件を満たし、stableリリース承認済みです。

## Lua プラグインシステム

Lua スクリプトで FileView を拡張できます:

```lua
-- ~/.config/fileview/plugins/init.lua

-- 起動時通知
fv.notify("プラグイン読み込み完了!")

-- イベントに反応
fv.on("file_selected", function(path)
    if path and path:match("%.secret$") then
        fv.notify("警告: 秘密ファイル!")
    end
end)

-- カスタムコマンド
fv.register_command("copy-path", function()
    local file = fv.current_file()
    if file then
        fv.set_clipboard(file)
        fv.notify("コピー: " .. file)
    end
end)
```

**[プラグイン API リファレンス](docs/PLUGINS_ja.md)**

## ドキュメント

- [Claude Code 連携](docs/CLAUDE_CODE.md) - AI ペアプログラミングガイド
- [Claude Code 連携（日本語）](docs/CLAUDE_CODE_ja.md) - AI ペアプログラミングガイド
- [キーバインド](docs/KEYBINDINGS_ja.md) - 完全なキーバインド一覧
- [プラグイン](docs/PLUGINS_ja.md) - Lua プラグインシステム
- [競合比較](docs/COMPARISON.md) - yazi, lf, ranger, nnn との比較
- [ロードマップ](docs/ROADMAP.md) - 今後の方針とリリース履歴
- [ベンチマーク](docs/BENCHMARKS.md) - パフォーマンスデータ
- [セキュリティ](docs/SECURITY.md) - セキュリティモデル
- [安定性](docs/STABILITY.md) - リリースチャネル方針とalpha終了条件
- [リリースポリシー](docs/RELEASE_POLICY.md) - バージョニングと運用ルール

## ライセンス

MIT
