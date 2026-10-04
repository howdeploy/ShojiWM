"""Run a compiled Rust test binary without inheriting the desktop's IPC namespace."""
import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile

if len(sys.argv) < 2:
    raise SystemExit("usage: python3 tests/run-native-tests.py TEST_BINARY [TEST_ARGUMENTS...]")

with tempfile.TemporaryDirectory(prefix="shoji-tests-") as directory:
    environment = os.environ.copy()
    environment.update(XDG_RUNTIME_DIR=directory, WAYLAND_DISPLAY="test")
    for key in ("DISPLAY", "SHOJI_RUNTIME_WAKE_PID"):
        environment.pop(key, None)
    # A live sentinel detects accidental removal of the inherited socket even
    # inside this private namespace. No connection to the desktop is made.
    path = Path(directory) / "shojiwm-test.sock"
    with socket.socket(socket.AF_UNIX) as sentinel:
        sentinel.bind(str(path))
        sentinel.listen()
        inode = path.stat().st_ino
        result = subprocess.run(sys.argv[1:], env=environment)
        assert path.stat().st_ino == inode, "tests replaced the inherited IPC socket"
        print("Isolated IPC sentinel preserved", flush=True)
    raise SystemExit(result.returncode)
