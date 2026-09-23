#!/usr/bin/python3.13
"""One bounded reference-VST comparison run against already staged runtimes."""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def systemctl(*arguments):
    return subprocess.run(["systemctl", "--user", *arguments], capture_output=True, text=True, timeout=15)


def group(unit):
    value = systemctl("show", unit, "-p", "ControlGroup", "--value").stdout.strip()
    if not value.endswith("/" + unit):
        raise RuntimeError("owned cgroup unavailable")
    return Path("/sys/fs/cgroup") / value.lstrip("/")


def cpu(path):
    return dict((key, int(value)) for key, value in
                (line.split() for line in (path / "cpu.stat").read_text().splitlines()))


def temperature():
    return int(Path("/sys/class/thermal/thermal_zone0/temp").read_text()) / 1000


def threads(path):
    result = {}
    for file in path.rglob("cgroup.procs"):
        for pid in file.read_text().split():
            proc = Path("/proc") / pid
            try:
                if "LVB ARM Appliance Synth.vst3" not in (proc / "maps").read_text():
                    continue
                for task in (proc / "task").iterdir():
                    stat = (task / "stat").read_text().rsplit(")", 1)[1].split()
                    sched = [int(x) for x in (task / "schedstat").read_text().split()]
                    result[task.name] = {"name": (task / "comm").read_text().strip(),
                                         "start": stat[19], "runtime_ns": sched[0],
                                         "runqueue_ns": sched[1], "slices": sched[2]}
            except FileNotFoundError:
                continue
    return result


