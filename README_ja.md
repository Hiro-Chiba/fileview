# FileView (fv)

[![Crates.io](https://img.shields.io/crates/v/fileview.svg)](https://crates.io/crates/fileview)
[![Downloads](https://img.shields.io/crates/d/fileview.svg)](https://crates.io/crates/fileview)
[![CI](https://github.com/Hiro-Chiba/fileview/actions/workflows/ci.yml/badge.svg)](https://github.com/Hiro-Chiba/fileview/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![MSRV](https://img.shields.io/badge/MSRV-1.75-blue.svg)](https://www.rust-lang.org)

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

- [キーバインド](docs/KEYBINDINGS_ja.md) - 完全なキーバインド一覧
- [プラグイン](docs/PLUGINS_ja.md) - Lua プラグインシステム
- [競合比較](docs/COMPARISON.md) - yazi, lf, ranger, nnn との比較
- [ベンチマーク](docs/BENCHMARKS.md) - パフォーマンスデータ
- [セキュリティ](docs/SECURITY.md) - セキュリティモデル

## ライセンス

MIT
