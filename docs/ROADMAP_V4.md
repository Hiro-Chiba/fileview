# FileView v4 以降ロードマップ

## 現状分析（v1.18.0 完了後）

### v3 (Phase 7-11) 完了機能

v1.18.0 で以下の機能を実装完了:

| Phase | 機能 | バージョン |
|-------|------|-----------|
| Phase 7 | 設定ファイル、キーマップカスタマイズ、テーマシステム | v1.16.0 |
| Phase 8 | カスタムコマンド、カスタムプレビュー | v1.17.0 |
| Phase 9 | 動画プレビュー（ffmpeg連携） | v1.18.0 |
| Phase 10 | イベントフック、シェル連携強化 | v1.18.0 |
| Phase 11 | タブサポート、バッチ操作（Visual Select） | v1.18.0 |

### スコア比較（v1.18.0）

| ツール | 総合スコア | 備考 |
|--------|-----------|------|
| **fileview** | **96点** | v1.18.0 |
| yazi | 98点 | プラグインシステムが強み |
| lf | 78点 | シェルスクリプトベース |
| ranger | 75点 | レガシー（Python） |
| nnn | 65点 | 最軽量だが機能限定 |

### 詳細スコア内訳

| カテゴリ | fileview (v1.18.0) | yazi | 差分 |
|----------|-------------------|------|------|
| 基本機能 | 15/15 | 15/15 | - |
| ナビゲーション | 13/15 | 14/15 | -1 |
| ファイル操作 | 14/15 | 15/15 | -1 |
| プレビュー | 14/15 | 15/15 | -1 |
| Git連携 | 10/10 | 10/10 | - |
| 設定/カスタマイズ | 13/15 | 15/15 | -2 |
| プラグイン/拡張性 | 7/10 | 10/10 | **-3** |
| シェル連携 | 10/10 | 10/10 | - |
| **合計** | **96/105** | **98/105** | **-2** |

**yaziとの差**: プラグイン/拡張性（-3点）

---

## v4 候補機能

### Phase 12: Lua プラグインシステム

**概要**: Lua スクリプトによる拡張機能

