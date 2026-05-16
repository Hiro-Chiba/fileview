# Contributing to FileView

Thanks for considering a contribution. This document spells out the
conventions used when working on the project.

## Table of Contents

- [Development Setup](#development-setup)
- [Coding Standards](#coding-standards)
- [Git Workflow](#git-workflow)
- [Commit Convention](#commit-convention)
- [Pull Request Guidelines](#pull-request-guidelines)
- [Testing Requirements](#testing-requirements)
- [Documentation Updates](#documentation-updates)

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

- **Indentation**: 4 spaces
- **Line length**: 100 characters max (80 preferred for documentation)
- **Import order**:
  1. Standard library (`std::`)
  2. External crates
  3. Internal modules (`crate::`, `super::`)

```rust
// Good
use std::path::PathBuf;

use ratatui::Frame;
use tokio::sync::mpsc;

use crate::app::AppState;
use crate::event::Event;
```

### Documentation

- Public APIs (`pub`) must carry doc comments (`///`).
- Complex logic gets inline comments (`//`).
- TODO comments follow the form `// TODO(username): description`.

### Error Handling

- Avoid `unwrap()` / `expect()` outside tests.
- Define errors with `thiserror` and return `Result<T, E>`.
- Reserve `panic!` for unrecoverable invariants.

---

## Git Workflow

### Branch Strategy

```
main
  │
  └── feature/xxx    # new features
  └── fix/xxx        # bug fixes
  └── refactor/xxx   # refactoring
  └── docs/xxx       # documentation updates
  └── test/xxx       # added or updated tests
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

Follows [Conventional Commits](https://www.conventionalcommits.org/).

### Format

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Type

| Type | Description |
|------|-------------|
| `feat` | new feature |
| `fix` | bug fix |
| `docs` | documentation only |
| `style` | formatting / whitespace, no logic change |
| `refactor` | code change that is neither a fix nor a feature |
| `perf` | performance improvement |
| `test` | added or fixed tests |
| `chore` | build process or tooling change |

### Scope (optional)

Name the module the change targets, e.g. `ui`, `event`, `fs`, `config`, `app`.

### Subject

- Imperative mood ("Add", "Fix", "Update").
- Start with a capital letter.
- No trailing period.
- 50 characters or fewer.

### Examples

```bash
# Good
feat(ui): Add syntax highlighting to preview panel
fix(fs): Handle symlink loop detection
refactor(event): Extract key binding logic to separate module
docs: Update installation instructions
chore: Bump ratatui to 0.26

# Bad
added new feature          # not imperative, no type
feat: Fixed bug            # type contradicts subject
FEAT(UI): ADD FEATURE.     # shouting, trailing period
```

### Breaking Changes

For breaking changes, add `!` and include a `BREAKING CHANGE:` footer:

```
feat(config)!: Change configuration file format

BREAKING CHANGE: Configuration file format changed from JSON to TOML.
Migrate existing config.json to config.toml.
```

---

## Pull Request Guidelines

### Before Creating PR

1. **Confirm tests pass**
   ```bash
   cargo test
   cargo clippy -- -D warnings
   cargo fmt --check
   ```

2. **Clean up commits** — group changes into meaningful commits, squash
   any WIP commits.

3. **Rebase onto the latest main**
   ```bash
   git fetch origin
   git rebase origin/main
   ```

### PR Title

Same format as commit messages:

```
feat(ui): Add file preview panel
```

### PR Description Template

```markdown
## Summary

1-3 lines describing what changed.

## Changes

- change 1
- change 2
- change 3

## Test Plan

- [ ] test item 1
- [ ] test item 2

## Screenshots (if applicable)

Attach screenshots for UI changes.

## Related Issues

Closes #123
```

### Review Process

1. All CI checks pass.
2. At least one review approval.
3. No merge conflicts.
4. Squash merge into `main`.

### Merge Strategy

- **Squash and merge**: collapse multiple commits into one on merge.
- The merge commit message defaults to the PR title.

---

## Testing Requirements

Run all of the following before opening a PR. CI re-checks them too.

```bash
# tests
cargo test

# lint (warnings are treated as errors)
cargo clippy -- -D warnings

# formatting
cargo fmt --check
```

---

## Documentation Updates

Rules when adding or changing features:

1. **Document only implemented features.** Skip anything that is not in
   the current code. Avoid grand framing ("VSCode-style", etc.) in
   favour of plain, accurate language.

2. **When keybindings change:**
   - Update the keybinding tables in `README.md` and `docs/KEYBINDINGS.md`.
   - Update `print_help()` in `src/main.rs`.
   - Update the help text in `src/handler/action.rs`.

---

## Questions?

Open an issue if anything is unclear.
