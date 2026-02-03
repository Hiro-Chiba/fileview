# FileView v4 以降ロードマップ

## 現状分析（v1.22.0 完了後）

### v3 (Phase 7-11) 完了機能

v1.18.0 で以下の機能を実装完了:

| Phase | 機能 | バージョン |
|-------|------|-----------|
| Phase 7 | 設定ファイル、キーマップカスタマイズ、テーマシステム | v1.16.0 |
| Phase 8 | カスタムコマンド、カスタムプレビュー | v1.17.0 |
| Phase 9 | 動画プレビュー（ffmpeg連携） | v1.18.0 |
| Phase 10 | イベントフック、シェル連携強化 | v1.18.0 |
| Phase 11 | タブサポート、バッチ操作（Visual Select） | v1.18.0 |

### v4 (Phase 12) 完了機能

v1.22.0 で以下の機能を実装完了:

| Phase | 機能 | バージョン |
|-------|------|-----------|
| Phase 12a | Luaランタイム統合、読み取りAPI | v1.19.0 |
| Phase 12b | アクションAPI | v1.20.0 |
| Phase 12c | イベントシステム、カスタムプレビューア | v1.21.0 |
| Phase 12d | プラグインローダー統合 | v1.22.0 |

### スコア比較（v1.22.0）

| ツール | 総合スコア | 備考 |
|--------|-----------|------|
| **fileview** | **99点** | v1.22.0 |
| yazi | 104点 | 非同期処理、豊富なプラグインエコシステム |
| lf | 78点 | シェルスクリプトベース |
| ranger | 75点 | レガシー（Python） |
| nnn | 65点 | 最軽量だが機能限定 |

### 詳細スコア内訳

| カテゴリ | fileview (v1.22.0) | yazi | 差分 |
|----------|-------------------|------|------|
| 基本機能 | 15/15 | 15/15 | - |
| ナビゲーション | 13/15 | 15/15 | -2 |
| ファイル操作 | 14/15 | 15/15 | -1 |
| プレビュー | 14/15 | 15/15 | -1 |
| Git連携 | 10/10 | 10/10 | - |
| 設定/カスタマイズ | 13/15 | 15/15 | -2 |
| プラグイン/拡張性 | 10/10 | 10/10 | - |
| シェル連携 | 10/10 | 10/10 | - |
| **合計** | **99/105** | **104/105** | **-5** |

**yaziとの差**: ナビゲーション(-2)、ファイル操作(-1)、プレビュー(-1)、設定/カスタマイズ(-2)
- yaziは非同期処理による大規模ディレクトリ対応が優れている
- プラグインエコシステムの成熟度でも差がある

---

## v4 完了機能

### Phase 12: Lua プラグインシステム ✅ 完了

**概要**: Lua スクリプトによる拡張機能