| 項目 | 内容 |
|------|------|
| 複雑度 | Very High |
| 効果 | プラグイン/拡張性 +3 |
| 技術選定 | [mlua](https://github.com/khvzak/mlua) crate |
| 推定行数 | 2,000-3,000行 |

**実装機能**:

1. **カスタムコマンド**（Luaで定義）
   ```lua
   -- ~/.config/fileview/plugins/my_command.lua
   function my_copy_path()
     local path = fv.current_file()
     fv.set_clipboard(path)
     fv.notify("Copied: " .. path)
   end

   fv.register_command("copy-path", my_copy_path)
   ```

2. **プレビュー拡張**（カスタムプレビューア）
   ```lua
   -- カスタムプレビューアの登録
   fv.register_previewer("*.custom", function(path)
     local content = fv.read_file(path)
     return fv.highlight(content, "custom")
   end)
   ```

3. **イベントフック拡張**
   ```lua
   -- イベントへの反応
   fv.on("file_selected", function(path)
     if path:match("%.secret$") then
       fv.notify("Warning: Secret file!")
     end
   end)
   ```

**API設計**:

```lua
-- fileview Lua API
fv = {
  -- ファイル操作
  current_file = function() end,     -- 現在のファイルパス
  current_dir = function() end,      -- 現在のディレクトリ
  selected_files = function() end,   -- 選択されたファイル一覧

  -- アクション
  navigate = function(path) end,     -- ディレクトリ移動
  select = function(path) end,       -- ファイル選択
  preview = function(content) end,   -- プレビュー表示
  refresh = function() end,          -- 画面更新

  -- ユーティリティ
  notify = function(msg) end,        -- 通知表示
  confirm = function(msg) end,       -- 確認ダイアログ
  input = function(prompt) end,      -- 入力ダイアログ
  set_clipboard = function(s) end,   -- クリップボード設定

  -- ファイルシステム
  read_file = function(path) end,    -- ファイル読み込み
  file_exists = function(path) end,  -- 存在確認
  is_dir = function(path) end,       -- ディレクトリ判定

  -- 登録
  register_command = function(name, fn) end,
  register_previewer = function(pattern, fn) end,
  register_keymap = function(key, fn) end,
  on = function(event, fn) end,
}
```

**リスク**:
- 学習コスト（ユーザーがLuaを学ぶ必要）
- セキュリティ（サンドボックス化が必要）
- 「ゼロ設定」の設計原則との矛盾

---

### Phase 13: WASM プラグイン

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

---

### Phase 14: フル Async 化（tokio 移行）

**概要**: 非同期ランタイムへの全面移行

| 項目 | 内容 |
|------|------|
| 複雑度 | Very High |
| 効果 | 性能向上、大規模ディレクトリ対応 |
| 技術選定 | [tokio](https://tokio.rs/) |
| 推定行数 | 既存コード全体の大規模リファクタリング |

**期待される改善**:

1. **大規模ディレクトリのスムーズなスクロール**
2. **複数プレビューの並列読み込み**
3. **Git操作のノンブロッキング化**
4. **ファイル監視の効率化**

**影響範囲**:

| モジュール | 変更内容 |
|------------|----------|
| `tree/` | `async fn load_children()` |
| `git/` | `async fn get_status()` |
| `preview/` | `async fn load_preview()` |
| `event_loop` | `tokio::select!` ベースに |

**リスク**:
- 大規模リファクタリングによる不安定化
- 起動時間への影響（tokio runtime初期化）
- 既存のテストの全面書き換え
- シンプルさの設計原則との矛盾

---

## 優先順位と推奨順序

### 推奨: Phase 12 → Phase 13

| 順位 | Phase | 理由 |
|------|-------|------|
| 1 | Phase 12: Lua プラグイン | yaziとの最大の差を埋められる。ユーザーの需要が高い |
| 2 | Phase 13: WASM プラグイン | Luaの代替として検討。Phase 12 と排他的 |
| 保留 | Phase 14: Async化 | リスクが高く、ユーザーメリットが見えにくい |

### 代替案: プラグインなしで差別化

プラグインシステムは「やらないこと」に含まれていた。代わりに:

1. **カスタムコマンドの強化**（v1.17.0で実装済み）
2. **カスタムプレビューの拡充**（v1.17.0で実装済み）
3. **イベントフックの拡張**（v1.18.0で実装済み）

これらの組み合わせで「Lua不要の軽量カスタマイズ」として差別化する方向性もある。

---

## 目標スコア（v4 完了後）

### Phase 12 実装後

| カテゴリ | v1.18.0 | v4目標 | 増分 |
|----------|---------|--------|------|
| プラグイン/拡張性 | 7/10 | 10/10 | +3 |
| **合計** | **96/105** | **99/105** | **+3** |

### 競合との比較（v4 完了後）

| ツール | スコア |
|--------|--------|
| **fileview (v4)** | **99点** |
| yazi | 98点 |

**yaziを超える可能性あり**

---

## 設計原則との整合性

### 現在の設計原則

> 1. **シンプルに保つ** - 機能追加より安定性
> 2. **高速起動** - ターミナルツールは軽くあるべき
> 3. **ゼロ設定** - インストールしてすぐ使える
> 4. **画像プレビュー** - これが差別化ポイント

### v4 で変更が必要な原則

| 原則 | v4での変更 |
|------|-----------|
| プラグインシステム | 「やらないこと」→「やること」に移動 |
| ゼロ設定 | 「プラグインなしでもフル機能」を維持 |

**重要**: プラグインは**オプション**であり、プラグインなしでも fileview は完全に動作する。「ゼロ設定」の原則は維持される。

---

## 次のステップ

### v4 開発を開始する前に

1. **ユーザーフィードバックの収集**: プラグインシステムへの需要を確認
2. **mlua の PoC**: 小規模なプロトタイプで技術検証
3. **API 設計の確定**: プラグインに公開するAPI の詳細設計
4. **セキュリティモデルの設計**: サンドボックス化の方針決定

### 検討事項

- Lua vs WASM vs 両方サポート
- プラグインのパッケージ管理（npm/cargo 的なもの）
- プラグインのバージョニング
- 後方互換性の維持方針

---

## 参考リンク

- [mlua - High level Lua bindings for Rust](https://github.com/khvzak/mlua)
- [wasmtime - A fast and secure runtime for WebAssembly](https://github.com/bytecodealliance/wasmtime)
- [Yazi Lua Plugin Docs](https://yazi-rs.github.io/docs/plugins/overview)
- [tokio - A runtime for writing reliable asynchronous applications](https://tokio.rs/)
