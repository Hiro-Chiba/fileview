#!/usr/bin/env python3
"""Opt-in Unix smoke test. Run after `cargo build --bin fv`.

Uses only Python's standard library and isolates all user data in a temporary
HOME. Exercises both normal input and a real stdin pipe through a controlling
PTY. The small screen reader handles the cursor operations emitted by ratatui.
"""

import argparse
import codecs
import fcntl
import os
from pathlib import Path
import pty
import re
import select
import signal
import struct
import tempfile
import termios
import time


class Screen:
    def __init__(self):
        self.rows = [[" "] * 120 for _ in range(40)]
        self.row = self.column = 0
        self.pending = ""
        self.decoder = codecs.getincrementaldecoder("utf-8")("replace")

    def feed(self, data):
        text = self.pending + self.decoder.decode(data)
        self.pending = ""
        offset = 0
        while offset < len(text):
            char = text[offset]
            if char == "\x1b":
                sequence = re.match(r"\x1b\[([0-9;?]*)([ -/]*)([@-~])", text[offset:])
                if sequence:
                    params, _, command = sequence.groups()
                    values = [int(value or 0) for value in params.lstrip("?").split(";")]
                    if command in "Hf":
                        self.row = (values[0] or 1) - 1
                        self.column = (values[1] or 1) - 1 if len(values) > 1 else 0
                    elif command == "J" and values[0] in (2, 3):
                        self.rows = [[" "] * 120 for _ in range(40)]
                    elif command == "K" and 0 <= self.row < 40:
                        start = self.column if values[0] == 0 else 0
                        end = self.column + 1 if values[0] == 1 else 120
                        self.rows[self.row][start:end] = [" "] * (end - start)
                    elif command == "G":
                        self.column = (values[0] or 1) - 1
                    elif command in "ABCD":
                        amount = values[0] or 1
                        self.row += amount * ((command == "B") - (command == "A"))
                        self.column += amount * ((command == "C") - (command == "D"))
                    offset += len(sequence.group())
                    continue
                # OSC titles and terminal controls that do not affect cell text.
                sequence = re.match(r"\x1b\].*?(?:\x07|\x1b\\)", text[offset:], re.S)
                if sequence:
                    offset += len(sequence.group())
                    continue
                self.pending = text[offset:]
                break
            offset += 1
            if char == "\r":
                self.column = 0
            elif char == "\n":
                self.row = min(39, self.row + 1)
            elif char >= " ":
                if 0 <= self.row < 40 and 0 <= self.column < 120:
                    self.rows[self.row][self.column] = char
                self.column += 1

    def text(self):
        return "\n".join("".join(row).rstrip() for row in self.rows)

    def results(self):
        # Popup result rows, excluding the input and the underlying tree.
        return "\n".join("".join(row[21:98]) for row in self.rows[10:25])


class App:
    def __init__(self, binary, root, home, stdin_paths=None):
        self.screen = Screen()
        self.pid, self.fd = pty.fork()
        if self.pid == 0:
            fcntl.ioctl(1, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))
            environment = os.environ.copy()
            environment.update(
                HOME=str(home), XDG_CONFIG_HOME=str(home / "config"),
                XDG_CACHE_HOME=str(home / "cache"), XDG_DATA_HOME=str(home / "data"),
                FILEVIEW_WORKSPACE_CACHE_DIR=str(home / "workspace-cache"),
                FILEVIEW_IMAGE_PROTOCOL="halfblocks", TERM="xterm-256color",
            )
            arguments = [str(binary), "--no-icons", str(root)]
            if stdin_paths is not None:
                reader, writer = os.pipe()
                os.write(writer, "".join(f"{path}\n" for path in stdin_paths).encode())
                os.close(writer)
                os.dup2(reader, 0)
                os.close(reader)
                arguments.append("--stdin")
            os.execve(binary, arguments, environment)
        self.wait_for(lambda: str(root.name) in self.screen.text(), "initial tree")

    def pump(self, seconds=0.1):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            if not select.select([self.fd], [], [], max(0, deadline - time.monotonic()))[0]:
                continue
            try:
                data = os.read(self.fd, 65536)
            except OSError:
                return
            if not data:
                return
            self.screen.feed(data)

    def send(self, keys):
        os.write(self.fd, keys)
        self.pump(0.15)

    def wait_for(self, predicate, description):
        deadline = time.monotonic() + 8
        while not predicate() and time.monotonic() < deadline:
            self.pump()
        if not predicate():
            raise AssertionError(f"Timed out waiting for {description}\n{self.screen.text()}")

    def query(self, text):
        self.send(b"\x10")
        self.wait_for(lambda: "Fuzzy Find" in self.screen.text(), "fuzzy popup")
        self.send(text.encode())

    def cancel(self):
        self.send(b"\x1b")
        self.wait_for(lambda: "Fuzzy Find" not in self.screen.text(), "popup closed")

    def close(self):
        try:
            self.send(b"\x1b")
            self.send(b"q")
        except OSError:
            pass
        self.pump(0.2)
        pid, status = os.waitpid(self.pid, os.WNOHANG)
        if not pid:
            os.kill(self.pid, signal.SIGTERM)
            _, status = os.waitpid(self.pid, 0)
        os.close(self.fd)
        return os.waitstatus_to_exitcode(status)


