#!/usr/bin/env python3
"""Bounded, one-Hz observer outside the RPI1 translated cohort. No recovery actions."""
import json
import argparse
import os
from pathlib import Path
import re
import subprocess
import time

DISPLAY_NAMES = {"Xorg", "Xwayland", "Xtigervnc", "Xvnc", "x11vnc", "wayvnc"}
MAX_THREADS = 256


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


def cooling():
    return [{"name": path.name, "type": read(path / "type"),
             "current": read(path / "cur_state"), "max": read(path / "max_state")}
            for path in sorted(Path("/sys/class/thermal").glob("cooling_device*"))]


def cpu_policies():
    root = Path("/sys/devices/system/cpu/cpufreq")
    return [{"policy": path.name, "affected_cpus": read(path / "affected_cpus"),
             "governor": read(path / "scaling_governor"),
             "current_khz": read(path / "scaling_cur_freq")}
            for path in sorted(root.glob("policy*"))]


def scope_pids(root):
    pids = set()
    for path in root.glob("lvb-rpi1-*.service"):
        if path.name == "lvb-rpi1-audio-gate.service" or re.fullmatch(
                r"lvb-rpi1-[0-9a-f]{32}\.service", path.name):
            for listing in path.rglob("cgroup.procs"):
                pids.update(int(value) for value in (read(listing) or "").splitlines()
                            if value.isdecimal())
    return pids


def thread_sample(pid, tid, role, process_name):
    task = Path(f"/proc/{pid}/task/{tid}")
    stat = read(task / "stat")
    sched = read(task / "schedstat")
    status = read(task / "status")
    if not stat or not sched or not status:
        return None
    fields = stat.rsplit(") ", 1)[-1].split()
    numbers = sched.split()
    if len(fields) < 39 or len(numbers) < 3:
        return None
    values = dict(line.split(":", 1) for line in status.splitlines() if ":" in line)
    return {
        "pid": pid, "tid": tid, "role": role, "process": process_name,
        "thread": read(task / "comm"), "start_ticks": int(fields[19]),
        "runtime_ns": int(numbers[0]), "runqueue_wait_ns": int(numbers[1]),
        "timeslices": int(numbers[2]), "minor_faults": int(fields[7]),
        "major_faults": int(fields[9]), "priority": int(fields[15]),
        "policy": int(fields[38]), "cpu": int(fields[36]),
        "voluntary_switches": int(values.get("voluntary_ctxt_switches", "0")),
        "involuntary_switches": int(values.get("nonvoluntary_ctxt_switches", "0")),
        "allowed_cpus": values.get("Cpus_allowed_list", "").strip(),
    }


def threads(root, previous):
    owned = scope_pids(root)
    choices = [(pid, "rpi1_cohort") for pid in sorted(owned)]
    for proc in Path("/proc").iterdir():
        if not proc.name.isdecimal() or int(proc.name) in owned:
            continue
        name = read(proc / "comm")
        if name in DISPLAY_NAMES or name == "wineserver":
            choices.append((int(proc.name), "display" if name in DISPLAY_NAMES else "wineserver"))
    choices.append((os.getpid(), "health_observer"))
    output, current = [], {}
    omitted = 0
    for pid, role in choices:
        process_name = read(f"/proc/{pid}/comm")
        if process_name is None:
            continue
        try:
            tids = sorted(int(path.name) for path in Path(f"/proc/{pid}/task").iterdir()
                          if path.name.isdecimal())
        except OSError:
            continue
        for tid in tids:
            if len(output) >= MAX_THREADS:
                omitted += 1
                continue
            try:
                value = thread_sample(pid, tid, role, process_name)
            except (OSError, ValueError):
                continue
            if value is None:
                continue
            key = (pid, tid, value["start_ticks"])
            earlier = previous.get(key)
            if earlier:
                for field in ("runtime_ns", "runqueue_wait_ns", "timeslices",
                              "minor_faults", "major_faults", "voluntary_switches",
                              "involuntary_switches"):
                    value[field + "_delta"] = max(0, value[field] - earlier[field])
            current[key] = value
            output.append(value)
    return output, current, omitted


