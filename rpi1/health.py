#!/usr/bin/env python3
"""Bounded, one-Hz observer outside the RPI1 translated cohort. No recovery actions."""
import json
import os
from pathlib import Path
import re
import subprocess
import time


def read(path):
    try:
        return Path(path).read_text().strip()
    except OSError:
        return None


def firmware(*args):
    try:
        return subprocess.check_output(
            ["/usr/bin/vcgencmd", *args], timeout=0.5, text=True,
            stderr=subprocess.DEVNULL).strip()
    except (OSError, subprocess.SubprocessError):
        return None


def sample():
    memory = dict(line.split(":", 1) for line in Path("/proc/meminfo").read_text().splitlines())
    scopes = {}
    root = Path(f"/sys/fs/cgroup/user.slice/user-{os.getuid()}.slice/user@{os.getuid()}.service/app.slice")
    for path in root.glob("lvb-rpi1-*.service"):
        if path.name == "lvb-rpi1-audio-gate.service" or re.fullmatch(r"lvb-rpi1-[0-9a-f]{32}\.service", path.name):
            scopes[path.name] = {key: read(path / key) for key in
                                ("memory.current", "memory.peak", "cpu.stat", "pids.current")}
            scopes[path.name]["processes"] = sum(
                len((read(p) or "").splitlines()) for p in path.rglob("cgroup.procs"))
    return {
        "monotonic_seconds": round(time.monotonic(), 3),
        "temperature_millidegrees": read("/sys/class/thermal/thermal_zone0/temp"),
        "throttled": firmware("get_throttled"),
        "voltage": firmware("pmic_read_adc", "EXT5V_V"),
        "cpu_khz": read("/sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq"),
        "mem_available": memory.get("MemAvailable", "").strip(),
        "swap_total": memory.get("SwapTotal", "").strip(),
        "swap_free": memory.get("SwapFree", "").strip(),
        "memory_pressure": read("/proc/pressure/memory"),
        "io_pressure": read("/proc/pressure/io"),
        "system_processes": sum(1 for p in Path("/proc").iterdir() if p.name.isdecimal()),
        "cohorts": scopes,
    }


if __name__ == "__main__":
    start = time.monotonic()
    for index in range(900):
        print("RPI1_HEALTH " + json.dumps(sample(), separators=(",", ":")), flush=True)
        time.sleep(max(0, start + index + 1 - time.monotonic()))
