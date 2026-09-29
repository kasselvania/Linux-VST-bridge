#!/usr/bin/env python3
"""Fixed, read-only PB0-C0 staged Deck measurement; prints no product identities."""
import json
import os
import statistics
import subprocess
import time


MANAGER = "/tmp/pb0-c0-manager-stage"


def desktop_environment():
    result = subprocess.run(
        ["systemctl", "--user", "show-environment"],
        capture_output=True, text=True, check=True, timeout=5,
    )
    values = dict(line.split("=", 1) for line in result.stdout.splitlines()
                  if "=" in line and line.split("=", 1)[0] in
                  {"DISPLAY", "XDG_SESSION_TYPE"})
    if values.get("XDG_SESSION_TYPE") not in {"wayland", "x11"}:
        raise RuntimeError("desktop_session_unavailable")
    if not values.get("DISPLAY", "").startswith(":"):
        raise RuntimeError("desktop_display_unavailable")
    return {**os.environ, **values}


def read(verb, environment):
    start = time.monotonic()
    result = subprocess.run(
        [MANAGER, "operator", verb], capture_output=True, check=True,
        timeout=30, env=environment,
    )
    elapsed = round(time.monotonic() - start, 3)
    if len(result.stdout) > 8 * 1024 * 1024 or len(result.stderr) > 8192:
        raise RuntimeError("readback_extent")
    value = json.loads(result.stdout)
    phases = []
    for line in result.stderr.decode().splitlines():
        if not line.startswith("PB0_PHASE "):
            raise RuntimeError("unexpected_diagnostic")
        phases.append(json.loads(line.removeprefix("PB0_PHASE ")))
    return elapsed, value, phases


def main():
    environment = desktop_environment()
    overviews = [read("overview", environment) for _ in range(5)]
    pulses = [read("pulse", environment) for _ in range(5)]
    _, export, _ = read("support-export-preview", environment)
    first = overviews[0][1]
    summaries = [{
        "status": value["readiness"]["overall_status"],
        "products": len(value["current"]["products"]),
        "setups": len(value["current"]["installer_setups"]),
        "workspaces": len(value["current"]["workspaces"]),
        "step": value["readiness"]["ordered_steps"][0]["title"],
        "state_token": value["current"]["state_token"],
    } for _, value, _ in overviews]
    if any(summary != summaries[0] for summary in summaries[1:]):
        raise RuntimeError("overview_changed_during_readonly_measurement")
    if any(pulse[1]["schema"] != 12 for pulse in pulses):
        raise RuntimeError("pulse_schema")
    encoded = json.dumps(export, sort_keys=True).lower()
    for forbidden in ("state_token", "support_export_action", '"kind"',
                      "/home/deck", "compatdata/pfx", "credential=", "token="):
        if forbidden in encoded:
            raise RuntimeError("support_export_forbidden_material")
    print(json.dumps({
        "schema": 1,
        "overview_seconds": [row[0] for row in overviews],
        "overview_median_seconds": round(statistics.median(row[0] for row in overviews), 3),
        "overview_max_seconds": max(row[0] for row in overviews),
        "pulse_seconds": [row[0] for row in pulses],
        "pulse_max_seconds": max(row[0] for row in pulses),
        "current": {key: value for key, value in summaries[0].items()
                    if key != "state_token"},
        "overview_schema": first["schema"],
        "operator_schema": first["operator_schema"],
        "phases": overviews[0][2],
        "export_schema": export["schema"],
        "export_products": len(export["assessment"]["products"]),
        "selected_generation": export["selected_software_generation"],
    }, sort_keys=True))


if __name__ == "__main__":
    main()
