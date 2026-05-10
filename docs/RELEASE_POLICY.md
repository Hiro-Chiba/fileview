# Release Policy

## Goal

リリースを「速さ」と「分かりやすさ」の両立で運用する。

- 開発速度は落とさない
- ユーザーには常に「今使うべき版」を明確に示す

## Version Channels

### Stable (`x.y.z`)
- 一般ユーザー向けの推奨版
- 互換性と安定性を優先

### Pre-release (`x.y.z-alpha.N`, `x.y.z-rc.N`)
- 早期検証向け
- 破壊的変更や挙動変更を含む可能性あり

## Cadence

1. Stable は月1〜2回を基本
2. 重大修正は patch (`x.y.z+1`) を即時リリース
3. 細かな改善は pre-release に集約

## Promotion Rules

Stable 昇格前に以下を満たすこと:

1. `cargo check`
2. `cargo clippy --all-targets -- -D warnings`
3. `cargo test`
4. `cargo audit`（運用ポリシー込みで許容状態）
5. 重大バグ（クラッシュ/データ破壊/緊急セキュリティ修正）が直近サイクルで発生していない

## Tagging

1. 各公開版に Git tag を付与（例: `v2.3.0`, `v2.3.1`）
2. pre-release は suffix を含める（例: `v2.4.0-alpha.1`）
3. superseded な版（例: 公開直後に置き換え）を `CHANGELOG.md` に明記

## Publishing to crates.io

crates.io への公開は**手動**で行う（CI では行わない）。

```bash
cargo publish --no-verify
```

GitHub Actions の `release.yml` はタグ push でバイナリビルドと GitHub
Release 作成のみ自動化する。crates.io publish を CI から外してあるのは、
公開タイミングをメンテナ側で制御できるようにするため、および
`CARGO_REGISTRY_TOKEN` を Actions secret として常時持ち込まないため。

## Communication Rules

毎リリースで以下を更新:

1. `CHANGELOG.md`
2. `README.md`（必要なら推奨版や導線）
3. `README_ja.md`

## “Recommended Version” Policy

- 常に最新 stable を推奨版とする
- pre-release は検証目的として明示する

## Practical Example

- 開発中: `2.4.0-alpha.1` → `2.4.0-alpha.2`
- 収束後: `2.4.0-rc.1`
- 安定化完了: `2.4.0`（推奨版）
- 緊急修正: `2.4.1`

## Release Notes Format

GitHub Releases ページの本文は以下のテンプレで統一する。`CHANGELOG.md`
の "Keep a Changelog" 形式と整合するセクション分けで、過去の小刻みリリー
スで生じた言語・見出し・密度のばらつきを抑える。

### Template

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

### Rules

1. **Language**: English only. The TUI and CLI are English first; release
   notes follow the same convention. Japanese narrative belongs in
   `README_ja.md` and `CHANGELOG_ja.md`, not in GitHub Releases.
2. **Title**: `vX.Y.Z` is required. An optional subtitle separated by
   ` - ` is allowed for milestone releases (e.g. `v1.24.0 - Claude Code
   Integration`).
3. **Heading levels**: The release body starts at h2 (`##`). Do not use
   `# FileView vX.Y.Z` as a body heading; the `vX.Y.Z` title field
   already plays that role on the Releases page.
4. **Section order**: Highlights, Added, Changed, Fixed, Notes. Omit any
   section that would be empty rather than printing a placeholder.
5. **No emojis**: Avoid decorative emojis throughout the body. The
   README also drops marketing phrasing, and Releases follow that tone.
6. **Pull request references**: Append `(#NNN)` to each bullet when a
   pull request is the source of the change.
7. **Full Changelog footer**: End with the GitHub auto-generated compare
   link so readers can drill into commit-level diffs.

### Why not rely on auto-generated notes alone?

GitHub の "Generate release notes" ボタンは PR タイトル一覧を出すだけ
で、ユーザ目線の "何が変わったか" がぼやける。Highlights を 2〜3 行手で
書いてからセクションを並べることで、Releases ページを開いた瞬間に価値が
伝わるリリースノートにする。
