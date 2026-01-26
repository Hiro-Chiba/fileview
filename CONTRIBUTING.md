# Contributing to FileView

FileViewへの貢献に感謝します。本ドキュメントでは、開発に参加する際のルールとガイドラインを定めます。

## Table of Contents

- [Development Setup](#development-setup)
- [Coding Standards](#coding-standards)
- [Git Workflow](#git-workflow)
- [Commit Convention](#commit-convention)
- [Pull Request Guidelines](#pull-request-guidelines)

---

## Development Setup

### Prerequisites

- Rust 1.75.0+
- cargo
- Git

### Build

```bash
git clone https://github.com/Hiro-Chiba/fileview.git
cd fileview
cargo build
cargo test
```

---

## Coding Standards

### Naming Conventions

| Item | Convention | Example |
|------|------------|---------|
| Variables | snake_case | `file_path`, `tree_state` |
| Functions | snake_case | `get_entries()`, `render_tree()` |
| Types/Structs | PascalCase | `AppState`, `FileEntry` |
| Enums | PascalCase | `OperationMode` |
| Enum Variants | PascalCase | `OperationMode::Normal` |
| Constants | SCREAMING_SNAKE_CASE | `MAX_PREVIEW_LINES` |
| Modules | snake_case | `file_system`, `event_handler` |
| Traits | PascalCase | `Renderable`, `FileOperation` |

### Code Style

- **インデント**: スペース4つ
- **行の長さ**: 最大100文字（ドキュメントは80文字推奨）
- **import順序**:
  1. 標準ライブラリ (`std::`)
  2. 外部クレート
  3. 内部モジュール (`crate::`, `super::`)

```rust
// Good
use std::path::PathBuf;

use ratatui::Frame;
use tokio::sync::mpsc;

use crate::app::AppState;
use crate::event::Event;
```

### Documentation

- 公開API（`pub`）には必ずドキュメントコメント（`///`）を付ける
- 複雑なロジックにはインラインコメント（`//`）を付ける
- TODOコメントは `// TODO(username): description` 形式

### Error Handling

- `unwrap()` / `expect()` は原則禁止（テストコード除く）
- エラーは `thiserror` で定義し、`Result<T, E>` で返す
- 致命的エラーのみ `panic!`

---

## Git Workflow

### Branch Strategy

```
main
  │
  └── feature/xxx    # 新機能開発
  └── fix/xxx        # バグ修正
  └── refactor/xxx   # リファクタリング
  └── docs/xxx       # ドキュメント更新
  └── test/xxx       # テスト追加
```

### Branch Naming

```
<type>/<short-description>

Examples:
  feature/add-preview-panel
  fix/tree-scroll-overflow
  refactor/event-handler
  docs/update-readme
```

---

## Commit Convention

[Conventional Commits](https://www.conventionalcommits.org/) に準拠する。

### Format

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Type

| Type | Description |
|------|-------------|
| `feat` | 新機能追加 |
| `fix` | バグ修正 |
| `docs` | ドキュメントのみの変更 |
| `style` | コードの意味に影響しない変更（空白、フォーマット等） |
| `refactor` | バグ修正でも機能追加でもないコード変更 |
| `perf` | パフォーマンス改善 |
| `test` | テストの追加・修正 |
| `chore` | ビルドプロセスや補助ツールの変更 |

### Scope (optional)

変更対象のモジュールを指定:
- `ui`, `event`, `fs`, `config`, `app`

### Subject

- 命令形で記述（"Add", "Fix", "Update"）
- 先頭は大文字
- 末尾にピリオドを付けない
- 50文字以内

### Examples

```bash
# Good
feat(ui): Add syntax highlighting to preview panel
fix(fs): Handle symlink loop detection
refactor(event): Extract key binding logic to separate module
docs: Update installation instructions
chore: Bump ratatui to 0.26

# Bad
added new feature          # 命令形でない、typeがない
feat: Fixed bug            # typeとsubjectが矛盾
FEAT(UI): ADD FEATURE.     # 大文字すぎ、ピリオド
```

### Breaking Changes

破壊的変更がある場合は `!` を追加し、footerに `BREAKING CHANGE:` を記述:

```
feat(config)!: Change configuration file format

BREAKING CHANGE: Configuration file format changed from JSON to TOML.
Migrate existing config.json to config.toml.
```

---

## Pull Request Guidelines

### Before Creating PR

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

### PR Title

コミットメッセージと同じ形式を使用:

```
feat(ui): Add file preview panel
```

### PR Description Template

```markdown
## Summary

変更内容の概要を1-3行で記述

## Changes

- 変更点1
- 変更点2
- 変更点3

## Test Plan

- [ ] テスト項目1
- [ ] テスト項目2

## Screenshots (if applicable)

UIの変更がある場合はスクリーンショットを添付

## Related Issues

Closes #123
```

### Review Process

1. 自動チェック（CI）がすべてパス
2. 1名以上のレビュー承認
3. コンフリクトがない状態
4. Squash merge で main にマージ

### Merge Strategy

- **Squash and merge**: 複数コミットを1つにまとめてマージ
- マージコミットメッセージはPRタイトルを使用

---

## Questions?

不明点があれば、Issue を作成してください。
