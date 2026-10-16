#!/usr/bin/env python3
"""Run a temporary browser desktop viewer without host mouse/keyboard capture."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import secrets
import shlex
import signal
import shutil
import socket
import subprocess
import sys
import tarfile
import tempfile
import time
import urllib.request

SOURCE = Path(__file__).resolve().parent
NOVNC = "https://codeload.github.com/novnc/noVNC/tar.gz/refs/tags/v1.6.0"
NOVNC_SHA256 = "5066103959ef4e9b10f37e5a148627360dd8414e4cf8a7db92bdbd022e728aaa"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("ssh_target", help="SSH destination for the Deck, for example deck@steamdeck")
    parser.add_argument("--port", type=int, default=18080, help="Local browser port")
    parser.add_argument("--vnc-port", type=int, default=15900, help="Private forwarding port")
    parser.add_argument("--minutes", type=int, choices=range(1, 121), default=30, metavar="1..120")
    options = parser.parse_args()
    def interrupted(_number, _frame):
        raise KeyboardInterrupt
    signal.signal(signal.SIGTERM, interrupted)
    if options.ssh_target.startswith("-") or any(c.isspace() for c in options.ssh_target):
        parser.error("Use one SSH destination without command-line options")
    if not all(1024 <= port <= 65535 for port in (options.port, options.vnc_port)) or options.port == options.vnc_port:
        parser.error("Use two different unprivileged TCP ports")
    for tool in ("ssh", "zig"):
        if not shutil.which(tool):
            parser.error(tool + " is required on the operator's computer")
    for port in (options.port, options.vnc_port):
        with socket.socket() as probe:
            probe.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            probe.bind(("127.0.0.1", port))
    ssh = ["ssh", "-o", "BatchMode=yes", "-o", "ConnectTimeout=10", options.ssh_target]
    remote_code = (SOURCE / "remote.py").read_text()

    def remote(operation, settings=None):
        result = subprocess.run(ssh + ["python3 -c " + shlex.quote(remote_code) + " " + operation],
                                input=json.dumps(settings or {}), text=True, capture_output=True, timeout=180)
        if result.returncode:
            raise RuntimeError(result.stderr.strip() or "Remote helper failed")
        return json.loads(result.stdout)

    work = Path(tempfile.mkdtemp(prefix="lpb-deck-viewer-"))
    remote_work = None
    children = []
    try:
        subprocess.run([sys.executable, "-m", "venv", str(work / "venv")], check=True)
        python = work / "venv/bin/python"
        subprocess.run([str(python), "-m", "pip", "install", "--quiet", "websockify==0.13.0"], check=True)
        archive = work / "novnc.tar.gz"
        with urllib.request.urlopen(NOVNC, timeout=30) as response, archive.open("wb") as out:
            shutil.copyfileobj(response, out)
        if hashlib.sha256(archive.read_bytes()).hexdigest() != NOVNC_SHA256:
            raise RuntimeError("noVNC archive checksum differs")
        with tarfile.open(archive) as source:
            source.extractall(work, filter="data")
        shutil.copyfile(SOURCE / "viewer.html", work / "noVNC-1.6.0/index.html")
        subprocess.run(["zig", "cc", "-target", "x86_64-linux-gnu.2.31", "-shared", "-fPIC", "-O2",
                        str(SOURCE / "loopback.c"), "-o", str(work / "loopback.so"), "-ldl"], check=True)
        remote_work = remote("prepare")["work"]
        put = "from pathlib import Path;import sys;Path(" + repr(remote_work + "/loopback.so") + ").write_bytes(sys.stdin.buffer.read())"
        subprocess.run(ssh + ["python3 -c " + shlex.quote(put)], input=(work / "loopback.so").read_bytes(), check=True)
        password = secrets.token_urlsafe(12)[:8]
        (work / "credentials.json").write_text(json.dumps({"password": password}))
        (work / "credentials.json").chmod(0o600)
        remote("start", {"work": remote_work, "password": password, "port": options.vnc_port,
                         "seconds": options.minutes * 60})
        # Confirm actual binds. Krfb's native default is wildcard, so the wrapper is required.
        command = "ss -H -ltn 'sport = :" + str(options.vnc_port) + "'"
        deadline = time.monotonic() + 10
        while True:
            result = subprocess.run(ssh + [command], capture_output=True, text=True, check=True)
            addresses = [line.split()[3] for line in result.stdout.splitlines()]
            expected = {"127.0.0.1:" + str(options.vnc_port), "[::1]:" + str(options.vnc_port)}
            if addresses and set(addresses) <= expected:
                break
            if addresses or time.monotonic() > deadline:
                raise RuntimeError("Sharing did not establish only loopback listeners")
            time.sleep(.1)
        tunnel = subprocess.Popen(["ssh", "-N", "-o", "BatchMode=yes", "-o", "ExitOnForwardFailure=yes",
                                   "-o", "ServerAliveInterval=10", "-o", "ServerAliveCountMax=2", "-L",
                                   f"127.0.0.1:{options.vnc_port}:127.0.0.1:{options.vnc_port}", options.ssh_target])
        children.append(tunnel)
        with (work / "gateway.log").open("w") as log:
            gateway = subprocess.Popen([str(python), str(SOURCE / "gateway.py"), str(work), "--port", str(options.port),
                                        "--vnc-port", str(options.vnc_port)], stdout=log, stderr=log)
        children.append(gateway)
        print(f"Open http://127.0.0.1:{options.port}/ (starts view-only).", flush=True)
        print("If KDE requests sharing, the authorized agent can inspect and operate it with consent.py. Ctrl-C stops this session and removes its temporary files.", flush=True)
        deadline = time.monotonic() + options.minutes * 60
        while time.monotonic() < deadline:
            if any(child.poll() is not None for child in children):
                raise RuntimeError("Sharing transport stopped")
            time.sleep(.5)
    finally:
        # Keep the tunnel alive while browser sockets close and release their modifiers.
        for child in reversed(children):
            child.terminate()
            try:
                child.wait(timeout=3)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait()
        if remote_work:
            try:
                remote("stop", {"work": remote_work})
            except Exception:
                print("Remote cleanup could not be confirmed; the temporary server has a time limit.", file=sys.stderr)
        shutil.rmtree(work)


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        pass
    except Exception as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(1)
