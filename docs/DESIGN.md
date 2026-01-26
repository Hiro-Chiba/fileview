# FileView - Design Document

## 1. Overview

FileViewは、ターミナルエミュレーター上で動作するVSCode風のミニマルファイルツリーUIである。
Ghostty等のモダンターミナルでの使用を想定し、**軽量・高速・直感操作**を設計思想の中核とする。

### 1.1 設計目標

- **シンプル**: 必要最小限の機能に絞る
- **高速**: 大きなディレクトリでもスムーズに動作
- **直感的**: Vimライクなキーバインドで効率的な操作
- **連携性**: 外部ツールとの連携を重視（--pick, --on-select）

---

## 2. Architecture（アーキテクチャ）

### 2.1 ディレクトリ構成

```
src/
├── main.rs
├── lib.rs
├── core/
│   ├── mod.rs
│   ├── state.rs        # AppState - アプリケーション状態
│   └── mode.rs         # ViewMode - ビューモード定義
├── tree/
│   ├── mod.rs
│   ├── node.rs         # TreeEntry - ツリーノード
│   └── navigator.rs    # TreeNavigator - ツリー操作
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

### 2.2 モジュール責務

| モジュール | 責務 |
|-----------|------|
| `core` | アプリケーション状態とモード管理 |
| `tree` | ファイルツリーのデータ構造と操作 |
| `action` | ファイル操作とクリップボード |
| `render` | UI描画 |
| `handler` | イベント処理 |
| `integrate` | 外部ツール連携 |

### 2.3 モード定義

```rust
pub enum ViewMode {
    Browse,                            // 通常ブラウズ
    Search { query: String },          // 検索（状態を内包）
    Input { purpose: InputPurpose },   // 入力
    Confirm { action: PendingAction }, // 確認
    Preview { scroll: usize },         // プレビュー（スクロール状態を内包）
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

**設計ポイント:**
- 状態をenum variantに内包することで、状態管理を型安全に
- モードごとに必要なデータを明示

---

## 3. Core Algorithms（コアアルゴリズム）

### 3.1 ツリーのフラット化

ツリー構造を画面表示用のフラットリストに変換する。

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

フォーカスが画面外に出た場合、自動的にスクロール位置を調整する。

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

ターミナルではD&Dイベントが直接取得できないため、高速な文字入力パターンから推測する。

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

Unicode半ブロック文字（▀）を使用して、1文字で縦2ピクセルを表現する。

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

## 4. Integration Features（連携機能）

### 4.1 --pick モード

選択したパスを標準出力に出力し、シェルスクリプトから利用可能にする。

```bash
# 選択したパスを取得
selected=$(fv --pick)

# ディレクトリに移動
cd "$(fv --pick)"
```

### 4.2 --on-select コールバック

ファイル選択時に指定したコマンドを実行する。

```bash
# エディタで開く
fv --on-select "nvim {path}"

# ファイル情報を表示
fv --on-select "file {path}"
```

### 4.3 終了コード

| Code | 意味 |
|------|------|
| 0 | パス選択あり |
| 1 | キャンセル |
| 2 | エラー |

---

## 5. Key Bindings

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
| `c` | パスをクリップボードへ |
| `P` | プレビュー切替 |
| `o` | フルスクリーンプレビュー |
| `.` | 隠しファイル切替 |
| `q` | 終了 |

---

## 6. Technology Stack

| Category | Choice |
|----------|--------|
| Language | Rust |
| TUI | ratatui |
| Terminal | crossterm |
| Clipboard | arboard |
| Image | image |
| Error | anyhow |

---

## 7. Design Principles

1. **シンプルさを保つ**: 機能追加より既存機能の洗練を優先
2. **型安全性**: Rustの型システムを活用した安全な設計
3. **モジュール性**: 責務を明確に分離し、テスト容易性を確保
4. **外部連携**: スタンドアロンよりも他ツールとの連携を重視
