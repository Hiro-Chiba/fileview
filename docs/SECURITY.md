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
the Lua standard libraries except `debug`, including `os.execute`, `io.popen`,
and `require` for Lua files, and can run commands, touch any accessible file,
and use the network through those commands. `init.lua` autoloads
on startup, so only install plugin code you trust. See `docs/PLUGINS.md` for
details and how to disable the plugin runtime at build time.

## MCP Server

`fv --mcp-server <root>` speaks JSON-RPC over stdio (no network listener, no
authentication; the trust boundary is whoever can spawn the process). It is
intended for a local AI client.

- File read/write/delete and listing tools confine paths to `<root>` by
  canonicalizing and rejecting anything that resolves outside it.
- `write_file` and `delete_file` additionally refuse sensitive paths. This
  covers everything inside a `.git` directory, including the directory itself,
  as well as `.env` and credential files such as `.ssh` keys. The check uses the
  normalized path relative to `<root>` after resolving symlinked parent
  directories, so spellings such as `.git//hooks` or a symlink into `.git` are
  refused too. Windows name forms that resolve to `.git`, such as `.git.` or
  `GIT~1`, are treated the same way. Versions before 3.1.1 checked the raw argument and could be
  bypassed this way.
- `run_build`, `run_test`, `run_lint`, and `get_diagnostics` execute the
  project's own toolchain (build scripts, test code, lint plugins), and
  `run_build` also accepts a command from a fixed allowlist that includes `npm`,
  `make`, and `pip`. `stage_files` and `create_commit` run `git add` and
  `git commit`, which run the repository's Git hooks and filters. Exposing these
  tools to an AI client is equivalent to allowing local code execution within the
  project, so only point the server at repositories you trust.

## --on-select Callback

The `--on-select` option executes shell commands. Security considerations:

- Commands run through `sh -c` with your permissions
- `{path}`, `{dir}`, `{name}`, `{stem}`, and `{ext}` are replaced with values
  escaped using single-quote wrapping
- Do NOT use with untrusted command strings
- Equivalent to running commands manually in terminal

### Safe Usage

```bash
fv --pick --on-select "code {path}"  # Open in editor
fv --pick --on-select "cat {path}"   # Display file
```

### Unsafe (Avoid)

```bash
fv --on-select "$UNTRUSTED_VAR {path}"   # Never use untrusted input
```

## Reporting Vulnerabilities

Report security issues privately through
[GitHub Security Advisories](https://github.com/Hiro-Chiba/fileview/security/advisories/new).
Fixes are released for the latest version only, so please update before
reporting.
