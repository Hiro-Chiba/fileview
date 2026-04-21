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
| `Ctrl+Y` | Claude向けフォーマットでコピー |
| `Ctrl+G` | Git変更ファイル選択 |
| `Ctrl+Shift+T` | テストペア選択 |

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

## ライブAIアクティビティ反映（v2.5.0+）

`fv --mcp-server` を別プロセスで動かしている AI エージェントからの
tool call を、対話型 `fv` がリアルタイムで UI に反映します。ステータスバーに
直近の操作を表示し、follow-mode を有効にすれば AI が読んだファイルに
自動でフォーカスが移動します。

### セットアップ

1. `fileview >= 2.5.0` であることを確認（`fv --version`）
2. Claude Code に `fv --mcp-server` を登録。`fv init claude` が簡単で、
   `~/.claude.json` に以下のようなエントリを追加します:

   ```json
   {
     "mcpServers": {
       "fileview": {
         "command": "fv",
         "args": ["--mcp-server", "/absolute/path/to/your/project"]
       }
     }
   }
   ```

3. 別ターミナル（同じマシン、同じユーザー）で、Claude Code に登録した
   プロジェクトルートと同じ場所で対話型 fv を起動:

   ```bash
   cd /absolute/path/to/your/project
   fv --follow-ai .
   ```

   `--follow-ai` を付けない場合、ステータスバーには表示されますが
   自動フォーカスは行われません。

4. Claude Code が MCP 経由で `read_file src/auth.rs` などを呼ぶと、
   対話型 fileview に以下のように表示されます:

   ```
   [AI*] claude: read_file src/auth.rs
   ```

   follow-mode 有効時は tree がそのパスに自動でジャンプします。

### キーバインド

| キー | 動作 |
|---|---|
| `Alt+A` | follow-mode 切り替え（AI の直近ファイルに自動フォーカス） |
| `Alt+L` | ライブ活動ログ popup を開く（`j`/`k` 移動、`Enter` でジャンプ、`Esc`/`q` で閉じる） |

検索・フィルタ・リネーム・bulk rename・fuzzy finder・確認ダイアログ等の
入力モード中は follow-mode が自動的に抑止されるので、
意図したキー入力が勝手に消えたりフォーカスが奪われたりはしません。

### 2プロセスの連携方法

MCP サーバーと対話型 TUI は別 OS プロセスとして動作し、ユーザーのキャッシュ
ディレクトリ経由で連携します:

- 対話型プロセスは `~/.cache/fileview/sessions/<pid>/` を作り、
  `session.json`（pid + ルート + 起動時刻）と追記専用の
  `activity.jsonl`（unix では権限 `0600`）を置きます
- `fv --mcp-server` は tool call のたびに、対象パスを祖先に持つ生存中の
  全セッションの `activity.jsonl` に JSONL 1行を追記します
- 対話型プロセスは `notify` crate でそのファイルを監視し、フレームごとに
  新しいイベントを取り込みます

### トラブルシューティング

- **ステータスバーに何も出ない**: 対話型 `fv` のルートが、`fv --mcp-server`
  に渡したディレクトリと同じか、その祖先になっているか確認してください。
  無関係なプロジェクト間で混線しないようパスでスコープを絞っています
- **`--follow-ai` 起動時に "activity registry unavailable" が出た**:
  キャッシュディレクトリを作成できない状況（読み取り専用ホームなど）で発生します。
  イベント配信自体は試みますが、follow-mode は無効化されます
- **古いセッションディレクトリが残っている**: MCP サーバーは次の emit 時に
  PID が生存していないセッションを掃除します。気になる場合は
  `~/.cache/fileview/sessions/*` を手動で削除しても問題ありません。
  対話型 fileview は起動時に自分の分を作り直します
