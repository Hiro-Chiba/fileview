# FileView - Implementation Roadmap

## Overview

本ドキュメントは FileView の実装計画を定義する。
**myfileの設計をベース**に、書き方を変えつつ実装する。

---

## Phase 1: Project Foundation

- [ ] 1.1 プロジェクト初期化
  - Cargo.toml（myfileと同じ依存クレート）
  - .gitignore
  - PR: `chore: Initialize Rust project`

- [ ] 1.2 CI設定
  - .github/workflows/ci.yml
  - PR: `chore: Set up GitHub Actions CI`

---

## Phase 2: Core Implementation（myfileベース）

- [ ] 2.1 main.rs
  - エントリーポイント
  - イベントループ（myfileベース）
  - ターミナル初期化/復元
  - PR: `feat: Implement main event loop`

- [ ] 2.2 file_tree.rs
  - FileNode構造体（→ TreeNode）
  - FileTree構造体
  - フラット化リスト（flat_list → visible_indices）
  - 展開/折りたたみ
  - PR: `feat(tree): Implement file tree with flatten`

- [ ] 2.3 app.rs
  - App構造体（→ AppState）
  - InputMode enum
  - 状態管理
  - PR: `feat(app): Implement application state`

- [ ] 2.4 input.rs
  - キーイベント処理
  - マウスイベント処理
  - モード別ハンドラー
  - PR: `feat(input): Implement input handling`

- [ ] 2.5 ui.rs
  - ツリー描画
  - ステータスバー
  - 入力UI
  - PR: `feat(ui): Implement UI rendering`

---

## Phase 3: File Operations（myfileベース）

- [ ] 3.1 file_ops.rs - 基本操作
  - create_file / create_dir
  - rename_file
  - delete_file
  - PR: `feat(ops): Implement basic file operations`

- [ ] 3.2 file_ops.rs - コピー/移動
  - copy_file
  - クリップボード管理
  - PR: `feat(ops): Implement copy and clipboard`

- [ ] 3.3 ドラッグ&ドロップ
  - バッファ検出（myfileベース）
  - パス正規化
  - PR: `feat: Implement drag and drop`

---

## Phase 4: Preview（myfileベース）

- [ ] 4.1 preview.rs - テキスト
  - テキストファイル表示
  - 行数制限
  - PR: `feat(preview): Implement text preview`

- [ ] 4.2 preview.rs - 画像
  - 半ブロック文字描画（myfileベース）
  - PR: `feat(preview): Implement image preview`

- [ ] 4.3 クイック/フルスクリーン
  - P キーでトグル
  - o キーでフルスクリーン
  - PR: `feat(preview): Add quick and fullscreen modes`

---

## Phase 5: Path Integration（fileview独自）

- [ ] 5.1 --pick モード
  - コマンドライン引数処理
  - Enter で stdout 出力
  - 終了コード
  - PR: `feat: Add --pick mode for path selection`

- [ ] 5.2 --on-select コールバック
  - プレースホルダー展開（{path}, {paths}）
  - 外部コマンド実行
  - PR: `feat: Add --on-select callback option`

- [ ] 5.3 クリップボード連携
  - Y キーでパスコピー
  - arboard + OSC 52
  - PR: `feat: Implement path copy to clipboard`

---

## Phase 6: Polish

- [ ] 6.1 エラーハンドリング
  - ユーザーフレンドリーなメッセージ
  - PR: `fix: Improve error messages`

- [ ] 6.2 README.md
  - インストール手順
  - 使用方法
  - myfileとの比較
  - PR: `docs: Add README`

- [ ] 6.3 テスト
  - file_tree テスト
  - file_ops テスト
  - PR: `test: Add unit tests`

---

## Progress Summary

| Phase | Items | Completed |
|-------|-------|-----------|
| 1. Foundation | 2 | 0 |
| 2. Core | 5 | 0 |
| 3. File Ops | 3 | 0 |
| 4. Preview | 3 | 0 |
| 5. Path Integration | 3 | 0 |
| 6. Polish | 3 | 0 |
| **Total** | **19** | **0** |

---

## myfileから流用するロジック

| ファイル | 流用内容 | 書き方の変更 |
|----------|----------|--------------|
| main.rs | イベントループ | 変数名変更 |
| file_tree.rs | フラット化、展開/折畳 | 構造体名変更 |
| app.rs | 状態管理 | フィールド名変更 |
| input.rs | キー/マウス処理 | 関数名変更 |
| ui.rs | 描画ロジック | - |
| file_ops.rs | CRUD操作 | - |

---

## fileview独自実装

| 機能 | 説明 |
|------|------|
| --pick | 選択パスを stdout 出力 |
| --on-select | 選択時にコールバック実行 |
| preview.rs | プレビュー処理を分離 |

---

## Notes

- Git統合（git_status.rs）は実装しない
- 外部コマンド実行（:）は --on-select で代替
- myfileのコードを参考に、書き方を変えて実装