def main():
    lane, label = sys.argv[1:]
    if lane not in ("ge", "box64") or not re.fullmatch(r"[a-z0-9-]{1,32}", label):
        raise RuntimeError("invalid comparison selection")
    root = Path(__file__).resolve().parent
    lock = (root / "run.lock").open("a")
    fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    if systemctl("is-active", "lvb-rpi1-audio-gate.service").returncode == 0:
        raise RuntimeError("RPI1 audio is active")
    if temperature() > 56:
        raise RuntimeError("comparison start above 56 C; cool before this run")
    run = root / "runs" / label
    run.mkdir()
    prefix = root / ("prefix-ge-01" if lane == "ge" else "box64-reference/compatdata/pfx")
    if not (prefix / "system.reg").is_file():
        raise RuntimeError("prepare the reference prefix before comparison")
    ge_prefix = root / "prefix-ge-01"
    for relative in ("probes/wf0-factory-probe.exe", "probes/rpi0-windows-probe.exe",
                     "fixtures/LVB ARM Appliance Synth.vst3"):
        source = ge_prefix / "drive_c/bridge" / relative
        target = prefix / "drive_c/bridge" / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        if source != target:
            shutil.copyfile(source, target)
    session_root = prefix / "drive_c/bridge/sessions"
    session_root.mkdir(exist_ok=True)
    before = set(session_root.iterdir())
    base = dict(line.split("=", 1) for line in (root / "runs/bridge-01/appliance.conf").read_text().splitlines())
    base.update(prefix_path=str(prefix), evidence_path=str(run / "native.jsonl"),
                windows_evidence_path=str(run / "windows.jsonl"))
    paths = {"launcher": root / f"compare-{lane}.py", "leader": Path("/usr/bin/python3.13"),
             "windows_host": prefix / "drive_c/bridge/probes/wf0-factory-probe.exe",
             "windows_probe": prefix / "drive_c/bridge/probes/rpi0-windows-probe.exe",
             "plugin": prefix / "drive_c/bridge/fixtures/LVB ARM Appliance Synth.vst3"}
    for key, path in paths.items():
        base[key + "_path"], base[key + "_sha256"] = str(path), sha(path)
    config = run / "appliance.conf"
    config.write_text("".join(f"{key}={value}\n" for key, value in base.items()))
    binary = root / "source-bridge/rpi0/standalone/target/release/lvb-arm-standalone"
    if sha(binary) != "35146bf051aeba36f67ae72cfe2c41ac87a01c9717a83f1163576e29ab44683b":
        raise RuntimeError("native bridge changed")
    unit = "lvb-rpi2-" + label + ".service"
    common = ["systemd-run", "--user", "--wait", "--collect", "--pipe", "--service-type=exec",
              "--property=KillMode=control-group", "--property=TimeoutStopSec=10"]
    started = time.monotonic()
    windows_unit = None
    fixture = None
    report = {"lane": lane, "label": label, "start_temperature_c": temperature(),
              "samples": [], "native_binary_sha256": sha(binary)}
    log_path = run / "outer.log"
    with log_path.open("x") as log:
        process = subprocess.Popen(common + ["--unit=" + unit, "--property=RuntimeMaxSec=85",
            "--property=MemoryMax=1G", str(binary), "run", str(config)],
            stdin=subprocess.PIPE, stdout=log, stderr=subprocess.STDOUT, text=True)
        try:
            while "RPI0_READY " not in log_path.read_text():
                if process.poll() is not None or time.monotonic() - started > 35:
                    raise RuntimeError("bridge did not become ready")
                time.sleep(.1)
            sessions = set(session_root.iterdir()) - before
            if len(sessions) != 1:
                raise RuntimeError("new session identity ambiguous")
            session = sessions.pop()
            windows_unit = "lvb-rpi0-" + session.name + ".service"
            report.update(session=session.name, startup_seconds=time.monotonic()-started)
            for name in ("rpi0.arch.native", "rpi0.arch.windows"):
                shutil.copyfile(session / name, run / name)
            groups = {"native": group(unit), "windows": group(windows_unit)}
            begin_threads = threads(groups["windows"])
            begin = {key: cpu(path) for key, path in groups.items()}
            measured_start = time.monotonic()
            with (run / "polyphony.log").open("x") as audio_log:
                fixture = subprocess.Popen(common + ["--unit=lvb-rpi2-notes-"+label+".service",
                    "--property=RuntimeMaxSec=42", "--property=MemoryMax=128M",
                    "--setenv=LVB_QUALIFICATION_CLIENT=lvb-arm-standalone", str(root / "qualification"),
                    "polyphony", "--capture", str(run / "audio.f32le")],
                    stdout=audio_log, stderr=subprocess.STDOUT)
                while fixture.poll() is None:
                    sample = {"seconds": time.monotonic()-measured_start, "temperature_c": temperature(),
                        "clock_khz": int(Path("/sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq").read_text()),
                        "throttled": int(subprocess.check_output(["vcgencmd", "get_throttled"], text=True).strip().split("=")[1],16),
                        "cpu": {key: cpu(path) for key,path in groups.items()}}
                    report["samples"].append(sample)
                    if sample["temperature_c"] >= 75 or sample["throttled"] & 0xf:
                        raise RuntimeError("thermal/power condition invalidates comparison")
                    if sample["seconds"] > 42:
                        raise RuntimeError("audio fixture timeout")
                    time.sleep(1)
                report["fixture_exit"] = fixture.returncode
                if fixture.returncode != 0:
                    raise RuntimeError("audio fixture failed")
            report["measured_seconds"] = time.monotonic()-measured_start
            report["cpu_usage_usec"] = {key: cpu(path)["usage_usec"]-begin[key]["usage_usec"] for key,path in groups.items()}
            end_threads = threads(groups["windows"])
            report["host_thread_deltas"] = []
            for tid, end in end_threads.items():
                first = begin_threads.get(tid)
                if first and first["start"] == end["start"]:
                    report["host_thread_deltas"].append({"tid": tid, "name": end["name"],
                        **{key: end[key]-first[key] for key in ("runtime_ns", "runqueue_ns", "slices")}})
            process.stdin.write("status\nquit\n")
            process.stdin.flush()
            report["bridge_exit"] = process.wait(timeout=20)
            report["session_removed"] = not session.exists()
            if report["bridge_exit"] != 0 or not report["session_removed"]:
                raise RuntimeError("bridge shutdown failed")
            report["result"] = "passed"
        except Exception as error:
            report["result"], report["error"] = "failed", str(error)
            raise
        finally:
            if fixture is not None and fixture.poll() is None:
                systemctl("stop", "lvb-rpi2-notes-" + label + ".service")
            if process.poll() is None:
                systemctl("stop", unit)
                process.wait(timeout=15)
            # Cover early startup failures before Windows unit discovery.
            for session in set(session_root.iterdir())-before:
                if re.fullmatch(r"[0-9a-f]{32}",session.name):
                    systemctl("stop", "lvb-rpi0-"+session.name+".service")
            report["elapsed_seconds"] = time.monotonic()-started
            report["end_temperature_c"] = temperature()
            report["jack_graph_after"] = subprocess.check_output(["jack_lsp", "-c"], text=True)
            (run / "run.json").write_text(json.dumps(report, indent=2)+"\n")
    print(json.dumps({key:report[key] for key in ("lane","label","result","startup_seconds","measured_seconds","cpu_usage_usec","start_temperature_c","end_temperature_c")}),flush=True)


if __name__ == "__main__":
    main()
