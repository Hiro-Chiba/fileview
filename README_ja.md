# FileView (fv)

[![Crates.io](https://img.shields.io/crates/v/fileview.svg)](https://crates.io/crates/fileview)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

モダンなターミナルエミュレータ向けの、ミニマルなVSCodeスタイルのファイルツリーTUI。

[English](README.md) | 日本語

## 特徴

- Vimライクなキーバインドによる高速なファイルツリーナビゲーション
- 複数選択によるバッチ操作
- テキスト・画像プレビュー（半ブロック描画）
- 内部クリップボードによるコピー/カット/ペースト
- システムクリップボード連携（パス/ファイル名コピー）
- 外部ツール連携用のPickモード
- ファイル選択時のコールバック実行
- 隠しファイルの表示切り替え
- マウス対応（クリック、ダブルクリック、スクロール）

## インストール

### crates.io から（推奨）

```bash
cargo install fileview
```

### ソースから

```bash
git clone https://github.com/Hiro-Chiba/fileview.git
cd fileview
cargo install --path .
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

### プレビュー

| キー | 動作 |
|------|------|
| `P` | プレビューパネル切り替え |
| `o` | フルスクリーンプレビュー |

### その他

| キー | 動作 |
|------|------|
| `.` | 隠しファイル表示切り替え |
| `R` / `F5` | リフレッシュ |
| `c` | パスをシステムクリップボードにコピー |
| `C` | ファイル名をシステムクリップボードにコピー |
| `?` | ヘルプ表示 |
| `q` | 終了 |

## CLIオプション

| オプション | 説明 |
|-----------|------|
| `-p`, `--pick` | Pickモード: 選択したパスを標準出力 |
| `-f`, `--format FMT` | 出力形式: `lines`（デフォルト）, `null`, `json` |
| `--on-select CMD` | ファイル選択時に実行するコマンド |
| `-h`, `--help` | ヘルプ表示 |
| `-V`, `--version` | バージョン表示 |

### `--on-select` のプレースホルダー

| プレースホルダー | 説明 |
|-----------------|------|
| `{path}` | フルパス |
| `{dir}` | 親ディレクトリ |
| `{name}` | 拡張子付きファイル名 |
| `{stem}` | 拡張子なしファイル名 |
| `{ext}` | 拡張子のみ |

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
