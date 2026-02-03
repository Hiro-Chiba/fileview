# FileView vs Other Terminal File Managers

## Score Comparison (Objective)

| Category | fileview | yazi | lf | nnn | ranger |
|----------|:--------:|:----:|:--:|:---:|:------:|
| **Ease of Setup** | 10 | 6 | 4 | 7 | 5 |
| **Image Preview** | 9 | 10 | 5 | 6 | 7 |
| **Plugin/Extensibility** | 6 | 10 | 8 | 8 | 9 |
| **Feature Richness** | 7 | 10 | 6 | 7 | 8 |
| **Startup Speed** | 9 | 8 | 9 | 10 | 3 |
| **Memory Efficiency** | 8 | 5 | 8 | 10 | 5 |
| **Large Directory Handling** | 7 | 10 | 8 | 9 | 4 |
| **Docs/Community** | 4 | 9 | 7 | 8 | 10 |
| **Customizability** | 6 | 10 | 9 | 7 | 9 |
| **Stability/Maturity** | 6 | 7 | 9 | 10 | 10 |
| **AI Integration** | 10 | 0 | 0 | 0 | 0 |
| **Total** | **82** | **85** | **73** | **82** | **70** |

## At a Glance

```
Lightweight ◄─────────────────────────────────► Feature-rich

   nnn    fileview    lf    joshuto    ranger    yazi
    │        │        │        │          │        │
  0.1MB    3.4MB    3.5MB    5.4MB      28MB     4.5MB
```

## Feature Comparison

| Feature | fileview | yazi | lf | ranger | nnn |
|---------|:--------:|:----:|:--:|:------:|:---:|
| Zero config | **Yes** | Partial | No | No | Partial |
| Image preview | **Auto** | Yes | Script | Script | Plugin |
| Video preview | **Yes** | Yes | No | No | No |
| Syntax highlighting | **Yes** | Yes | No | Yes | No |
| PDF preview | **Yes** | Yes | No | No | No |
| Archive preview | **Yes** | Yes | No | Yes | Plugin |
| Git integration | **Yes** | Yes | Script | Yes | No |
| Fuzzy finder | **Yes** | Yes | Yes | Yes | Plugin |
| Tab support | **Yes** | Yes | No | Yes | Yes |
| Plugin system | Lua | Lua | Shell | Python | Shell |
| Async I/O | Partial | **Full** | Partial | No | No |
| Config file | Optional | TOML | Config | rc | Env |
| Mouse support | **Yes** | Yes | No | Yes | No |
| Vim keybindings | **Yes** | Yes | Yes | Yes | Custom |
| Adaptive UI | **Yes** | No | No | No | No |
| MCP Server | **Yes** | No | No | No | No |
| Claude Code format | **Yes** | No | No | No | No |
| Tree output | **Yes** | No | No | No | No |

## Strengths and Weaknesses

### FileView

**Strengths:**
- Zero configuration required (works out of the box)
- Auto-detects terminal image protocol (Kitty/iTerm2/Sixel/etc.)
- Compact 3.4MB binary (including Lua support)
- Fast startup (2.3ms)
- Adaptive status bar for narrow terminals
- **AI-first design**: MCP server, Claude Code integration, tree output for context

**Weaknesses:**
- Small community (single developer)
- Immature plugin ecosystem
- Partial async I/O (yazi has full async)
- Limited documentation

### Yazi

**Strengths:**
- Full async I/O (UI never blocks, even with large directories)
- Mature Lua plugin ecosystem with package manager
- Active community and extensive documentation
- Type definitions for plugin development

**Weaknesses:**
- Requires configuration for optimal experience
- Higher memory usage (~38MB)
- Still in heavy development (breaking changes)

### nnn

**Strengths:**
- Extremely lightweight (0.1MB binary, 3.4MB memory)
- 10+ years of stability and maturity
- POSIX compliant, runs everywhere
- Environment variable based (no config file)

**Weaknesses:**
- No built-in image preview
- Custom keybindings (not vim-like by default)
- Requires plugins for modern features

