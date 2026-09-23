#!/usr/bin/python3
"""Run the existing vendor editor path without the standalone audio/state gate."""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time
import uuid


def main():
    root = Path(__file__).resolve().parent
    label = sys.argv[1]
    if not re.fullmatch(r"editor-ge-[0-9]{2}", label):
        raise ValueError("invalid label")
    locks = []
    for path in (root.parent / "run.lock", Path.home() / "rpi1-private/operation.lock"):
        lock = path.open("a")
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        locks.append(lock)
    if subprocess.run(["systemctl", "--user", "is-active", "--quiet",
                       "lvb-rpi1-audio-gate.service"]).returncode == 0:
        raise RuntimeError("original Pigments is active")
    config = dict(line.split("=", 1) for line in (root / "appliance.conf").read_text().splitlines())
    for key in ("launcher", "leader", "windows_host", "plugin", "xauthority"):
        if hashlib.sha256(Path(config[key + "_path"]).read_bytes()).hexdigest() != config[key + "_sha256"]:
            raise RuntimeError(key + " changed")
    run = root / "logs" / label
    run.mkdir(mode=0o700)
    prefix = root / "compatdata/pfx"
    sid = uuid.uuid4().hex
    directory = prefix / "drive_c/bridge/sessions" / sid
    directory.mkdir(mode=0o700)
    win = lambda path: "C:\\" + str(Path(path).relative_to(prefix / "drive_c")).replace("/", "\\")
    mode, case = "ap12-vendor-access", "class:" + config["pigments_class_id"].lower()
    pairs = [("session", sid), ("scanner-sha256", config["windows_host_sha256"]),
             ("module-sha256", config["plugin_sha256"]), ("bundle-manifest-sha256", config["plugin_sha256"]),
             ("implementation-source-manifest-sha256", config["source_manifest_sha256"]),
             ("mode", mode), ("component-case", case)]
    binding = ("schema=linux-vst-bridge-wf0-handshake/v1\n" +
               "".join(k.replace("-", "_") + "=" + v + "\n" for k, v in pairs) + "run_ordinal=1\n").encode()
    pairs += [("module", win(config["plugin_path"])), ("ready", win(directory / (sid + ".ready"))),
              ("gate", win(directory / (sid + ".gate"))), ("max-classes", "256"), ("stdout-cap", "1048576")]
    env = {"WINEPREFIX": str(prefix), "DISPLAY": config["display"], "XAUTHORITY": config["xauthority_path"],
           "LANG": "C.UTF-8", "WINEDLLOVERRIDES": "uiautomationcore=",
           "LVB_EVENT_OUTPUT_POLICY": config["event_output_policy"],
           "LVB_EDITOR_LIFETIME": config["editor_lifetime_policy"],
           "LVB_VENDOR_RETIREMENT": config["vendor_retirement_policy"]}
    unit = "lvb-rpi2-" + label + ".service"
    command = ["systemd-run", "--user", "--wait", "--collect", "--pipe", "--service-type=exec",
               "--unit=" + unit, "--property=KillMode=control-group", "--property=TimeoutStopSec=15",
               "--property=RuntimeMaxSec=300", "--property=MemoryMax=3G", "--property=TasksMax=512"]
    command += ["--setenv=" + key + "=" + value for key, value in env.items()]
    command += [config["launcher_path"], win(config["windows_host_path"])]
    arguments = dict(pairs)
    # The installed host parses a fixed order, even though every value is named.
    for key in ("session", "scanner-sha256", "implementation-source-manifest-sha256",
                "module", "module-sha256", "bundle-manifest-sha256", "ready", "gate",
                "max-classes", "stdout-cap", "mode", "component-case"):
        command += ["--" + key, arguments[key]]
    report = {"session": sid, "label": label, "samples": [], "gate_sent": False, "editor_open_record": False}
    started, opened, stop_sent = time.monotonic(), None, None
    log_path = run / "windows.log"
    with log_path.open("x") as output:
        process = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=output, stderr=subprocess.STDOUT)
        try:
            while process.poll() is None:
                elapsed = time.monotonic() - started
                temp = int(Path("/sys/class/thermal/thermal_zone0/temp").read_text()) / 1000
                flags = int(subprocess.check_output(["vcgencmd", "get_throttled"], text=True).strip().split("=")[1], 16)
                report["samples"].append({"seconds": elapsed, "temperature_c": temp, "throttled": flags})
                ready = directory / (sid + ".ready")
                if not report["gate_sent"] and ready.exists():
                    if ready.read_bytes() != binding:
                        raise RuntimeError("host handshake mismatch")
                    with (directory / (sid + ".gate")).open("xb") as gate:
                        gate.write(binding)
                    report["gate_sent"] = True
                text = log_path.read_text()
                if opened is None and '"state":"ap12_vendor_access_open"' in text:
                    opened = elapsed
                    report["editor_open_record"], report["editor_open_seconds"] = True, elapsed
                    print("EDITOR_ATTACHED " + str(round(elapsed, 2)), flush=True)
                reason = ("thermal/power bound" if temp >= 75 or flags & 15 else
                          "host failure" if '"state":"ap8_failure"' in text else
                          "editor observation complete" if opened is not None and elapsed - opened >= 15 else
                          "session time bound" if elapsed >= 240 else None)
                if reason and stop_sent is None:
                    report["stop_reason"] = reason
                    (directory / "vendor.stop").write_text(sid + "\n")
                    stop_sent = elapsed
                if stop_sent is not None and elapsed - stop_sent >= 20:
                    break
                time.sleep(1)
        except Exception as error:
            report["error"] = str(error)
            raise
        finally:
            if process.poll() is None:
                subprocess.run(["systemctl", "--user", "stop", unit], timeout=25, capture_output=True)
            report["exit_code"] = process.wait(timeout=10)
            report["elapsed_seconds"] = time.monotonic() - started
            report["unit_active_after"] = subprocess.run(["systemctl", "--user", "is-active", "--quiet", unit]).returncode == 0
            report["jack_graph_after"] = subprocess.check_output(["jack_lsp", "-c"], text=True)
            (run / "run.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({k: v for k, v in report.items() if k != "samples"}), flush=True)


if __name__ == "__main__":
    main()
