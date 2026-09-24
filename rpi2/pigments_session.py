#!/usr/bin/python3
"""One supervised editor/preset session in the private Pigments copy."""
import argparse
import fcntl
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time


def main():
    root = Path(__file__).resolve().parent
    parser = argparse.ArgumentParser()
    parser.add_argument("label")
    parser.add_argument("--config", type=Path, default=root / "appliance.conf")
    parser.add_argument("--binary", type=Path, default=root.parent / "source-bridge/rpi0/standalone/target/release/lvb-arm-pigments-standalone")
    parser.add_argument("--phase-trace", choices=("off", "on"))
    arguments = parser.parse_args()
    label = arguments.label
    if not arguments.config.is_absolute() or not arguments.binary.is_absolute():
        raise ValueError("session paths must be absolute")
    if not re.fullmatch(r"[a-z0-9-]{1,32}", label):
        raise ValueError("invalid session label")
    lock = (root.parent / "run.lock").open("a")
    fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    active = subprocess.run(["systemctl", "--user", "is-active", "--quiet",
                             "lvb-rpi1-audio-gate.service"])
    if active.returncode == 0:
        raise RuntimeError("original Pigments is active")
    run = root / "logs" / label
    run.mkdir(mode=0o700)
    fifo = run / "commands.fifo"
    os.mkfifo(fifo, 0o600)
    descriptor = os.open(fifo, os.O_RDWR)
    unit = "lvb-rpi2-" + label + ".service"
    sessions = root / "compatdata/pfx/drive_c/bridge/sessions"
    before = set(sessions.iterdir())
    started = time.monotonic()
    report = {"label": label, "samples": [], "ready": False}
    log_path = run / "outer.log"
    with log_path.open("x") as output:
        process = subprocess.Popen([
            "systemd-run", "--user", "--wait", "--collect", "--pipe",
            "--service-type=exec", "--unit=" + unit,
            "--property=KillMode=control-group", "--property=TimeoutStopSec=15",
            "--property=RuntimeMaxSec=320", "--property=MemoryMax=1G",
            *(["--setenv=LVB_RPI1_PHASE_TRACE=" + arguments.phase_trace] if arguments.phase_trace else []),
            str(arguments.binary), str(arguments.config)], stdin=descriptor, stdout=output,
            stderr=subprocess.STDOUT)
        try:
            while process.poll() is None:
                elapsed = time.monotonic() - started
                temperature = int(Path("/sys/class/thermal/thermal_zone0/temp").read_text()) / 1000
                flags = int(subprocess.check_output(["vcgencmd", "get_throttled"], text=True)
                            .strip().split("=")[1], 16)
                report["samples"].append({"seconds": elapsed, "temperature_c": temperature,
                                          "throttled": flags})
                text = log_path.read_text()
                if not report["ready"] and "RPI1_READY " in text:
                    report["ready"], report["ready_seconds"] = True, elapsed
                    print("PIGMENTS_READY commands=" + str(fifo), flush=True)
                if temperature >= 75 or flags & 15 or elapsed >= 280:
                    report["stop_reason"] = "thermal/power bound" if temperature >= 75 or flags & 15 else "session time bound"
                    os.write(descriptor, b"status\nquit\n")
                    process.wait(timeout=25)
                    break
                time.sleep(1)
            report["exit_code"] = process.wait(timeout=1)
            report["clean_shutdown"] = "RPI1_CLEAN_SHUTDOWN " in log_path.read_text()
        finally:
            if process.poll() is None:
                subprocess.run(["systemctl", "--user", "stop", unit], timeout=25)
                process.wait(timeout=10)
            added = set(sessions.iterdir()) - before
            report["sessions_retained"] = sorted(p.name for p in added)
            for directory in added:
                if re.fullmatch(r"[0-9a-f]{32}", directory.name):
                    child = "lvb-rpi1-" + directory.name + ".service"
                    subprocess.run(["systemctl", "--user", "stop", child],
                                   timeout=25, capture_output=True)
                    journal = subprocess.run(["journalctl", "--user-unit", child,
                        "--output=cat", "--no-pager"], timeout=20, capture_output=True)
                    (run / "windows.log").write_bytes(journal.stdout)
            os.close(descriptor)
            fifo.unlink()
            report["elapsed_seconds"] = time.monotonic() - started
            report["jack_graph_after"] = subprocess.check_output(["jack_lsp", "-c"], text=True)
            (run / "run.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({k: v for k, v in report.items() if k != "samples"}), flush=True)


if __name__ == "__main__":
    main()
