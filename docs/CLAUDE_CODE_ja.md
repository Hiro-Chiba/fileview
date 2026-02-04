# Claude Code 連携ガイド（日本語）

FileView は Claude Code / Codex などのAIコーディング支援を前提にしたワークフローを提供します。

## クイックスタート

```bash
# プロジェクト全体の文脈を出力
fv --context

# ツリー出力
fv --tree --depth 2 ./src

# ファイル選択
selected=$(fv --select-mode --multi)

# MCPサーバーとして起動
fv --mcp-server
```

## 主要CLI（AI向け）

| オプション | 説明 |
|---|---|
| `--context` | AI向けプロジェクトコンテキストを出力 |
| `--context-pack P` | コンテキストパック出力（`minimal` / `review` / `debug` / `refactor` / `incident` / `onboarding`） |
| `--context-format F` | 形式（`ai-md` / `jsonl`） |
| `--agent A` | エージェントプロファイル（`claude` / `codex` / `cursor`） |
| `--token-budget N` | トークン予算 |
| `--include-git-diff` | git差分要約を含める |
| `--include-tests` | テスト候補を含める |
| `--context-depth N` | フォールバック探索深度 |
| `--select-related F` | 関連ファイル候補を出力 |
| `--explain-selection` | 候補のスコア理由を出力 |
| `--resume-ai-session [NAME]` | 名前付きAIセッションを復元（省略時: `ai`） |
| `benchmark ai` | AIワークフローベンチマーク |
| `init claude` | Claude設定に`fileview` MCPを自動登録 |

## キーバインド（AI向け）

| キー | 動作 |
|---|---|
| `Ctrl+Shift+Y` | minimal context pack をコピー |
| `Ctrl+Shift+Enter` | review context pack をコピー |
| `Ctrl+Shift+P` | AI履歴ポップアップ |
| `Ctrl+A` | AIフォーカスモード切替 |
| `Ctrl+R` | 関連ファイル選択 |
| `Ctrl+G` | Git変更ファイル選択 |
| `Ctrl+T` | テストペア選択 |

## Claude設定の自動初期化

```bash
# ~/.claude.json に fileview のMCP設定を追加/更新
fv init claude

# パス指定
fv init claude --path ~/.claude.json
```

## MCP設定（手動）

```json
{
  "mcpServers": {
    "fileview": {
      "command": "fv",
      "args": ["--mcp-server", "/path/to/project"]
    }
  }
}
```

詳細仕様は英語版も参照してください: `docs/CLAUDE_CODE.md`
