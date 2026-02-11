# FileView への貢献

FileViewへの貢献に感謝します。本ドキュメントでは、開発に参加する際のルールとガイドラインを定めます。

## 目次

- [開発環境のセットアップ](#開発環境のセットアップ)
- [コーディング規約](#コーディング規約)
- [Git ワークフロー](#git-ワークフロー)
- [コミット規約](#コミット規約)
- [プルリクエストガイドライン](#プルリクエストガイドライン)
- [テスト要件](#テスト要件)
- [ドキュメント更新](#ドキュメント更新)

---

## 開発環境のセットアップ

### 前提条件

- Rust 1.75.0+
- cargo
- Git

### ビルド

```bash
git clone https://github.com/Hiro-Chiba/fileview.git
cd fileview
cargo build
cargo test
```

---

## コーディング規約

### 命名規則

| 項目 | 規則 | 例 |
|------|------|-----|
| 変数 | snake_case | `file_path`, `tree_state` |
| 関数 | snake_case | `get_entries()`, `render_tree()` |
| 型/構造体 | PascalCase | `AppState`, `FileEntry` |
| Enum | PascalCase | `OperationMode` |
| Enumバリアント | PascalCase | `OperationMode::Normal` |
| 定数 | SCREAMING_SNAKE_CASE | `MAX_PREVIEW_LINES` |
| モジュール | snake_case | `file_system`, `event_handler` |
| トレイト | PascalCase | `Renderable`, `FileOperation` |

### コードスタイル

- **インデント**: スペース4つ
- **行の長さ**: 最大100文字（ドキュメントは80文字推奨）
- **import順序**:
  1. 標準ライブラリ (`std::`)
  2. 外部クレート
  3. 内部モジュール (`crate::`, `super::`)

```rust
// 良い例
use std::path::PathBuf;

use ratatui::Frame;
use tokio::sync::mpsc;

use crate::app::AppState;
use crate::event::Event;
```

### ドキュメント

- 公開API（`pub`）には必ずドキュメントコメント（`///`）を付ける
- 複雑なロジックにはインラインコメント（`//`）を付ける
- TODOコメントは `// TODO(username): description` 形式

### エラーハンドリング

- `unwrap()` / `expect()` は原則禁止（テストコード除く）
- エラーは `thiserror` で定義し、`Result<T, E>` で返す
- 致命的エラーのみ `panic!`

---

## Git ワークフロー

### ブランチ戦略

```
main
  │
  └── feature/xxx    # 新機能開発
  └── fix/xxx        # バグ修正
  └── refactor/xxx   # リファクタリング
  └── docs/xxx       # ドキュメント更新
  └── test/xxx       # テスト追加
```

### ブランチ命名

```
<type>/<short-description>

例:
  feature/add-preview-panel
  fix/tree-scroll-overflow
  refactor/event-handler
  docs/update-readme
```

---

## コミット規約

[Conventional Commits](https://www.conventionalcommits.org/ja/) に準拠する。

### フォーマット

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Type

| Type | 説明 |
|------|------|
| `feat` | 新機能追加 |
| `fix` | バグ修正 |
| `docs` | ドキュメントのみの変更 |
| `style` | コードの意味に影響しない変更（空白、フォーマット等） |
| `refactor` | バグ修正でも機能追加でもないコード変更 |
| `perf` | パフォーマンス改善 |
| `test` | テストの追加・修正 |
| `chore` | ビルドプロセスや補助ツールの変更 |

### 例

```bash
# 良い例
feat(ui): Add syntax highlighting to preview panel
fix(fs): Handle symlink loop detection
refactor(event): Extract key binding logic to separate module
docs: Update installation instructions
chore: Bump ratatui to 0.26

# 悪い例
added new feature          # 命令形でない、typeがない
feat: Fixed bug            # typeとsubjectが矛盾
```

---

## プルリクエストガイドライン

### PR作成前

1. **テスト通過を確認**
   ```bash
   cargo test
   cargo clippy -- -D warnings
   cargo fmt --check
   ```

2. **コミット履歴を整理**
   - 意味のある単位でコミットをまとめる
   - WIPコミットは squash する

3. **ブランチを最新に**
   ```bash
   git fetch origin
   git rebase origin/main
   ```

### PRタイトル

コミットメッセージと同じ形式を使用:

```
feat(ui): Add file preview panel
```

### レビュープロセス

1. 自動チェック（CI）がすべてパス
2. 1名以上のレビュー承認
3. コンフリクトがない状態
4. Squash merge で main にマージ

---

## テスト要件

PRを作成する前に、以下のチェックをすべて通過させてください：

```bash
# テスト実行
cargo test

# Lintチェック（警告はエラーとして扱う）
cargo clippy -- -D warnings

# フォーマットチェック
cargo fmt --check
```

これらはCIでも自動チェックされます。

---

## ドキュメント更新

機能追加・変更時のドキュメント更新ルール：

1. **README.md** と **README_ja.md** は常に同期
   - 英語版を先に更新し、日本語版も同じ内容に更新
   - 機能の追加・削除・変更は両方に反映

2. **実装済み機能のみ記載**
   - 未実装の機能を記載しない
   - 過大な表現を避ける
   - シンプルで正確な表現を心がける

---

## 質問がある場合

不明点があれば、Issue を作成してください。
