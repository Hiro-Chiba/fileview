# Architecture

## Overview

```
┌─────────┐     ┌─────────┐     ┌──────────┐
│  main   │────▶│   lib   │────▶│  utils   │
└─────────┘     └─────────┘     └──────────┘
                     │
                     ▼
                ┌─────────┐
                │ Config  │
                └─────────┘
```

## Modules

### `main.rs`
Entry point. Initializes config and runs the pipeline.

### `lib.rs`
Core library with `Config` struct and public API.

### `utils/helper.rs`
Utility functions for file operations and formatting.

## Design Decisions

- **Serde for serialization**: Chosen for its ecosystem support
- **Tokio runtime**: Async-ready for future network features
- **Minimal dependencies**: Keep the binary small
