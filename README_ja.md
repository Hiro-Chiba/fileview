# FileView (fv)

[![Crates.io](https://img.shields.io/crates/v/fileview.svg)](https://crates.io/crates/fileview)
[![Downloads](https://img.shields.io/crates/d/fileview.svg)](https://crates.io/crates/fileview)
[![CI](https://github.com/Hiro-Chiba/fileview/actions/workflows/ci.yml/badge.svg)](https://github.com/Hiro-Chiba/fileview/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![MSRV](https://img.shields.io/badge/MSRV-1.70-blue.svg)](https://www.rust-lang.org)

> ミニマルで高速なターミナルファイルブラウザ（Vimキーバインド対応）

[English](README.md) | 日本語

## なぜ fv？

- **即起動** - 設定不要、`cargo install fileview` だけ
- **画像プレビュー** - Kitty/iTerm2/Sixel 自動検出対応
- **Git連携** - ファイル状態を色で一目確認
- **Vimキーバインド** - j/k/h/l で高速操作
- **ファジーファインダー** - `Ctrl+P` で素早く検索

## クイックスタート

```bash
cargo install fileview
fv
```

## 機能

| 機能 | 説明 |
|------|------|
| ツリーナビゲーション | Vimキーで展開/折りたたみ |
| 複数選択 | バッチ操作対応 |
| プレビューパネル | テキスト、画像、Hexダンプ |
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
| `q` | 終了 |

**[完全なキーバインド一覧](docs/KEYBINDINGS_ja.md)**

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

## ライセンス

MIT
