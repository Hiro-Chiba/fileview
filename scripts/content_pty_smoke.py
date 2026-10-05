#!/usr/bin/env python3
"""Actual Unix TUI content navigation and bounded paging, isolated HOME."""
import argparse
from pathlib import Path
import tempfile
from workspace_pty_smoke import App


def smoke(binary):
    with tempfile.TemporaryDirectory(prefix="fv-content-pty-") as temporary:
        base = Path(temporary)
        root = base / "project"
        root.mkdir()
        home = base / "home"
        home.mkdir()
        lines = [f"ROW_{i:05d} ordinary text" for i in range(1, 10001)]
        lines[0] = "FIRST_PAGE_MARKER"
        lines[399] = "CONTENT_MATCH_SENTINEL"
        lines[-1] = "LAST_PAGE_MARKER"
        (root / "ordinary.txt").write_text("OTHER_FILE_SENTINEL\n")
        with (root / "giant.log").open("wb") as output:
            for _ in range(16):
                output.write(b"x" * (1024 * 1024))
            output.write(b"LONG_LINE_MATCH_SENTINEL")
        (root / "sample.log").write_text("\n".join(lines) + "\n")
        app = App(binary, root, home)
        try:
            app.send(b"F*.txt\r")
            app.wait_for(lambda: "Filter:" in app.screen.text(), "tree filter")
            app.query("text:CONTENT_MATCH_SENTINEL")
            app.wait_for(lambda: "sample.log:400" in app.screen.results(), "content line result")
            app.send(b"\r")
            app.wait_for(lambda: "Fuzzy Find" not in app.screen.text() and "400" in app.screen.text() and "CONTENT_MATCH_SENTINEL" in app.screen.text(), "matching preview line")
            app.send(b"G")
            app.wait_for(lambda: "LAST_PAGE_MARKER" in app.screen.text() and "10000" in app.screen.text(), "bounded last page")
            app.send(b"g")
            app.wait_for(lambda: "FIRST_PAGE_MARKER" in app.screen.text(), "first page")
            for _ in range(14):
                app.send(b"f")
            app.wait_for(lambda: "ROW_00281" in app.screen.text(), "forward page boundary")
            for _ in range(14):
                app.send(b"b")
            app.wait_for(lambda: "FIRST_PAGE_MARKER" in app.screen.text(), "backward page boundary")
            app.send(b"\x1b")
            app.query("text:DOES_NOT_EXIST")
            app.send(b"\x7f" * len("DOES_NOT_EXIST") + b"FIRST_PAGE_MARKER")
            app.wait_for(lambda: "sample.log:1 " in app.screen.results(), "newest query result")
            assert "sample.log:400" not in app.screen.results()
            app.cancel()
            app.query("text:LONG_LINE_MATCH_SENTINEL")
            app.wait_for(lambda: "giant.log:1 " in app.screen.results(), "16MiB single-line search")
            app.send(b"\r")
            app.wait_for(lambda: "Fuzzy Find" not in app.screen.text() and "LONG_LINE_MATCH_SENTINEL" in app.screen.text(), "16MiB single-line preview")
        finally:
            code = app.close()
        assert code == 0, code
    print("Content TUI search, matching line, paging, query replacement and 16MiB single line passed")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("binary", nargs="?", default="target/debug/fv")
    args = parser.parse_args()
    smoke(str(Path(args.binary).resolve()))
