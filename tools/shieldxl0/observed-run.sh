#!/usr/bin/env bash

set -Eeuo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=lib.sh
source "$SCRIPT_DIR/lib.sh"

usage() {
  printf 'usage: %s --label SAFE_LABEL --shield-airflow obstructed|not-obstructed|unknown --cooling none|external-airflow|active-cooler --output DIRECTORY -- COMMAND [ARG...]\n' "$0" >&2
  exit 2
}

label=
airflow=
cooling=
output=
while [[ $# -gt 0 ]]; do
  case "$1" in
    --label) label=${2:-}; shift 2 ;;
    --shield-airflow) airflow=${2:-}; shift 2 ;;
    --cooling) cooling=${2:-}; shift 2 ;;
    --output) output=${2:-}; shift 2 ;;
    --) shift; break ;;
    *) usage ;;
  esac
done

[[ $label =~ ^[A-Za-z0-9._-]+$ ]] || usage
[[ $airflow == obstructed || $airflow == not-obstructed || $airflow == unknown ]] || usage
[[ $cooling == none || $cooling == external-airflow || $cooling == active-cooler ]] || usage
[[ -n $output && $# -gt 0 ]] || usage

require_platform
config=$(boot_config_path)
refuse_boot_conflicts "$config"
[[ ! -e $output ]] || die "observed-run destination already exists: $output"
mkdir -m 0700 -p "$output"

samples=$output/thermal-samples.csv
printf 'phase,monotonic_ns,temperature_c,throttled,cpu_frequency_hz\n' >"$samples"

monotonic_ns() {
  awk '{printf "%.0f", $1 * 1000000000}' /proc/uptime
}

record_sample() {
  local phase=$1 stamp temperature throttled clock_khz
  stamp=$(monotonic_ns)
  temperature=$(vcgencmd measure_temp | tr -cd '0-9.')
  throttled=$(vcgencmd get_throttled | cut -d= -f2)
  clock_khz=$(cat /sys/devices/system/cpu/cpufreq/policy0/scaling_cur_freq)
  printf '%s,%s,%s,%s,%s\n' "$phase" "$stamp" "$temperature" "$throttled" "$((clock_khz * 1000))" >>"$samples"
}

record_sample before
start_ns=$(monotonic_ns)
"$@" >"$output/workload.stdout" 2>"$output/workload.stderr" &
runner=$!
record_sample during
while kill -0 "$runner" 2>/dev/null; do
  sleep 1
  kill -0 "$runner" 2>/dev/null && record_sample during
done
set +e
wait "$runner"
workload_status=$?
set -e
end_ns=$(monotonic_ns)
record_sample after

python3 - "$samples" "$output/result.json" "$label" "$airflow" "$cooling" \
  "$workload_status" "$start_ns" "$end_ns" "$@" <<'PY'
import csv
import json
import pathlib
import sys

source = pathlib.Path(sys.argv[1])
destination = pathlib.Path(sys.argv[2])
label, airflow, cooling = sys.argv[3:6]
workload_status, start_ns, end_ns = map(int, sys.argv[6:9])
command = sys.argv[9:]
with source.open() as stream:
    rows = list(csv.DictReader(stream))
before = rows[0]
after = rows[-1]
during = [row for row in rows if row["phase"] == "during"]
temperatures = [float(row["temperature_c"]) for row in rows]
frequencies = [int(row["cpu_frequency_hz"]) for row in rows]
flags = sorted(set(row["throttled"] for row in rows))
throttled = flags != ["0x0"]
result = {
    "schema_version": 1,
    "label": label,
    "shieldxl_passive_airflow": airflow,
    "cooling_fixture": cooling,
    "command": command,
    "workload_exit_code": workload_status,
    "workload_started_monotonic_ns": start_ns,
    "workload_finished_monotonic_ns": end_ns,
    "workload_duration_ns": end_ns - start_ns,
    "sample_count": len(rows),
    "temperature_before_c": float(before["temperature_c"]),
    "temperature_during_maximum_c": max(float(row["temperature_c"]) for row in during),
    "temperature_after_c": float(after["temperature_c"]),
    "maximum_temperature_c": max(temperatures),
    "cpu_frequency_before_hz": int(before["cpu_frequency_hz"]),
    "cpu_frequency_during_minimum_hz": min(int(row["cpu_frequency_hz"]) for row in during),
    "cpu_frequency_during_maximum_hz": max(int(row["cpu_frequency_hz"]) for row in during),
    "cpu_frequency_after_hz": int(after["cpu_frequency_hz"]),
    "minimum_cpu_frequency_hz": min(frequencies),
    "maximum_cpu_frequency_hz": max(frequencies),
    "throttling_before": before["throttled"],
    "throttling_during": sorted(set(row["throttled"] for row in during)),
    "throttling_after": after["throttled"],
    "throttling_flags": flags,
    "sustained_performance_classification": (
        "thermally_unqualified" if throttled else "bounded_run_no_throttling_observed"
    ),
}
destination.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
PY

cat "$output/result.json"
exit "$workload_status"
