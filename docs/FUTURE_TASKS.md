# Future Tasks / 将来のタスク

This document tracks planned improvements and configurations that are not yet implemented.

このドキュメントは、まだ実装されていない予定の改善や設定を追跡します。

---

## 1. Codecov Integration / Codecov 連携

### English

**Purpose:** Visualize and track test coverage over time.

**Benefits:**
- See which parts of the code are tested/untested
- Automatic PR comments showing coverage changes
- Display coverage badge in README
- Track coverage trends over time

**Setup Steps:**
1. Visit https://codecov.io and log in with GitHub
2. Add the `fileview` repository
3. Go to `Settings` → `General` and copy the **Upload Token**
4. Add to GitHub: `Repository Settings` → `Secrets and variables` → `Actions`
   - Name: `CODECOV_TOKEN`
   - Value: (paste the token)

**Priority:** Low - Nice to have for OSS credibility

---

### 日本語

**目的:** テストカバレッジを可視化し、時系列で追跡する。

**メリット:**
- コードのどの部分がテストされているか/されていないかを確認できる
- PRへの自動コメントでカバレッジの変化を表示
- READMEにカバレッジバッジを表示できる
- カバレッジの推移をグラフで確認できる

**設定手順:**
1. https://codecov.io にアクセスし、GitHubでログイン
2. `fileview` リポジトリを追加
3. `Settings` → `General` で **Upload Token** をコピー
4. GitHubに追加: `Repository Settings` → `Secrets and variables` → `Actions`
   - Name: `CODECOV_TOKEN`
   - Value: (トークンを貼り付け)

**優先度:** 低 - OSSとしての信頼性向上に有用

---

## 2. Automated crates.io Publishing / crates.io 自動公開

### English

**Purpose:** Automatically publish to crates.io when a new version tag is pushed.

**Benefits:**
- No manual `cargo publish` needed
- Consistent release process
- Reduces human error in releases

**Setup Steps:**
1. Visit https://crates.io and log in with GitHub
2. Go to `Account Settings` → `API Tokens` → `New Token`
3. Name: `github-actions`
4. Scopes: Check `publish-new` and `publish-update`
5. Click `Generate Token` and copy (shown only once!)
6. Add to GitHub: `Repository Settings` → `Secrets and variables` → `Actions`
   - Name: `CARGO_REGISTRY_TOKEN`
   - Value: (paste the token)

**Current Workflow:**
The release workflow (`.github/workflows/release.yml`) is already configured to publish automatically. It just needs the token to be set.

**Priority:** Medium - Useful when release frequency stabilizes

---

### 日本語

**目的:** 新しいバージョンタグがプッシュされたときに、crates.ioへ自動公開する。

**メリット:**
- 手動での `cargo publish` が不要になる
- 一貫したリリースプロセス
- リリース時のヒューマンエラーを削減

**設定手順:**
1. https://crates.io にアクセスし、GitHubでログイン
2. `Account Settings` → `API Tokens` → `New Token` に移動
3. Name: `github-actions`
4. Scopes: `publish-new` と `publish-update` にチェック
5. `Generate Token` をクリックしてコピー（一度しか表示されない！）
6. GitHubに追加: `Repository Settings` → `Secrets and variables` → `Actions`
   - Name: `CARGO_REGISTRY_TOKEN`
   - Value: (トークンを貼り付け)

**現在のワークフロー:**
リリースワークフロー（`.github/workflows/release.yml`）は自動公開するよう既に設定済み。トークンの設定だけが必要。

**優先度:** 中 - リリース頻度が安定したら有用

---

## Status / ステータス

| Task | Status | Notes |
|------|--------|-------|
| Codecov Integration | Pending | CI already generates coverage, just needs token |
| crates.io Auto-publish | Pending | Workflow ready, just needs token |

| タスク | ステータス | 備考 |
|--------|-----------|------|
| Codecov 連携 | 未着手 | CIはカバレッジを生成済み、トークンのみ必要 |
| crates.io 自動公開 | 未着手 | ワークフロー準備済み、トークンのみ必要 |
