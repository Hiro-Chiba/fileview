# FileView ベンチマーク

他のターミナルファイルマネージャーとの性能比較。

## 概要比較

| ツール | 起動時間 | メモリ (アイドル) | バイナリサイズ | 言語 |
|--------|----------|------------------|---------------|------|
| **fileview** | **2.2ms** | **~8MB** | **5.9MB** | Rust |
| nnn | 1.5ms | 3.4MB | 0.1MB | C |
| lf | 3ms | 12MB | 3.5MB | Go |
| ranger | 400ms | 28MB | - | Python |
| yazi | 15ms | 38MB | 4.5MB | Rust |

*出典: [joshuto discussion](https://github.com/kamiyaa/joshuto/discussions/454)、独自計測*

## FileView ベンチマーク (v2.4.0)

### 起動時間

```
$ hyperfine --warmup 3 './target/release/fv --help'

Benchmark: fv --help
  Time (mean ± σ):       2.2 ms ±   0.2 ms    [625 runs]
  Range (min … max):     1.8 ms …   3.1 ms
```

### バイナリサイズ

```
$ ls -lh target/release/fv
5.9M target/release/fv
```

ビルド設定:
- Profile: release
- LTO: true (full)
- Codegen units: 1

### メモリ使用量

アイドルメモリ消費 (`/usr/bin/time -l` で計測):
- アイドル: ~7.5MB (7,815,168 bytes)
- 1000ファイル: ~9MB
- 画像プレビュー時: ~16MB

## FileView が速い理由

1. **遅延読み込み** - 可視エントリのみ読み込む
2. **遅延Git検出** - 初回描画後にGit状態をチェック
3. **効率的なツリー構造** - ディレクトリごとに単一アロケーション
4. **インタプリタなし** - ネイティブRustバイナリ

## テスト環境

- OS: macOS (Darwin 25.3.0)
- アーキテクチャ: ARM64 (Apple Silicon)
- Rust: 1.93.0
- 日付: 2026-03-21

## トレードオフ

FileView は**起動速度**と**低メモリ**を機能より優先:

| 機能 | fileview | yazi |
|------|----------|------|
| プラグインシステム | Lua | あり (Lua) |
| 非同期I/O | 部分的 | 完全 |
| 組み込みシンタックスハイライト | あり | あり |
| 設定ファイル | なし | あり |

これは意図的な設計判断です - 詳細は [DESIGN_ja.md](DESIGN_ja.md) を参照。
