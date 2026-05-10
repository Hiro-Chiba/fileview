# リリースポリシー

## 目標

リリースを「速さ」と「分かりやすさ」の両立で運用する。

- 開発速度は落とさない
- ユーザーには常に「今使うべき版」を明確に示す

## バージョンチャンネル

### Stable (`x.y.z`)
- 一般ユーザー向けの推奨版
- 互換性と安定性を優先

### Pre-release (`x.y.z-alpha.N`, `x.y.z-rc.N`)
- 早期検証向け
- 破壊的変更や挙動変更を含む可能性あり

## リリース頻度

1. Stable は月1〜2回を基本
2. 重大修正は patch (`x.y.z+1`) を即時リリース
3. 細かな改善は pre-release に集約

## 昇格ルール

Stable 昇格前に以下を満たすこと:

1. `cargo check`
2. `cargo clippy --all-targets -- -D warnings`
3. `cargo test`
4. `cargo audit`（運用ポリシー込みで許容状態）
5. 重大バグ（クラッシュ/データ破壊/緊急セキュリティ修正）が直近サイクルで発生していない

## タグ付け

1. 各公開版に Git tag を付与（例: `v2.3.0`, `v2.3.1`）
2. pre-release は suffix を含める（例: `v2.4.0-alpha.1`）
3. superseded な版（例: 公開直後に置き換え）を `CHANGELOG.md` に明記

## crates.io への公開

crates.io への公開は**手動**で行う（CI からは行わない）。

```bash
cargo publish --no-verify
```

GitHub Actions の `release.yml` はタグ push 時にバイナリビルドと
GitHub Release 作成のみを自動化する。crates.io publish を CI 経由にしないのは、
公開タイミングをメンテナ側で制御できるようにするため、および
`CARGO_REGISTRY_TOKEN` を Actions secret として常時持ち込まないため。

## コミュニケーションルール

毎リリースで以下を更新:

1. `CHANGELOG.md`
2. `README.md`（必要なら推奨版や導線）
3. `README_ja.md`

## 「推奨版」ポリシー

- 常に最新 stable を推奨版とする
- pre-release は検証目的として明示する

## 実践例

- 開発中: `2.4.0-alpha.1` → `2.4.0-alpha.2`
- 収束後: `2.4.0-rc.1`
- 安定化完了: `2.4.0`（推奨版）
- 緊急修正: `2.4.1`

## リリースノート形式

GitHub Releases の本文は次のテンプレで統一する。`CHANGELOG.md` の
"Keep a Changelog" 形式と整合させ、過去の小刻みリリースで生じていた
言語・見出し・密度のばらつきを抑える。

### テンプレート

```markdown
# vX.Y.Z[ - Optional Subtitle]

One or two sentence English summary describing the release theme.

## Highlights

- Headline feature one
- Headline feature two

## Added

- New CLI flag `--foo` for X (#PR)
- New keybinding `Alt+B` for Y (#PR)

## Changed

- Behavior of Z now does W (was V) (#PR)

## Fixed

- Bug where A caused B (#PR)

## Notes

Breaking changes, migration steps, or compatibility notes. Omit when
the section would be empty.

---

**Full Changelog**: https://github.com/Hiro-Chiba/fileview/compare/PREV...vX.Y.Z
```

### ルール

1. **言語**: 英語のみ。TUI/CLI が英語ベースなので Releases も合わせる。
   日本語の記述は `README_ja.md` や `CHANGELOG_ja.md` に集約し、
   GitHub Releases には混在させない。
2. **タイトル**: `vX.Y.Z` を必須とする。節目リリースには ` - ` 区切り
   でサブタイトルを付けて良い（例: `v1.24.0 - Claude Code Integration`）。
3. **見出しレベル**: 本文は h2 (`##`) から始める。`# FileView vX.Y.Z`
   のような h1 を本文先頭には使わない（タイトルフィールドが同じ役割を
   果たすため重複になる）。
4. **セクション順序**: Highlights → Added → Changed → Fixed → Notes。
   空のセクションはプレースホルダを残さず省略する。
5. **絵文字なし**: 装飾目的の絵文字は使わない。README も marketing 表現
   を抑える方針なので、Releases もそれに揃える。
6. **PR 参照**: 各 bullet の末尾に `(#NNN)` を付ける（PR が出元の場合）。
7. **Full Changelog**: 末尾に GitHub 自動生成の compare リンクを置き、
   commit レベルで深掘りできるようにする。

### auto-generate ボタンだけに頼らない理由

GitHub の "Generate release notes" は PR タイトル一覧を吐くだけで、
ユーザ目線の "何が変わったか" がぼやける。Highlights を 2〜3 行手書きし
てからセクションを並べることで、Releases ページを開いた瞬間に価値が
伝わるリリースノートになる。