### lf

**Strengths:**
- Single Go binary, no dependencies
- Server/client architecture (copy between terminals)
- Highly customizable via shell scripts
- Stable and mature

**Weaknesses:**
- Requires extensive scripting for previews
- No built-in image preview
- No mouse support

### ranger

**Strengths:**
- Most mature and well-documented
- Large community and plugin ecosystem
- Python-based (easy to extend)

**Weaknesses:**
- Slow startup (400ms)
- High memory usage (28MB)
- Python dependency required

## Recommendation by Use Case

| Use Case | Recommended |
|----------|-------------|
| No setup, just works | **fileview** |
| AI pair programming (Claude Code) | **fileview** |
| Maximum features & customization | **yazi** |
| Extreme lightweight | **nnn** |
| Shell script extensibility | **lf** |
| Stability & documentation | **ranger** / **nnn** |
| Narrow terminal (80x24) | **fileview** |

## Performance Comparison

| Metric | fileview | yazi | lf | ranger | nnn |
|--------|----------|------|-----|--------|-----|
| Startup | 2.3ms | 15ms | 3ms | 400ms | 1.5ms |
| Memory | 8MB | 38MB | 12MB | 28MB | 3.4MB |
| Binary | 3.4MB | 4.5MB | 3.5MB | - | 0.1MB |

## Image Preview Comparison

| Terminal | fileview | yazi | lf | ranger |
|----------|:--------:|:----:|:--:|:------:|
| Kitty | Auto | Config | Script | Config |
| iTerm2 | Auto | Config | Script | Config |
| WezTerm | Auto | Config | Script | Config |
| Sixel | Auto | Config | Script | Config |
| Fallback | Halfblocks | Halfblocks | None | ASCII |

FileView automatically detects your terminal and uses the best available protocol.

## AI Integration Comparison

| Feature | fileview | yazi | lf | ranger | nnn |
|---------|:--------:|:----:|:--:|:------:|:---:|
| MCP Server | **Yes** | No | No | No | No |
| Tree output (`--tree`) | **Yes** | No | No | No | No |
| Claude format copy | **Yes** | No | No | No | No |
| Interactive select mode | **Yes** | No | No | No | No |
| Content with paths | **Yes** | No | No | No | No |

FileView is designed for AI pair programming workflows where:
- AI assistants need codebase context (tree structure)
- Users select files for AI analysis
- Claude Code directly browses via MCP

## Honest Assessment

FileView positions itself as a "balanced" option, but is squeezed between:
- **yazi**: Superior in features, async I/O, and ecosystem
- **nnn**: Superior in lightweight and stability

**Unique differentiator: AI Integration**

FileView is the only terminal file manager with built-in AI tooling support:
- **MCP Server**: Claude Code can directly browse and read files
- **Claude-friendly output**: `Ctrl+Y` copies with syntax hints
- **Tree output**: `--tree` for AI context without interaction
- **Select mode**: `--select-mode` for AI-driven file selection

This makes FileView the **de facto choice for AI pair programming workflows**.

FileView is best suited for users who:
1. Use AI coding assistants (Claude Code, Cursor, etc.)
2. Want something that works immediately without reading documentation
3. Use narrow terminals (Claude Code, tmux splits)
4. Value simplicity over maximum features

## Migration from Other Tools

### From ranger
- Same vim keybindings (j/k/h/l)
- Preview works without configuration
- No Python dependency
- Much faster startup

### From lf
- Similar keybindings
- Built-in previews (no scripts needed)
- Built-in fuzzy finder

### From yazi
- Simpler, no configuration needed
- Lower memory usage
- Fewer features (intentional)
- Better narrow screen support

## References

- [yazi](https://github.com/sxyazi/yazi) - Blazing fast terminal file manager
- [lf](https://github.com/gokcehan/lf) - Terminal file manager
- [nnn](https://github.com/jarun/nnn) - The unorthodox terminal file manager
- [ranger](https://github.com/ranger/ranger) - Console file manager with VI key bindings