| 項目 | 内容 |
|------|------|
| 状態 | **完了** (v1.19.0-v1.22.0) |
| 複雑度 | Very High |
| 効果 | プラグイン/拡張性 +3 |
| 技術選定 | [mlua](https://github.com/khvzak/mlua) crate |
| 実装行数 | 約2,500行 |

**実装機能**:

1. **カスタムコマンド**（Luaで定義）
   ```lua
   -- ~/.config/fileview/plugins/init.lua
   fv.register_command("copy-path", function()
     local path = fv.current_file()
     if path then
       fv.set_clipboard(path)
       fv.notify("Copied: " .. path)
     end
   end)
   ```

2. **プレビュー拡張**（カスタムプレビューア）
   ```lua
   -- カスタムプレビューアの登録
   fv.register_previewer("*.csv", function(path)
     local file = io.open(path, "r")
     if not file then return "Cannot read file" end
     local content = file:read("*a")
     file:close()
     return content
   end)
   ```

3. **イベントフック拡張**
   ```lua
   -- イベントへの反応
   fv.on("file_selected", function(path)
     if path and path:match("%.env$") then
       fv.notify("Warning: Environment file!")
     end
   end)
   ```

**実装済みAPI**: [docs/PLUGINS.md](PLUGINS.md) を参照

---

## 見送り機能

### Phase 13: WASM プラグイン ⏸️ 見送り

**見送り理由**: Luaプラグインシステムで十分な拡張性を実現。WASMは開発の敷居が高く、ユーザーメリットが限定的。

### Phase 13: WASM プラグイン（参考）

**概要**: WebAssembly による言語非依存のプラグインシステム

| 項目 | 内容 |
|------|------|
| 複雑度 | High |
| 効果 | プラグイン/拡張性 +2 |
| 技術選定 | [wasmtime](https://github.com/bytecodealliance/wasmtime) crate |
| 推定行数 | 1,500-2,000行 |

**利点**:

1. **言語非依存**: Rust, Go, C, AssemblyScript など任意の言語でプラグイン開発可能
2. **サンドボックス化**: WASM のセキュリティモデルにより安全に実行
3. **パフォーマンス**: ネイティブに近い実行速度

**実装構成**:

```
~/.config/fileview/plugins/
├── my_plugin.wasm        # コンパイル済みプラグイン
└── my_plugin.toml        # プラグイン設定
```

**プラグイン設定例**:
```toml
# my_plugin.toml
[plugin]
name = "my-plugin"
version = "1.0.0"
wasm = "my_plugin.wasm"

[permissions]
filesystem = ["read"]
network = false
```

**リスク**:
- 開発の敷居が高い（WASM のビルド環境が必要）
- Lua より普及していない
- ファイルサイズの増加

### Phase 14: フル Async 化（tokio 移行） ⏸️ 見送り

**見送り理由**: 大規模リファクタリングによるリスクが高い。現状の同期処理でも実用上十分なパフォーマンスを実現しており、メリットに対してリスクが大きすぎる。

**参考情報**:

| 項目 | 内容 |
|------|------|
| 複雑度 | Very High |
| 効果 | 性能向上、大規模ディレクトリ対応 |
| 技術選定 | [tokio](https://tokio.rs/) |
| 推定行数 | 既存コード全体の大規模リファクタリング |

**リスク**:
- 大規模リファクタリングによる不安定化
- 起動時間への影響（tokio runtime初期化）
- 既存のテストの全面書き換え
- シンプルさの設計原則との矛盾

---

## v4 成果サマリー

### 達成事項

| 項目 | 結果 |
|------|------|
| Phase 12 Luaプラグイン | ✅ 完了 (v1.19.0-v1.22.0) |
| プラグイン/拡張性スコア | 7点 → 10点 (+3) |
| 総合スコア | 96点 → 99点 (+3) |

### 競合との比較（v1.22.0）

| ツール | スコア | 備考 |
|--------|--------|------|
| yazi | 104点 | 非同期処理、プラグインエコシステム |
| **fileview** | **99点** | 軽量、設定不要、Luaプラグイン対応 |
| lf | 78点 | シェルスクリプトベース |
| ranger | 75点 | レガシー（Python） |
| nnn | 65点 | 最軽量だが機能限定 |

**結論**: yaziには及ばないが、「軽量・設定不要・バッテリー同梱」の設計思想を維持しながら、十分な拡張性を実現

---

## 設計原則との整合性

### 現在の設計原則

> 1. **シンプルに保つ** - 機能追加より安定性
> 2. **高速起動** - ターミナルツールは軽くあるべき
> 3. **ゼロ設定** - インストールしてすぐ使える
> 4. **画像プレビュー** - これが差別化ポイント

### v4 で変更した原則

| 原則 | v4での変更 |
|------|-----------|
| プラグインシステム | 「やらないこと」→「やること」に移動 |
| ゼロ設定 | 「プラグインなしでもフル機能」を維持 ✅ |

**重要**: プラグインは**オプション**であり、プラグインなしでも fileview は完全に動作する。「ゼロ設定」の原則は維持されている。

---

## 次のステップ

### 今後の方向性

Phase 12 完了により、fileview は十分な機能と拡張性を備えた。今後は:

1. **安定性の維持**: バグ修正、パフォーマンス改善
2. **プラグインエコシステムの育成**: サンプルプラグイン、ドキュメント充実
3. **ユーザーフィードバック対応**: 要望に応じた機能追加

### 検討中（優先度低）

- プラグインのパッケージ管理（npm/cargo 的なもの）
- プラグインのバージョニング
- コミュニティプラグインリポジトリ

---

## 参考リンク

- [mlua - High level Lua bindings for Rust](https://github.com/khvzak/mlua)
- [wasmtime - A fast and secure runtime for WebAssembly](https://github.com/bytecodealliance/wasmtime)
- [Yazi Lua Plugin Docs](https://yazi-rs.github.io/docs/plugins/overview)
- [tokio - A runtime for writing reliable asynchronous applications](https://tokio.rs/)
