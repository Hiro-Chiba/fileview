# FileView - Design Document

## 1. Overview

FileViewは、ターミナルエミュレーター上で動作するVSCode風のミニマルファイルツリーUIである。
Ghostty等のモダンターミナルでの使用を想定し、**軽量・高速・直感操作**を設計思想の中核とする。

### 1.1 myfileとの関係

本プロジェクトは [myfile](../myfile) の設計思想を参考にしている。
ただし、**構造・命名・モジュール分割は独自に設計**し、別プロジェクトとして成立させる。

| 観点 | myfile | fileview |
|------|--------|----------|
| 参考にする | アルゴリズム、ロジックの考え方 | ← |
| 変える | - | 構造、命名、モジュール分割 |
| 追加する | - | パス連携機能 |
| 削除する | Git統合、外部コマンド | - |

---

## 2. Structural Differences（構造の違い）

### 2.1 ディレクトリ構成

**myfile（フラット構造）:**
```
src/
├── main.rs
├── app.rs
├── file_tree.rs
├── file_ops.rs
├── git_status.rs
├── ui.rs
└── input.rs
```

**fileview（モジュール構造）:**
```
src/
├── main.rs
├── lib.rs
├── core/
│   ├── mod.rs
│   ├── state.rs        # AppState（myfile: App）
│   └── mode.rs         # ViewMode（myfile: InputMode）
├── tree/
│   ├── mod.rs
│   ├── node.rs         # TreeEntry（myfile: FileNode）
│   └── navigator.rs    # TreeNavigator（myfile: FileTree）
├── action/
│   ├── mod.rs
│   ├── file.rs         # ファイル操作
│   └── clipboard.rs    # クリップボード
├── render/
│   ├── mod.rs
│   ├── tree.rs         # ツリー描画
│   ├── preview.rs      # プレビュー描画
│   └── status.rs       # ステータスバー
├── handler/
│   ├── mod.rs
│   ├── key.rs          # キーイベント
│   └── mouse.rs        # マウスイベント
└── integrate/
    ├── mod.rs
    ├── pick.rs         # --pick モード
    └── callback.rs     # --on-select
```

### 2.2 命名規則の違い

| 概念 | myfile | fileview | 理由 |
|------|--------|----------|------|
| アプリ状態 | `App` | `AppState` | 状態であることを明示 |
| 入力モード | `InputMode` | `ViewMode` | 入力だけでなくビュー状態も含む |
| ツリーノード | `FileNode` | `TreeEntry` | ファイル以外も想定（将来の拡張性） |
| ツリー本体 | `FileTree` | `TreeNavigator` | ナビゲーション機能を強調 |
| フラット化リスト | `flat_list` | `visible_entries` | 可視エントリであることを明示 |
| 選択位置 | `selected` | `focus_index` | フォーカスの概念 |
| スクロール位置 | `scroll_offset` | `viewport_top` | ビューポートの上端 |
| マーク済み | `marked` | `selected_paths` | 選択（複数）とフォーカス（単一）を区別 |
| プレビュー表示 | `quick_preview_enabled` | `preview_visible` | 簡潔に |

### 2.3 モード定義の違い

**myfile:**
```rust
pub enum InputMode {
    Normal,
    Search,
    Rename,
    NewFile,
    NewDir,
    Confirm(ConfirmAction),
    Preview,
    ExternalCommand,
}
```

**fileview:**
```rust
pub enum ViewMode {
    Browse,                          // 通常ブラウズ（myfile: Normal）
    Search { query: String },        // 検索（状態を内包）
    Input { purpose: InputPurpose }, // 入力（Rename/NewFile/NewDir統合）
    Confirm { action: PendingAction }, // 確認
    Preview { scroll: usize },       // プレビュー（スクロール状態を内包）
}

pub enum InputPurpose {
    CreateFile,
    CreateDir,
    Rename { original: PathBuf },
}

pub enum PendingAction {
    Delete { targets: Vec<PathBuf> },
}
```

**違い:**
- 状態をenum variantに内包（myfileは別フィールド）
- ExternalCommandを削除（--on-selectで代替）
- Input系を統合

---

## 3. Algorithm Reference（参考にするロジック）

以下のロジックはmyfileの**考え方**を参考にするが、**コードは独自に書き直す**。

### 3.1 ツリーのフラット化

**考え方（myfileと同じ）:**
- 再帰的にノードを走査
- 展開されたノードの子のみリストに追加
- インデックスでO(1)アクセス

**fileviewでの実装:**
```rust
impl TreeNavigator {
    /// ツリーを可視エントリのリストに変換
    pub fn flatten(&self) -> Vec<&TreeEntry> {
        let mut entries = Vec::new();
        self.collect_visible(&self.root, &mut entries);
        entries
    }

    fn collect_visible<'a>(&'a self, entry: &'a TreeEntry, out: &mut Vec<&'a TreeEntry>) {
        out.push(entry);
        if entry.is_expanded() {
            for child in entry.children() {
                self.collect_visible(child, out);
            }
        }
    }
}
```

### 3.2 スクロール自動調整

**考え方（myfileと同じ）:**
- フォーカスが画面外に出たらスクロール
- 上に出たら上にスクロール、下に出たら下にスクロール

