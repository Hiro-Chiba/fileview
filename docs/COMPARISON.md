# FileView vs Other Terminal File Managers

## At a Glance

```
Lightweight ◄─────────────────────────────────► Feature-rich

   nnn      lf    fileview    joshuto    ranger    yazi
    │        │        │          │          │        │
  3.4MB    12MB     8MB       5.4MB      28MB     38MB
```

## Feature Comparison

| Feature | fileview | yazi | lf | ranger | nnn |
|---------|:--------:|:----:|:--:|:------:|:---:|
| Zero config | **Yes** | No | No | No | Partial |
| Image preview | **Auto** | Yes | Script | Script | Plugin |
| Syntax highlighting | **Yes** | Yes | No | Yes | No |
| PDF preview | **Yes** | Yes | No | No | No |
| Archive preview | **Yes** | Yes | No | Yes | Plugin |
| Git integration | **Yes** | Yes | Script | Yes | No |
| Fuzzy finder | **Yes** | Yes | Yes | Yes | Plugin |
| Plugin system | No | Lua | Shell | Python | Shell |
| Config file | No | TOML | Config | rc | Env |
| Mouse support | **Yes** | Yes | No | Yes | No |
| Vim keybindings | **Yes** | Yes | Yes | Yes | Custom |

## When to Use FileView

**Choose fileview if you want:**
- Install and use immediately (no configuration)
- Image preview that "just works" (auto-detects terminal)
- Lightweight memory footprint (~8MB vs yazi's 38MB)
- Fast startup (2.3ms vs ranger's 400ms)
- Simple tool for quick file browsing

**Choose yazi if you want:**
- Maximum features and customization
- Lua scripting for automation
- Async operations for large directories
- Active community and plugins

**Choose lf if you want:**
- Minimal footprint with shell scripting
- Go-based single binary
- Maximum control via scripts

**Choose nnn if you want:**
- Absolute minimum resource usage (3.4MB)
- C-based, POSIX compliant
- Plugin-based extensibility

## Image Preview Comparison

| Terminal | fileview | yazi | lf | ranger |
|----------|:--------:|:----:|:--:|:------:|
| Kitty | Auto | Config | Script | Config |
| iTerm2 | Auto | Config | Script | Config |
| WezTerm | Auto | Config | Script | Config |
| Sixel | Auto | Config | Script | Config |
| Fallback | Halfblocks | Halfblocks | None | ASCII |

FileView automatically detects your terminal and uses the best available protocol.

## Performance Comparison

| Metric | fileview | yazi | lf | ranger | nnn |
|--------|----------|------|-----|--------|-----|
| Startup | 2.3ms | 15ms | 3ms | 400ms | 1.5ms |
| Memory | 8MB | 38MB | 12MB | 28MB | 3.4MB |
| Binary | 2.9MB | 4.5MB | 3.5MB | - | 0.1MB |

## Philosophy

FileView follows the Unix philosophy with a twist:

> "Do one thing well, but include batteries"

- **One thing**: Browse files in terminal
- **Batteries**: Image preview, Git status, syntax highlighting

Unlike traditional Unix tools that require external scripts for previews,
FileView includes everything needed for a modern file browsing experience
while staying lightweight.

## Migration from Other Tools

### From ranger
- Same vim keybindings (j/k/h/l)
- Preview works without configuration
- No Python dependency

### From lf
- Similar keybindings
- Built-in previews (no scripts needed)
- Built-in fuzzy finder

### From yazi
- Simpler, no configuration needed
- Lower memory usage
- Fewer features (intentional)
