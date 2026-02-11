# テーマのカスタマイズ

FileView は `~/.config/fileview/theme.toml` でカスタムカラーテーマをサポートしています。

## テーマファイルの構造

```toml
[colors]
background = "default"    # メイン背景
foreground = "white"      # デフォルトテキスト色
selection = "blue"        # 選択ハイライト
border = "gray"           # ボーダー色
help_key_fg = "black"     # ヘルプポップアップのキーテキスト色
help_key_bg = "cyan"      # ヘルプポップアップのキー背景色

[file_colors]
directory = "blue"        # ディレクトリ色
executable = "green"      # 実行可能ファイル色
symlink = "cyan"          # シンボリックリンク色
archive = "red"           # アーカイブファイル色 (.zip, .tar等)
image = "magenta"         # 画像ファイル色
media = "magenta"         # メディアファイル色

[git_colors]
modified = "yellow"       # 変更されたファイル
staged = "green"          # ステージされたファイル
untracked = "red"         # 未追跡ファイル
conflict = "red"          # コンフリクトファイル
```

## カラーフォーマット

FileView は複数のカラーフォーマットをサポートしています:

### 名前付きカラー

```toml
foreground = "white"
directory = "blue"
modified = "yellow"
```

利用可能な名前付きカラー:
- `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`
- `gray`, `grey` (ダークグレー)
- `lightred`, `lightgreen`, `lightyellow`, `lightblue`, `lightmagenta`, `lightcyan`
- `darkgray`, `darkgrey`
- `default` (ターミナルのデフォルト)
- `reset` (デフォルトにリセット)

### 16進カラー

```toml
# 短縮形 (#RGB)
background = "#123"

# 長形式 (#RRGGBB)
foreground = "#ff5733"
```

### RGB関数

```toml
selection = "rgb(255, 87, 51)"
```

### 256色パレット

```toml
# color<N> または colorN 構文を使用
border = "color240"
accent = "color33"

# または番号だけ
dim = "245"
```

## テーマ例

### ダークテーマ (デフォルト)

```toml
[colors]
background = "default"
foreground = "white"
selection = "blue"
border = "gray"

[file_colors]
directory = "blue"
executable = "green"
symlink = "cyan"

[git_colors]
modified = "yellow"
staged = "green"
untracked = "red"
```

### ライトテーマ

```toml
[colors]
background = "#ffffff"
foreground = "#1a1a1a"
selection = "#0066cc"
border = "#cccccc"

[file_colors]
directory = "#0066cc"
executable = "#008800"
symlink = "#008888"

[git_colors]
modified = "#cc6600"
staged = "#008800"
untracked = "#cc0000"
```

### Nord テーマ

```toml
[colors]
background = "#2e3440"
foreground = "#eceff4"
selection = "#5e81ac"
border = "#4c566a"

[file_colors]
directory = "#81a1c1"
executable = "#a3be8c"
symlink = "#88c0d0"

[git_colors]
modified = "#ebcb8b"
staged = "#a3be8c"
untracked = "#bf616a"
```

## ヒント

- ターミナル背景を継承するには背景に `default` を使用
- 正確な制御には256色パレット (`color0`-`color255`) を使用
- 16進カラーが最も柔軟性が高い
- 一貫性のため異なるターミナルエミュレーターでテーマをテスト
- ヘルプキーのハイライトスタイルは `FILEVIEW_HELP_KEY_STYLE=solid|outline|plain` で切り替え可能