**fileviewでの実装:**
```rust
impl AppState {
    pub fn adjust_viewport(&mut self, visible_height: usize) {
        if self.focus_index < self.viewport_top {
            self.viewport_top = self.focus_index;
        } else if self.focus_index >= self.viewport_top + visible_height {
            self.viewport_top = self.focus_index - visible_height + 1;
        }
    }
}
```

### 3.3 ドラッグ&ドロップ検出

**考え方（myfileと同じ）:**
- 高速な文字入力はD&Dの可能性
- タイムアウトでバッファを確定
- パス形式ならD&Dとして処理

**fileviewでの実装:**
```rust
pub struct DropDetector {
    buffer: String,
    last_input: Instant,
}

impl DropDetector {
    const CHAR_TIMEOUT_MS: u128 = 50;
    const CONFIRM_TIMEOUT_MS: u128 = 100;

    pub fn feed(&mut self, c: char) {
        let now = Instant::now();
        if now.duration_since(self.last_input).as_millis() > Self::CHAR_TIMEOUT_MS {
            self.buffer.clear();
        }
        self.buffer.push(c);
        self.last_input = now;
    }

    pub fn check(&mut self) -> Option<PathBuf> {
        if Instant::now().duration_since(self.last_input).as_millis() < Self::CONFIRM_TIMEOUT_MS {
            return None;
        }
        let path = self.buffer.trim();
        self.buffer.clear();
        if path.starts_with('/') && Path::new(path).exists() {
            Some(PathBuf::from(path))
        } else {
            None
        }
    }
}
```

### 3.4 画像の半ブロック描画

**考え方（myfileと同じ）:**
- 1文字で縦2ピクセル表現（▀）
- 上ピクセル=前景色、下ピクセル=背景色
- アスペクト比を保持してリサイズ

**fileviewでの実装:**
```rust
pub fn render_image(img: &DynamicImage, width: u32, height: u32) -> Vec<Line<'static>> {
    let resized = img.resize(width, height * 2, FilterType::Triangle);
    let mut lines = Vec::new();

    for row in 0..(height as usize) {
        let mut spans = Vec::new();
        for col in 0..(width as usize) {
            let top = resized.get_pixel(col as u32, (row * 2) as u32);
            let bottom = resized.get_pixel(col as u32, (row * 2 + 1) as u32);

            spans.push(Span::styled(
                "▀",
                Style::default()
                    .fg(Color::Rgb(top[0], top[1], top[2]))
                    .bg(Color::Rgb(bottom[0], bottom[1], bottom[2])),
            ));
        }
        lines.push(Line::from(spans));
    }
    lines
}
```

---

## 4. Original Features（fileview独自機能）

### 4.1 --pick モード

```bash
# 選択したパスを取得
selected=$(fileview --pick)

# ディレクトリに移動
cd "$(fileview --pick)"
```

### 4.2 --on-select コールバック

```bash
# エディタで開く
fileview --on-select "nvim {path}"

# 複数選択してアーカイブ
fileview --on-select "tar -cvf archive.tar {paths}"
```

### 4.3 終了コード

| Code | 意味 |
|------|------|
| 0 | パス選択あり |
| 1 | キャンセル |
| 2 | エラー |

---

## 5. Removed Features（削除する機能）

| 機能 | myfile | fileview | 理由 |
|------|--------|----------|------|
| Git統合 | あり | **なし** | lazygit等に任せる |
| 外部コマンド（:） | あり | **なし** | --on-selectで代替 |
| コマンド履歴 | あり | **なし** | シンプルさ優先 |

---

## 6. Key Bindings

| Key | Action |
|-----|--------|
| `j` / `↓` | 下移動 |
| `k` / `↑` | 上移動 |
| `l` / `→` / `Enter` | 展開 / 確定 |
| `h` / `←` | 折りたたみ / 親へ |
| `g` | 先頭へ |
| `G` | 末尾へ |
| `Space` | 選択切替（マルチセレクト） |
| `y` | コピー |
| `d` | カット |
| `p` | ペースト |
| `D` | 削除 |
| `r` | リネーム |
| `a` | 新規ファイル |
| `A` | 新規フォルダ |
| `/` | 検索 |
| `Y` | パスをクリップボードへ |
| `P` | プレビュー切替 |
| `o` | フルスクリーンプレビュー |
| `.` | 隠しファイル切替 |
| `q` | 終了 |

---

## 7. Technology Stack

| Category | Choice |
|----------|--------|
| Language | Rust |
| TUI | ratatui |
| Terminal | crossterm |
| Clipboard | arboard |
| Image | image |
| Error | anyhow |

---

## 8. Summary: myfile vs fileview

| Aspect | myfile | fileview |
|--------|--------|----------|
| 構造 | フラット（6ファイル） | モジュール階層（6ディレクトリ） |
| 命名 | 独自 | VSCode寄り |
| モード管理 | 別フィールド | enum内包 |
| Git | あり | なし |
| 外部コマンド | : キー | --on-select |
| パス連携 | なし | --pick |

**fileviewはmyfileのロジックを参考にしつつ、構造・命名を独自設計した別プロジェクトである。**
