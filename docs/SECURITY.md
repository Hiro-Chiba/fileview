# Security

## Overview

fileview (fv) is a file browser that runs with the user's permissions.

## Security Model

- Runs as the current user with their file permissions
- No privilege escalation
- File operations affect the user's own filesystem
- The core browser performs no network operations on its own. However, two
  opt-in features can run arbitrary local code and reach the network: Lua
  plugins (see below) and the MCP server's build/test/lint tools. Both execute
  with your full permissions.

## Lua Plugins

Plugins are trusted user code and are **not** sandboxed. A loaded plugin has
the full Lua standard library (`os.execute`, `io.*`, `require`, …) and can run
commands, touch any accessible file, and use the network. `init.lua` autoloads
on startup, so only install plugin code you trust. See `docs/PLUGINS.md` for
details and how to disable the plugin runtime at build time.

## MCP Server

`fv --mcp-server <root>` speaks JSON-RPC over stdio (no network listener, no
authentication; the trust boundary is whoever can spawn the process). It is
intended for a local AI client.

- File read/write/delete and listing tools confine paths to `<root>` by
  canonicalizing and rejecting anything that resolves outside it.
- `write_file` and `delete_file` additionally refuse sensitive paths such as
  `.git/hooks`, `.git/config`, `.env`, and credential files.
- `run_build`, `run_test`, `run_lint`, and `get_diagnostics` execute the
  project's own toolchain (build scripts, test code, lint plugins). Exposing
  these to an AI client is equivalent to allowing local code execution within
  the project, so only point the server at repositories you trust.

## --on-select Callback

The `--on-select` option executes shell commands. Security considerations:

- Commands run with your shell and permissions
- Paths are escaped using single-quote wrapping
- Do NOT use with untrusted command strings
- Equivalent to running commands manually in terminal

### Safe Usage

```bash
fv --pick --on-select "code {}"      # Open in editor
fv --pick --on-select "cat {}"       # Display file
```

### Unsafe (Avoid)

```bash
fv --on-select "$UNTRUSTED_VAR {}"   # Never use untrusted input
```

## Reporting Vulnerabilities

Report security issues via GitHub Security Advisories or email.
