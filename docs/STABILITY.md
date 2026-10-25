# Stability Policy

FileView ships stable releases only. The last alpha was 2.0.4-alpha, and every
release since 2.0 follows [Semantic Versioning](https://semver.org/).

- **Patch** (`x.y.Z`) fixes bugs and security issues. A setting or binding that
  was documented but never worked may be removed in a patch release, because no
  one can depend on it. Files that still mention it keep loading.
- **Minor** (`x.Y.0`) adds features without breaking existing flags, keys, or
  configuration.
- **Major** (`X.0.0`) may change or remove behavior that worked before, and the
  changelog explains how to upgrade.

Only the latest release receives fixes, including security fixes.

## Release Gate

A release is tagged only from a commit on `main` where:

- the required `CI Success` check passed, covering formatting, clippy with and
  without default features, rustdoc, and tests on Ubuntu, macOS, and Windows
- the post-merge Security Audit passed under the policy in `.cargo/audit.toml`,
  where each ignored advisory is documented
- the minimum supported Rust version, 1.90.0, still builds every target