def smoke(binary):
    with tempfile.TemporaryDirectory(prefix="fv-workspace-pty-") as temporary:
        base = Path(temporary)
        root = base / "project"
        (root / "src").mkdir(parents=True)
        (root / "src/needle.rs").write_text("WORKSPACE_PREVIEW_SENTINEL\n")
        (root / "ordinary.txt").write_text("ordinary\n")
        (root / ".secret.rs").write_text("hidden\n")
        home = base / "home"
        home.mkdir()
        app = App(binary, root, home)
        try:
            app.query("needle ext:rs type:file")
            app.wait_for(lambda: "src/needle.rs" in app.screen.results(), "structured result")
            assert "ordinary.txt" not in app.screen.results()
            app.cancel()
            app.query("needle ext:rs")
            app.wait_for(lambda: "src/needle.rs" in app.screen.results(), "warm result")
            app.send(b"\r")
            assert "Fuzzy Find" not in app.screen.text()
            app.send(b"P")
            app.wait_for(lambda: "WORKSPACE_PREVIEW_SENTINEL" in app.screen.text(), "selected preview")
            app.query("type:invalid")
            app.wait_for(lambda: "Workspace:" in app.screen.text() and "No matches" in app.screen.results(), "invalid query error")
            assert "src/needle.rs" not in app.screen.results()
            app.cancel()
            app.query("fresh ext:rs")
            (root / "src/fresh.rs").write_text("live")
            app.wait_for(lambda: "src/fresh.rs" in app.screen.results(), "live update")
            app.cancel()
            app.send(b".")
            app.query("secret ext:rs")
            app.wait_for(lambda: ".secret.rs" in app.screen.results(), "hidden result")
            app.cancel()
            (root / "src/delete-a.rs").write_text("REMAINING_RESULT_SENTINEL\n")
            (root / "src/delete-b.rs").write_text("removed\n")
            app.query("delete- ext:rs")
            app.wait_for(lambda: "src/delete-b.rs" in app.screen.results(), "two deletion candidates")
            app.send(b"\x1b[B")
            (root / "src/delete-b.rs").unlink()
            app.wait_for(
                lambda: "src/delete-a.rs" in app.screen.results()
                and "src/delete-b.rs" not in app.screen.results(),
                "selected result removed",
            )
            app.send(b"\r")
            app.wait_for(lambda: "REMAINING_RESULT_SENTINEL" in app.screen.text(), "remaining result selected")
        finally:
            exit_code = app.close()
        assert exit_code == 0, f"Normal TUI exit code {exit_code}"
        print("PASS structured search, cancel/reopen, Enter/preview, invalid query, live updates, hidden toggle, result deletion")

        home = base / "stdin-home"
        home.mkdir()
        app = App(binary, root, home, [root / "ordinary.txt"])
        try:
            app.query("ordinary")
            app.wait_for(lambda: "ordinary.txt" in app.screen.results(), "stdin result")
            assert "needle.rs" not in app.screen.text()
            assert not (home / "workspace-cache").exists()
            app.cancel()
        finally:
            exit_code = app.close()
        assert exit_code == 0, f"Stdin TUI exit code {exit_code}"
        print("PASS real stdin pipe remains interactive, searches supplied paths, and creates no workspace cache")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path(__file__).resolve().parents[1] / "target/debug/fv")
    arguments = parser.parse_args()
    smoke(arguments.binary.resolve(strict=True))
