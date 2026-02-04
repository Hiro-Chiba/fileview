# FileView Roadmap (2026-02-04)

## Vision

FileView は「単体FMの機能競争」ではなく、**AI駆動開発の補助レイヤー**として最速で価値を出すことを優先します。

- Zero config
- Fast startup
- AI context workflow (Claude/Codex 併用時の生産性最大化)

## Current Position

- Current channel: `stable`
- Current version: `2.1.0`
- Strength: MCP + context-pack + lightweight TUI
- Gap to close: 認知/導入導線、AI専用ワークフローの完成度、継続利用ループ

## 90-Day Product Focus

### Phase A: AI quick wins (2週間)
- `init claude` 導入導線強化
- review向け context-pack の即時コピー導線
- 名前付きAIセッション復元導線の運用改善

### Phase B: Review/Bug loop強化 (2〜4週間)
- Git range 起点の review context 自動生成
- related-file 選定理由の可視化精度向上
- エラー調査用コンテキストのテンプレ化

### Phase C: Adoption loop (4〜8週間)
- ドキュメント多言語同期と導入テンプレ拡充
- 主要ユースケース別の短いチュートリアル追加
- CLI/MCPの後方互換ポリシーの明文化

## Release History (Our Journey)

| Version | Date | Channel | Highlights |
|---|---|---|---|
| `v2.1.0` | 2026-02-04 | stable | stable昇格、alpha exit policy導入 |
| `v2.0.4-alpha` | 2026-02-04 | alpha | 安定版直前の最終調整 |
| `v2.0.3-alpha` | 2026-02-04 | alpha | AI context automation強化 |
| `v2.0.2-alpha` | 2026-02-04 | alpha | 2.xラインの継続改善 |
| `v2.0.1-alpha` | 2026-02-03 | alpha | セキュリティ強化（コマンド/パス保護） |
| `v2.0.0-alpha` | 2026-02-03 | alpha | MCP 2.0・AIネイティブ機能群 |
| `v1.26.0` | 2026-02-03 | stable | セッション永続化、AI選択系ショートカット |
| `v1.25.2` | 2026-02-03 | stable | 1.x系安定化 |
| `v1.25.1` | 2026-02-03 | stable | 1.x系安定化 |
| `v1.25.0` | 2026-02-03 | stable | 1.x系機能拡張 |
| `v1.24.1` | 2026-02-03 | stable | Claude Code連携ドキュメント拡充 |
| `v1.24.0` | 2026-02-03 | stable | Claude連携基盤（`--tree`, `--select-mode`, `--mcp-server`） |
| `v1.23.0` | 2026-02-03 | stable | 狭幅ターミナル向け適応UI |
| `v1.22.0` | 2026-02-03 | stable | Lua plugin loader統合 |
| `v1.21.0` | 2026-02-03 | stable | v4機能群拡張 |
| `v1.20.0` | 2026-02-03 | stable | v4機能群拡張 |
| `v1.19.0` | 2026-02-03 | stable | Lua runtime統合 |
| `v1.18.0` | 2026-02-03 | stable | 動画プレビュー、フック、タブ、バッチ操作 |
| `v1.17.0` | 2026-01-31 | stable | カスタムコマンド/プレビュー |
| `v1.16.0` | 2026-01-31 | stable | 設定ファイル/キーマップ/テーマ |

> 詳細な差分は `CHANGELOG.md` を参照。

## Success Metrics

- AI workflow開始までの平均時間を 30%短縮
- context-pack 利用率 50%向上
- stableラインで重大バグ（クラッシュ/データ破壊）ゼロ継続
- ドキュメント更新の英日同期遅延を1リリース以内に抑制
