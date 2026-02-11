# 設定ガイド

FileView は `~/.config/fileview/` にあるTOMLファイルで設定できます。

## 設定ファイル

| ファイル | 説明 |
|---------|------|
| `config.toml` | メイン設定 (一般、プレビュー、UI、パフォーマンス) |
| `keymap.toml` | カスタムキーバインド |
| `theme.toml` | カラーテーマのカスタマイズ |

## メイン設定 (`config.toml`)

### 一般設定

```toml
[general]
show_hidden = false       # デフォルトで隠しファイルを表示
enable_icons = true       # Nerd Font アイコンを有効化
mouse_enabled = true      # マウスサポートを有効化
```

### プレビュー設定

```toml
[preview]
hex_max_bytes = 4096         # hexプレビューの最大バイト数
max_archive_entries = 500    # アーカイブプレビューの最大エントリ数
image_protocol = "auto"      # 画像プロトコル: auto, sixel, kitty, iterm2, halfblocks, chafa

# カスタムプレビューコマンド (拡張子 -> コマンド)
[preview.custom]
md = "glow -s dark $f"       # Markdown を glow で
json = "jq -C . $f"          # JSON を jq で
csv = "column -s, -t $f | head -50"
```

### パフォーマンス設定

```toml
[performance]
git_poll_interval_secs = 5   # Git状態のポーリング間隔
```

### UI設定

```toml
[ui]
show_size = true                    # ツリービューにファイルサイズを表示
show_permissions = false            # ファイルパーミッションを表示
date_format = "%Y-%m-%d %H:%M"      # 日付フォーマット (strftime形式)
```

### カスタムコマンド

```toml
[commands]
# キーにバインドできるコマンドを定義
# プレースホルダー: $f (フルパス), $d (ディレクトリ), $n (ファイル名), $s (stem), $e (拡張子), $S (選択ファイル)
open = "open $f"
edit = "nvim $f"
terminal = "cd $d && $SHELL"
compress = "zip -r archive.zip $S"
```

## 環境変数

| 変数 | 説明 |
|------|------|
| `FILEVIEW_ICONS=0` | アイコンを無効化 |
| `FILEVIEW_IMAGE_PROTOCOL` | 画像プロトコルを強制 |
| `FILEVIEW_HELP_KEY_STYLE` | ヘルプキースタイル: `solid`, `outline`, `plain` |

## CLI引数

CLI引数は設定ファイルの設定を上書きします:

```
--hidden, -a     隠しファイルを表示
--no-hidden      隠しファイルを非表示
--icons, -i      アイコンを有効化
--no-icons       アイコンを無効化
```

## 設定の優先順位

1. CLI引数 (最高優先度)
2. 環境変数
3. 設定ファイル (`~/.config/fileview/config.toml`)
4. デフォルト値 (最低優先度)

## 設定例

完全な例は [examples/config.toml](../examples/config.toml) を参照してください。
