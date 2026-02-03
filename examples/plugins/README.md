# FileView Plugin Examples

Example Lua plugins for FileView. Copy to `~/.config/fileview/plugins/` to use.

## Available Plugins

### claude_code.lua
Integration with Claude Code and AI assistants.
- `claude-ref`: Copy file path for Claude reference
- `claude-context`: Copy marked files as context list

### git_actions.lua
Quick git operations from FileView.
- `git-add`: Stage current file
- `git-reset`: Unstage current file
- `git-diff`: Show diff for current file
- `git-log`: Show log for current file
- `git-add-marked`: Stage all marked files

### productivity.lua
General productivity commands.
- `edit`: Open in $EDITOR
- `code`: Open in VS Code
- `yank-path`: Copy full path
- `yank-name`: Copy file name only
- `info`: Show file info

## Installation

```bash
mkdir -p ~/.config/fileview/plugins
cp examples/plugins/*.lua ~/.config/fileview/plugins/
```

## Usage

Run commands with `:command-name` in FileView.

Example:
```
:git-add       # Stage current file
:claude-ref    # Copy path for Claude
```

## Writing Your Own

See [docs/PLUGINS.md](../../docs/PLUGINS.md) for the full API reference.
