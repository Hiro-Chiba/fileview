# カスタムコマンド

FileView ではカスタムシェルコマンドを定義し、キーにバインドできます。

## コマンドの定義

コマンドは `~/.config/fileview/config.toml` で定義します:

```toml
[commands]
open = "open $f"
edit = "nvim $f"
terminal = "cd $d && $SHELL"
compress = "zip -r archive.zip $S"
view = "less $f"
```

## プレースホルダー

| プレースホルダー | 説明 | 例 |
|-----------------|------|-----|
| `$f` | フルファイルパス | `/home/user/documents/file.txt` |
| `$d` | 親ディレクトリ | `/home/user/documents` |
| `$n` | 拡張子付きファイル名 | `file.txt` |
| `$s` | 拡張子なしファイル名 (stem) | `file` |
| `$e` | 拡張子のみ | `txt` |
| `$S` | 選択した全ファイル (クォート付き、スペース区切り) | `'/path/a.txt' '/path/b.txt'` |

## コマンドをキーにバインド

`~/.config/fileview/keymap.toml` で:

```toml
[browse]
"o" = "command:open"
"e" = "command:edit"
"T" = "command:terminal"
```

`command:` プレフィックスは FileView に名前付きコマンドを実行するよう指示します。

## 使用例

### デフォルトアプリケーションで開く

```toml
[commands]
open = "open $f"              # macOS
# open = "xdg-open $f"        # Linux
# open = "start $f"           # Windows
```

### エディタで編集

```toml
[commands]
edit = "nvim $f"
# edit = "code $f"            # VS Code
# edit = "vim $f"             # Vim
```

### ディレクトリでターミナルを開く

```toml
[commands]
terminal = "cd $d && $SHELL"
```

### アーカイブ操作

```toml
[commands]
compress = "zip -r archive.zip $S"
extract = "unzip $f -d $d"
```

### ファイル操作

```toml
[commands]
chmod_exec = "chmod +x $f"
chown = "sudo chown $USER $S"
```

### 表示とプレビュー

```toml
[commands]
view = "less $f"
preview = "bat --style=plain $f"
hex = "xxd $f | less"
```

### Git操作

```toml
[commands]
git_diff = "git diff $f"
git_log = "git log --oneline $f"
git_blame = "git blame $f | less"
```

### メディア操作

```toml
[commands]
play = "mpv $f"
convert = "ffmpeg -i $f ${s}_converted.mp4"
```

## カスタムプレビュー

特定の拡張子にカスタムプレビューコマンドを定義できます:

```toml
[preview.custom]
md = "glow -s dark $f"
json = "jq -C . $f"
csv = "column -s, -t $f | head -50"
yaml = "yq -C . $f"
```

これらは一致する拡張子のファイルをプレビューするときに自動的に使用されます。

## ヒント

- コマンドはシェル経由で実行されます (Unixでは `sh -c`、Windowsでは `cmd /C`)
- 複数の選択ファイルに対する操作には `$S` を使用
- 複雑なコマンドはシェルスクリプトとして書けます
- バインド前にターミナルでコマンドをテストしてください
