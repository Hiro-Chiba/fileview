# Security

## Overview

fileview (fv) is a file browser that runs with the user's permissions.

## Security Model

- Runs as the current user with their file permissions
- No privilege escalation or network operations
- File operations affect user's own filesystem

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
