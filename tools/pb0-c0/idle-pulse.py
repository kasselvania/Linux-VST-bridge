#!/usr/bin/env python3
"""Measure the staged idle pulse cadence without running a frontend or DAW."""
import json
import os
import resource
import subprocess
import time


MANAGER = "/tmp/pb0-c0-manager-stage"
COUNT = 6
INTERVAL = 10


def usage():
    result = resource.getrusage(resource.RUSAGE_CHILDREN)
    return result.ru_utime + result.ru_stime, result.ru_nvcsw + result.ru_nivcsw


def main():
    first_cpu, first_switches = usage()
    start = time.monotonic()
    durations = []
    for index in range(COUNT):
        if index:
            time.sleep(max(0, start + index * INTERVAL - time.monotonic()))
        began = time.monotonic()
        result = subprocess.run([MANAGER, "operator", "pulse"], check=True,
                                capture_output=True, timeout=2, env=os.environ.copy())
        pulse = json.loads(result.stdout)
        if pulse["schema"] != 12 or pulse["service_state"] != "active":
            raise RuntimeError("idle_pulse_changed")
        durations.append(round(time.monotonic() - began, 3))
    cpu, switches = usage()
    elapsed = time.monotonic() - start
    print(json.dumps({"schema": 1, "pulses": COUNT, "interval_seconds": INTERVAL,
                      "elapsed_seconds": round(elapsed, 3),
                      "pulse_seconds": durations,
                      "manager_child_cpu_seconds": round(cpu - first_cpu, 4),
                      "manager_child_context_switches": switches - first_switches},
                     sort_keys=True))


if __name__ == "__main__":
    main()
