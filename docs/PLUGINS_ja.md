# Lua プラグインシステム

FileView は Lua プラグインによる機能拡張をサポートしています。

## クイックスタート

`~/.config/fileview/plugins/init.lua` にプラグインファイルを作成:

```lua
fv.notify("プラグインからこんにちは!")
```

FileView を再起動すると通知が表示されます。

## プラグインの場所

```
~/.config/fileview/plugins/init.lua
```

このファイルは起動時に自動的に読み込まれます。

## API リファレンス

### 読み取り専用関数

| 関数 | 戻り値 | 説明 |
|------|--------|------|
| `fv.current_file()` | `string` または `nil` | 現在フォーカスしているファイルパス |
| `fv.current_dir()` | `string` | 現在のディレクトリパス |
| `fv.selected_files()` | `table` | 選択されたファイルパスの配列 |
| `fv.version()` | `string` | FileView のバージョン（例: "1.22.0"） |
| `fv.is_dir(path)` | `boolean` | パスがディレクトリかどうか |
| `fv.file_exists(path)` | `boolean` | パスが存在するかどうか |

### アクション関数

| 関数 | 説明 |
|------|------|
| `fv.notify(message)` | ステータスバーに通知を表示 |
| `fv.navigate(path)` | ディレクトリに移動 |
| `fv.select(path)` | ファイルを選択に追加 |
| `fv.deselect(path)` | ファイルを選択から除外 |
| `fv.clear_selection()` | すべての選択を解除 |
| `fv.refresh()` | ツリービューを再読み込み |
| `fv.set_clipboard(text)` | クリップボードにテキストを設定 |
| `fv.focus(path)` | 特定のファイルにフォーカス（表示して選択） |

### 登録関数

| 関数 | 説明 |
|------|------|
| `fv.register_command(name, fn)` | カスタムコマンドを登録 |
| `fv.on(event, fn)` | イベントハンドラを登録 |
| `fv.register_previewer(pattern, fn)` | カスタムプレビューアを登録 |

## イベント一覧

| イベント | 引数 | 説明 |
|----------|------|------|
| `start` | なし | プラグイン読み込み後 |
| `file_selected` | `path` | フォーカスファイルが変わった時 |
| `directory_changed` | `path` | 新しいディレクトリに移動した時 |
| `selection_changed` | なし | 複数選択が変わった時 |
| `before_quit` | なし | アプリケーション終了前 |

## 使用例

### ファイルパスをクリップボードにコピー

```lua
fv.register_command("copy-path", function()
    local file = fv.current_file()
    if file then
        fv.set_clipboard(file)
        fv.notify("コピー: " .. file)
    else
        fv.notify("ファイルが選択されていません")
    end
end)
```

### 秘密ファイルの警告

```lua
fv.on("file_selected", function(path)
    if path and path:match("%.env$") then
        fv.notify("警告: 環境変数ファイル!")
    end
end)
```

### 起動時にホームに移動

```lua
fv.on("start", function()
    local home = os.getenv("HOME")
    if home then
        fv.navigate(home)
    end
end)
```

### カスタム CSV プレビューア

```lua
fv.register_previewer("*.csv", function(path)
    local file = io.open(path, "r")
    if not file then
        return "ファイルを読み込めません"
    end

    local lines = {}
    local count = 0
    for line in file:lines() do
        table.insert(lines, line)
        count = count + 1
        if count >= 50 then break end
    end
    file:close()

    return table.concat(lines, "\n")
end)
```

### 拡張子でファイルを選択

```lua
fv.register_command("select-rs", function()
    local dir = fv.current_dir()
    -- 注意: これは簡単な例です。実際の実装では
    -- ディレクトリ内容を走査する必要があります。
    fv.notify(dir .. " で *.rs ファイルを選択")
end)
```

## プレビューアの Glob パターン

`register_previewer` 関数は glob パターンをサポートしています:

| パターン | マッチ |
|----------|--------|
| `*.txt` | `file.txt`, `readme.txt` |
| `*.??` | `file.rs`, `test.py` |
| `test*` | `test.txt`, `testing.md` |
| `*test*` | `my_test_file`, `testing` |

## 注意事項

- プラグインはサンドボックス化された Lua 5.4 環境で実行されます
- プラグイン内のエラーはキャッチされ、通知として表示されます
- 同じイベントに対して複数のハンドラを登録できます
- プレビューアはプレビューパネルに表示される文字列を返します
