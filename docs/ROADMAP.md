# FileView - Implementation Roadmap

## Overview

myfileのロジックを参考にしつつ、**構造・命名を独自設計**した別プロジェクトとして実装する。

---

## Phase 1: Foundation

- [x] 1.1 プロジェクト初期化
  - Cargo.toml
  - .gitignore
  - PR: `chore: Initialize Rust project`

- [x] 1.2 CI設定
  - .github/workflows/ci.yml
  - PR: `chore: Set up GitHub Actions CI`

- [x] 1.3 モジュール構造作成
  - src/lib.rs + 各モジュールのmod.rs
  - PR: `chore: Set up module structure`

---

## Phase 2: Core Module

- [x] 2.1 core/state.rs
  - AppState構造体
  - PR: `feat(core): Define AppState`

- [x] 2.2 core/mode.rs
  - ViewMode enum（状態内包型）
  - InputPurpose, PendingAction
  - PR: `feat(core): Define ViewMode with embedded state`

---

## Phase 3: Tree Module

- [x] 3.1 tree/node.rs
  - TreeEntry構造体
  - PR: `feat(tree): Define TreeEntry`

- [x] 3.2 tree/navigator.rs
  - TreeNavigator構造体
  - フラット化（flatten / collect_visible）
  - 展開/折りたたみ
  - PR: `feat(tree): Implement TreeNavigator with flatten`

---

## Phase 4: Action Module

- [x] 4.1 action/file.rs
  - create_file / create_dir
  - rename / delete
  - PR: `feat(action): Implement file operations`

- [x] 4.2 action/clipboard.rs
  - copy / cut / paste
  - Clipboard構造体
  - PR: `feat(action): Implement clipboard operations`

---

## Phase 5: Render Module

- [x] 5.1 render/tree.rs
  - ツリー描画
  - PR: `feat(render): Implement tree rendering`

- [x] 5.2 render/preview.rs
  - テキストプレビュー
  - 画像プレビュー（半ブロック）
  - PR: `feat(render): Implement preview rendering`

- [x] 5.3 render/status.rs
  - ステータスバー
  - 入力UI
  - PR: `feat(render): Implement status bar`

---

## Phase 6: Handler Module

- [x] 6.1 handler/key.rs
  - キーイベント処理
  - モード別ハンドラー
  - PR: `feat(handler): Implement key handling`

- [x] 6.2 handler/mouse.rs
  - マウスイベント処理
  - ダブルクリック検出
  - PR: `feat(handler): Implement mouse handling`

- [x] 6.3 DropDetector
  - D&D検出
  - PR: `feat(handler): Implement drag and drop detection`

---

## Phase 7: Integrate Module（独自機能）

- [x] 7.1 integrate/pick.rs
  - --pick オプション
  - stdout出力
  - 終了コード
  - PR: `feat(integrate): Implement --pick mode`

- [x] 7.2 integrate/callback.rs
  - --on-select オプション
  - プレースホルダー展開
  - PR: `feat(integrate): Implement --on-select callback`

---

## Phase 8: Main & Polish

- [x] 8.1 main.rs
  - イベントループ
  - ターミナル初期化/復元
  - PR: `feat: Implement main event loop`

- [ ] 8.2 README.md
  - インストール、使用方法
  - PR: `docs: Add README`

- [ ] 8.3 テスト
  - tree, action のユニットテスト
  - PR: `test: Add unit tests`

---

## Progress Summary

| Phase | Items | Completed |
|-------|-------|-----------|
| 1. Foundation | 3 | 3 |
| 2. Core | 2 | 2 |
| 3. Tree | 2 | 2 |
| 4. Action | 2 | 2 |
| 5. Render | 3 | 3 |
| 6. Handler | 3 | 3 |
| 7. Integrate | 2 | 2 |
| 8. Main & Polish | 3 | 1 |
| **Total** | **20** | **18** |

---

## 構造の違い（myfile vs fileview）

```
myfile (フラット)          fileview (モジュール階層)
─────────────────          ─────────────────────────
app.rs            →        core/state.rs, core/mode.rs
file_tree.rs      →        tree/node.rs, tree/navigator.rs
file_ops.rs       →        action/file.rs, action/clipboard.rs
ui.rs             →        render/tree.rs, render/preview.rs, render/status.rs
input.rs          →        handler/key.rs, handler/mouse.rs
git_status.rs     →        (削除)
-                 →        integrate/pick.rs, integrate/callback.rs (追加)
```

---

## 命名の違い

| myfile | fileview |
|--------|----------|
| App | AppState |
| InputMode | ViewMode |
| FileNode | TreeEntry |
| FileTree | TreeNavigator |
| flat_list | visible_entries |
| selected | focus_index |
| scroll_offset | viewport_top |
| marked | selected_paths |

---

## Notes

- ロジックの「考え方」は参考にするが、コードは独自に書く
- 構造・命名・モジュール分割で「別物」感を出す
- Git統合は実装しない
- 外部コマンド（:）は --on-select で代替
