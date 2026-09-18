#!/usr/bin/env bash

set -Eeuo pipefail

duration=${1:-600}
output=${2:-$PWD/shieldxl0-thermal}
[[ $duration =~ ^[0-9]+$ && $duration -ge 60 && $duration -le 3600 ]] || {
  printf 'usage: %s SECONDS_60_TO_3600 [output-directory]\n' "$0" >&2
  exit 2
}
[[ ! -e $output ]] || { printf 'destination exists: %s\n' "$output" >&2; exit 1; }
mkdir -m 0700 -p "$output"
systemctl --quiet is-active "shieldxl-jack@$(id -un).service" || {
  printf 'ordinary-user JACK service is not active\n' >&2
  exit 1
}

printf 'monotonic_ns,temperature_c,throttled,clock_hz\n' >"$output/samples.csv"
start=$(date +%s)
while (( $(date +%s) - start < duration )); do
  monotonic_ns=$(awk '{printf "%.0f", $1 * 1000000000}' /proc/uptime)
  temperature=$(vcgencmd measure_temp | tr -cd '0-9.')
  throttled=$(vcgencmd get_throttled | cut -d= -f2)
  clock=$(cat /sys/devices/system/cpu/cpufreq/policy0/scaling_cur_freq)
  printf '%s,%s,%s,%s\n' "$monotonic_ns" "$temperature" "$throttled" "$((clock * 1000))" >>"$output/samples.csv"
  sleep 1
done

python3 - "$output/samples.csv" "$output/result.json" <<'PY'
import csv, json, pathlib, sys
source, destination = map(pathlib.Path, sys.argv[1:])
with source.open() as stream:
    rows = list(csv.DictReader(stream))
temperatures = [float(row["temperature_c"]) for row in rows]
clocks = [int(row["clock_hz"]) for row in rows]
flags = sorted(set(row["throttled"] for row in rows))
result = {
    "sample_count": len(rows),
    "max_temperature_c": max(temperatures),
    "min_clock_hz": min(clocks),
    "max_clock_hz": max(clocks),
    "throttling_flags": flags,
    "no_throttling": flags == ["0x0"],
}
destination.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
raise SystemExit(0 if result["no_throttling"] else 1)
PY
cat "$output/result.json"