def sample(previous=None):
    memory = dict(line.split(":", 1) for line in Path("/proc/meminfo").read_text().splitlines())
    scopes = {}
    root = Path(f"/sys/fs/cgroup/user.slice/user-{os.getuid()}.slice/user@{os.getuid()}.service/app.slice")
    for path in root.glob("lvb-rpi1-*.service"):
        if path.name == "lvb-rpi1-audio-gate.service" or re.fullmatch(r"lvb-rpi1-[0-9a-f]{32}\.service", path.name):
            scopes[path.name] = {key: read(path / key) for key in
                                ("memory.current", "memory.peak", "cpu.stat", "pids.current")}
            scopes[path.name]["processes"] = sum(
                len((read(p) or "").splitlines()) for p in path.rglob("cgroup.procs"))
    thread_rows, current, omitted = threads(root, previous or {})
    result = {
        "monotonic_seconds": round(time.monotonic(), 3),
        "temperature_millidegrees": read("/sys/class/thermal/thermal_zone0/temp"),
        "throttled": firmware("get_throttled"),
        "voltage": firmware("pmic_read_adc", "EXT5V_V"),
        "cpu_khz": read("/sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq"),
        "arm_clock": firmware("measure_clock", "arm"),
        "v3d_clock": firmware("measure_clock", "v3d"),
        "cooling": cooling(),
        "cpu_policies": cpu_policies(),
        "mem_available": memory.get("MemAvailable", "").strip(),
        "swap_total": memory.get("SwapTotal", "").strip(),
        "swap_free": memory.get("SwapFree", "").strip(),
        "memory_pressure": read("/proc/pressure/memory"),
        "cpu_pressure": read("/proc/pressure/cpu"),
        "io_pressure": read("/proc/pressure/io"),
        "system_processes": sum(1 for p in Path("/proc").iterdir() if p.name.isdecimal()),
        "cohorts": scopes,
        "thread_count": len(thread_rows),
        "thread_records": thread_rows,
        "threads_omitted": omitted,
    }
    return result, current


def phase_mtime(directory):
    if directory is None:
        return 0
    latest = 0
    for path in Path(directory).glob("*-phase.jsonl"):
        try:
            latest = max(latest, path.stat().st_mtime_ns)
        except OSError:
            pass
    return latest


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--phase-directory", type=Path,
                        help="private evidence directory; changes trigger a five-second sampling burst")
    arguments = parser.parse_args()
    deadline = time.monotonic() + 900
    previous = {}
    last_flags = None
    last_phase = phase_mtime(arguments.phase_directory)
    burst_until = 0.
    while time.monotonic() < deadline:
        started = time.monotonic()
        observation, previous = sample(previous)
        thread_records = observation.pop("thread_records")
        flags = observation["throttled"]
        updated_phase = phase_mtime(arguments.phase_directory)
        if (last_flags is not None and flags != last_flags) or updated_phase != last_phase:
            burst_until = started + 5
            print("RPI1_HEALTH_EVENT " + json.dumps({
                "monotonic_seconds": observation["monotonic_seconds"],
                "throttled": flags, "phase_file_changed": updated_phase != last_phase,
            }, separators=(",", ":")), flush=True)
        last_flags, last_phase = flags, updated_phase
        print("RPI1_HEALTH " + json.dumps(observation, separators=(",", ":")), flush=True)
        for thread_record in thread_records:
            thread_record["monotonic_seconds"] = observation["monotonic_seconds"]
            print("RPI1_THREAD " + json.dumps(thread_record, separators=(",", ":")), flush=True)
        interval = 0.2 if time.monotonic() < burst_until else 1.
        time.sleep(max(0, started + interval - time.monotonic()))
