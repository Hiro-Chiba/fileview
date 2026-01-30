# FileView (fv)

[![Crates.io](https://img.shields.io/crates/v/fileview.svg)](https://crates.io/crates/fileview)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Vimライクなキーバインドと画像プレビューを備えた、ミニマルなファイルツリーブラウザ。

[English](README.md) | 日本語

## 特徴

- Vimライクなキーバインドによる高速なファイルツリーナビゲーション
- **Git連携** - ファイル状態のカラー表示とブランチ名表示
- 複数選択によるバッチ操作
- **プレビューパネル**（以下に対応）:
  - テキストファイル（行番号付き）
  - 画像（Kitty/iTerm2/Sixelプロトコル、自動検出対応）
  - ディレクトリ（ファイル数、サイズ統計）
  - バイナリファイル（Hexダンプ表示）
- **ファジーファインダー** (`Ctrl+P`) で素早くファイル検索
- 内部クリップボードによるコピー/カット/ペースト
- システムクリップボード連携（パス/ファイル名コピー）
- 外部ツール連携用のPickモード
- ファイル選択時のコールバック実行
- 隠しファイルの表示切り替え
- マウス対応（クリック、ダブルクリック、スクロール、ドラッグ&ドロップ）
- **Nerd Fontsアイコン**（デフォルト有効、`--no-icons`で無効化）

## Gitステータス表示

Gitリポジトリ内では、ファイルの状態に応じて色分け表示されます：

| 色 | 状態 |
|----|------|
| 黄色 | 変更あり (Modified) |
| 緑色 | 追加/未追跡 (Added/Untracked) |
| 赤色 | 削除 (Deleted) |
| シアン | リネーム (Renamed) |
| グレー | 無視 (Ignored) |
| マゼンタ | コンフリクト (Conflict) |

ステータスバーに現在のブランチ名が表示されます。

## 画像プレビュー

FileViewはターミナルを自動検出し、最適な画像プロトコルを選択します：

| ターミナル | プロトコル | 品質 |
|-----------|----------|------|
| Kitty | Kitty Graphics | 最高 |
| Ghostty | Kitty Graphics | 最高 |
| Konsole | Kitty Graphics | 最高 |
| iTerm2 | iTerm2 Inline | 最高 |
| WezTerm | iTerm2 Inline | 最高 |
| Warp | iTerm2 Inline | 最高 |
| Foot | Sixel | 良好 |
| Windows Terminal | Sixel | 良好 |
| VS Code | Halfblocks | 基本 |
| Alacritty | Halfblocks | 基本 |
| その他 | 自動検出 | 可変 |

`FILEVIEW_IMAGE_PROTOCOL` 環境変数でオーバーライド可能です（下記参照）。

## インストール

### crates.io から（推奨）

```bash
cargo install fileview
```

### Chafaサポート付き（オプション）

ネイティブ画像プロトコル非対応のターミナルで高品質な画像プレビューを利用する場合：

```bash
# 先にlibchafaをインストール
# macOS:
brew install chafa

# Ubuntu/Debian:
sudo apt install libchafa-dev

# その後、chafa featureを有効にしてインストール
cargo install fileview --features chafa
```

### ソースから

```bash
git clone https://github.com/Hiro-Chiba/fileview.git
cd fileview
cargo install --path .

# Chafaサポート付き:
cargo install --path . --features chafa
```

### 動作要件

- Rust 1.70以上
- True Color対応ターミナル（推奨: Ghostty, iTerm2, Alacritty）

## 使い方

```bash
# カレントディレクトリを開く
fv

# 指定ディレクトリを開く
fv /path/to/directory

# Pickモード（選択したパスを標準出力）
fv --pick

# PickモードでJSON出力
fv --pick --format json

# 選択時にコマンド実行
fv --on-select "code {path}"
```

## キーバインド

### ナビゲーション

| キー | 動作 |
|------|------|
| `j` / `↓` | 下に移動 |
| `k` / `↑` | 上に移動 |
| `g` | 先頭へ |
| `G` | 末尾へ |

### ツリー操作

| キー | 動作 |
|------|------|
| `l` / `→` / `Tab` | ディレクトリを展開 |
| `h` / `←` / `Backspace` | ディレクトリを折りたたむ |
| `Enter` | 展開/折りたたみ切り替え |
| `H` | すべて折りたたむ |
| `L` | すべて展開（深さ制限: 5） |

### 選択

| キー | 動作 |
|------|------|
| `Space` | マーク切り替え |
| `Esc` | マークをすべて解除 |

### ファイル操作

| キー | 動作 |
|------|------|
| `a` | 新規ファイル作成 |
| `A` | 新規ディレクトリ作成 |
| `r` | リネーム |
| `D` / `Delete` | 削除（確認あり） |
| `y` | クリップボードにコピー |
| `d` | クリップボードにカット |
| `p` | ペースト |

### 検索

| キー | 動作 |
|------|------|
| `/` | 検索開始 |
| `n` | 次の検索結果 |
| `Ctrl+P` | ファジーファインダーを開く |

### ファジーファインダー

`Ctrl+P` でビルトインのファジーファインダーを開き、素早くファイルを検索:

| キー | 動作 |
|------|------|
| `↑` / `Ctrl+K` | 上の結果へ移動 |
| `↓` / `Ctrl+J` | 下の結果へ移動 |
| `Enter` | 選択したファイルにジャンプ |
| `Esc` | キャンセル |

- 入力するとファイル名でフィルタリング
- 結果はマッチスコア順にソート
- 隠しファイルは現在の表示設定に従う

### プレビュー

| キー | 動作 |
|------|------|
| `P` | サイドプレビューパネル切り替え |
| `o` | フルスクリーンプレビュー |
| `Tab` | ツリー/プレビュー間のフォーカス切り替え（プレビュー表示時） |

#### サイドプレビューのフォーカスモード

サイドプレビュー表示中に `Tab` でフォーカスを切り替え:

| フォーカス | j/k/↑/↓ | g/G | b/f |
|------------|---------|-----|-----|
| ツリー | ファイル移動 | 先頭/末尾 | - |
| プレビュー | スクロール | プレビュー先頭/末尾 | ページスクロール |

- パネルをクリックでフォーカス切り替え
- スクロールはフォーカス位置に応じて動作
- `Esc` でツリーにフォーカスを戻す
- フォーカス中のパネルはシアン色のボーダーでハイライト

### その他

| キー | 動作 |
|------|------|
| `.` | 隠しファイル表示切り替え |
| `R` / `F5` | リフレッシュ |
| `c` | パスをシステムクリップボードにコピー |
| `C` | ファイル名をシステムクリップボードにコピー |
| `?` | ヘルプ表示 |
| `q` | 終了 |
| `Q` | 終了して現在のディレクトリにcd（`--choosedir`使用時） |

## CLIオプション

| オプション | 説明 |
|-----------|------|
| `-p`, `--pick` | Pickモード: 選択したパスを標準出力 |
| `-f`, `--format FMT` | 出力形式: `lines`（デフォルト）, `null`, `json` |
| `--on-select CMD` | ファイル選択時に実行するコマンド |
| `--choosedir` | 終了時にディレクトリパスを出力（シェルcd連携用） |
| `-i`, `--icons` | Nerd Fontsアイコンを有効化（デフォルト） |
| `--no-icons` | アイコンを無効化 |
| `-h`, `--help` | ヘルプ表示 |
| `-V`, `--version` | バージョン表示 |

### 終了コード

| コード | 意味 |
|--------|------|
| 0 | 成功（正常終了またはPickモードでファイル選択） |
| 1 | キャンセル（Pickモードでユーザーがキャンセル） |
| 2 | エラー（実行時エラー） |
| 3 | 不正な引数（不明なオプションまたは無効な値） |

### 環境変数

| 変数 | 説明 |
|------|------|
| `FILEVIEW_ICONS=0` | アイコンを無効化 |
| `FILEVIEW_IMAGE_PROTOCOL` | 画像プロトコルを指定: `auto`, `halfblocks`, `chafa`, `sixel`, `kitty`, `iterm2` |

### `--on-select` のプレースホルダー

| プレースホルダー | 説明 |
|-----------------|------|
| `{path}` | フルパス |
| `{dir}` | 親ディレクトリ |
| `{name}` | 拡張子付きファイル名 |
| `{stem}` | 拡張子なしファイル名 |
| `{ext}` | 拡張子のみ |

### シェル連携

fileviewでディレクトリを移動し、選択した場所にcdする:

```bash
# .bashrc または .zshrc に追加
fvcd() {
  local dir
  dir=$(fv --choosedir "$@")
  if [ -n "$dir" ] && [ -d "$dir" ]; then
    cd "$dir"
  fi
}
```

使い方:
- `fvcd` でfileviewを起動
- 目的のディレクトリに移動
- `Q` で終了してそこにcd
- `q` でディレクトリ移動なしで終了

### 使用例

```bash
# ファイルピッカーとして使用
selected=$(fv --pick)
echo "選択: $selected"

# 選択したファイルをエディタで開く
fv --on-select "vim {path}"

# 選択したファイルパスをクリップボードにコピー（macOS）
fv --on-select "echo {path} | pbcopy"

# 複数ファイル選択でJSON出力
fv --pick --format json
```

## ライセンス

MIT
