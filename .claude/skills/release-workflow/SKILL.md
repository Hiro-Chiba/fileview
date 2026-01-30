---
name: release-workflow
description: fileview プロジェクトのリリースワークフロー。リリース、バージョン更新、PR作成、マージ、タグ作成、GitHub/crates.io への公開を行う際に使用。"リリースして"、"バージョンを上げて"、"v1.2.0 をリリース"、"crates.io に公開して" などのリクエストで発動。
---

# Release Workflow for fileview

## バージョン番号ルール (Semantic Versioning)

```
MAJOR.MINOR.PATCH (例: 1.2.3)
  │     │     └── パッチ: バグ修正、軽微な変更
  │     └──────── マイナー: 新機能追加（後方互換性あり）
  └────────────── メジャー: 破壊的変更、大型リリース
```

| リリース種別 | 変更箇所 | 例 |
|-------------|---------|-----|
| パッチ | 右の数字 | 1.0.0 → 1.0.1 |
| マイナー | 真ん中の数字 | 1.0.1 → 1.1.0 |
| メジャー | 左の数字 | 1.1.0 → 2.0.0 |

## リリース手順

### 1. ブランチ作成
```bash
git checkout -b release/vX.Y.Z
```

### 2. バージョン更新
更新対象ファイル:
- `Cargo.toml`: `version = "X.Y.Z"`
- `CHANGELOG.md`: 新しいエントリを追加

### 3. PR 作成
PRタイトルは必ずリリース番号を含める:
```
release: vX.Y.Z - <簡潔な説明>
```

### 4. ローカルCI実行 (GitHub Actions は使用しない)
```bash
cargo check
cargo fmt --all -- --check
cargo clippy -- -D warnings
cargo test
```
全て通過するまでマージしない。

### 5. マージ
```bash
gh pr merge <PR番号> --merge
git checkout main && git pull
```

### 6. タグ作成 & GitHub リリース
```bash
git tag -a vX.Y.Z -m "vX.Y.Z: <説明>"
git push origin vX.Y.Z
gh release create vX.Y.Z --title "vX.Y.Z" --notes "<リリースノート>"
```

### 7. crates.io 公開
```bash
cargo publish
```

### 8. クリーンアップ
```bash
git branch -d release/vX.Y.Z
git push origin --delete release/vX.Y.Z
```

## チェックリスト

- [ ] Cargo.toml のバージョン更新
- [ ] CHANGELOG.md の更新
- [ ] PRタイトルにバージョン番号
- [ ] `cargo check` 通過
- [ ] `cargo fmt --all -- --check` 通過
- [ ] `cargo clippy -- -D warnings` 通過
- [ ] `cargo test` 通過
- [ ] PR マージ
- [ ] タグ作成 & プッシュ
- [ ] GitHub リリース作成
- [ ] crates.io 公開
- [ ] ブランチ削除
