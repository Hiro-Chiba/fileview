#!/usr/bin/env python3
"""Opt-in real-file, real-terminal large preview check, with isolated HOME."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
from workspace_pty_smoke import App


def rss(pid):
    return int(subprocess.check_output(["ps", "-o", "rss=", "-p", str(pid)]).strip())


def run(binary, mib):
    assert 1 <= mib <= 1024
    with tempfile.TemporaryDirectory(prefix="fv-large-preview-") as temporary:
        base = Path(temporary)
        assert shutil.disk_usage(base).free > (mib + 512) * 1024 * 1024
        root = base / "project"
        root.mkdir()
        home = base / "home"
        home.mkdir()
        path = root / "large.log"
        line = b"ordinary log record".ljust(127, b" ") + b"\n"
        block = line * 8192
        with path.open("xb") as output:
            for _ in range(mib):
                output.write(block)
            output.seek(0)
            output.write(b"FIRST_LARGE_MARKER".ljust(127, b" ") + b"\n")
            output.seek(mib * 1024 * 1024 - 128)
            output.write(b"LAST_LARGE_MARKER".ljust(127, b" ") + b"\n")
            output.flush()
            os.fsync(output.fileno())
        app = App(binary, root, home)
        before = rss(app.pid)
        try:
            app.query("text:LAST_LARGE_MARKER")
            final_line = mib * 8192
            app.wait_for(lambda: f"large.log:{final_line}" in app.screen.results(), "large content result")
            after_search = rss(app.pid)
            app.send(b"\r")
            app.wait_for(lambda: "Fuzzy Find" not in app.screen.text() and "LAST_LARGE_MARKER" in app.screen.text(), "large matching preview")
            after_preview = rss(app.pid)
            app.send(b"g")
            app.wait_for(lambda: "FIRST_LARGE_MARKER" in app.screen.text(), "large first page")
            # Superseding an end scan must keep the first page visible.
            app.send(b"G")
            app.send(b"g")
            app.wait_for(lambda: "FIRST_LARGE_MARKER" in app.screen.text(), "superseding end scan")
            app.pump(1)
            assert "FIRST_LARGE_MARKER" in app.screen.text()
            after_cancel = rss(app.pid)
        finally:
            code = app.close()
        assert code == 0, code
        return dict(file_bytes=mib * 1024 * 1024, final_line=final_line,
                    tui_rss_kib=dict(before=before, after_search=after_search,
                                     after_preview=after_preview, after_cancel=after_cancel),
                    result="passed", scope="real release TUI, warm actual file; sampled RSS, not continuous peak or latency benchmark")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("binary", nargs="?", default="target/release/fv")
    parser.add_argument("--mib", type=int, default=100)
    args = parser.parse_args()
    print(json.dumps(run(str(Path(args.binary).resolve()), args.mib), indent=2))
