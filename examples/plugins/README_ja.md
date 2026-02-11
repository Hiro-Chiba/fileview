# FileView プラグイン例

FileView 用の Lua プラグイン例。`~/.config/fileview/plugins/` にコピーして使用してください。

## 利用可能なプラグイン

### claude_code.lua
Claude Code と AI アシスタントとの連携。
- `claude-ref`: Claude 参照用にファイルパスをコピー
- `claude-context`: マークしたファイルをコンテキストリストとしてコピー

### git_actions.lua
FileView からの簡単な git 操作。
- `git-add`: 現在のファイルをステージ
- `git-reset`: 現在のファイルをアンステージ
- `git-diff`: 現在のファイルの差分を表示
- `git-log`: 現在のファイルのログを表示
- `git-add-marked`: マークした全ファイルをステージ

### productivity.lua
一般的な生産性コマンド。
- `edit`: $EDITOR で開く
- `code`: VS Code で開く
- `yank-path`: フルパスをコピー
- `yank-name`: ファイル名のみをコピー
- `info`: ファイル情報を表示

## インストール

```bash
mkdir -p ~/.config/fileview/plugins
cp examples/plugins/*.lua ~/.config/fileview/plugins/
```

## 使い方

FileView で `:command-name` でコマンドを実行します。

例:
```
:git-add       # 現在のファイルをステージ
:claude-ref    # Claude 用にパスをコピー
```

## 自分で作成する

完全な API リファレンスは [docs/PLUGINS_ja.md](../../docs/PLUGINS_ja.md) を参照してください。
