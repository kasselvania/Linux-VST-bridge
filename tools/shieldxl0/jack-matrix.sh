#!/usr/bin/env bash

set -Eeuo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=lib.sh
source "$SCRIPT_DIR/lib.sh"

usage() {
  printf 'usage: %s --user ORDINARY_USER --loopback-connected OUTPUT_DIRECTORY\n' "$0" >&2
  exit 2
}

target_user=
loopback=
output=
while [[ $# -gt 0 ]]; do
  case "$1" in
    --user) target_user=${2:-}; shift 2 ;;
    --loopback-connected) loopback=yes; shift ;;
    -*) usage ;;
    *) [[ -z $output ]] || usage; output=$1; shift ;;
  esac
done
[[ -n $target_user && $loopback == yes && -n $output ]] || usage
require_root
require_platform
[[ ! -e $output ]] || die "JACK matrix destination already exists: $output"
mkdir -m 0700 -p "$output"
service="shieldxl-jack@$target_user.service"
env_file=$SHIELDXL0_CONFIG_DIR/jack.env
[[ -f $env_file ]] || die "missing JACK environment file: $env_file"
saved_env=$(mktemp /var/tmp/shieldxl0-jack-env.XXXXXX)
cp -p "$env_file" "$saved_env"

restore() {
  cp "$saved_env" "$env_file"
  rm -f "$saved_env"
  systemctl restart "$service" >/dev/null 2>&1 || true
}
trap restore EXIT
printf 'period_frames\tperiods\tmeasured_rate\txruns\tleft_loopback\tright_loopback\tcpu_percent\ttemperature_c\tthrottled\tserver_restart\n' >"$output/results.tsv"

as_user() {
  runuser -u "$target_user" -- "$@"
}

loopback_channel() {
  local period=$1 channel=$2 log=$3 runner ready=no
  as_user timeout --signal=INT 8s jack_iodelay >"$log" 2>&1 &
  runner=$!
  for _attempt in $(seq 1 30); do
    if as_user jack_lsp 2>/dev/null | grep -Fqx 'jack_delay:out'; then
      ready=yes
      break
    fi
    sleep 0.1
  done
  if [[ $ready != yes ]]; then
    wait "$runner" || true
    return 1
  fi
  as_user jack_connect jack_delay:out "system:playback_$channel"
  as_user jack_connect "system:capture_$channel" jack_delay:in
  wait "$runner" || true
  grep -Fq 'total roundtrip latency:' "$log"
}

for period in 128 256 512 1024; do
  cat >"$env_file" <<EOF
SHIELDXL_SAMPLE_RATE=48000
SHIELDXL_PERIOD_FRAMES=$period
SHIELDXL_PERIODS=3
EOF
  started=$(date --iso-8601=seconds)
  restart=pass
  if ! systemctl restart "$service"; then
    restart=fail
  fi
  sleep 2
  if ! systemctl is-active --quiet "$service"; then
    printf '%s\t3\t0\t0\tfail\tfail\t0\t%s\t%s\tfail\n' \
      "$period" "$(vcgencmd measure_temp | tr -cd '0-9.')" "$(vcgencmd get_throttled | cut -d= -f2)" >>"$output/results.tsv"
    journalctl -u "$service" --since "$started" --no-hostname >"$output/jack-$period.log"
    continue
  fi
  as_user jack_lsp -tp >"$output/ports-$period.txt"
  left=fail
  right=fail
  loopback_channel "$period" 1 "$output/loopback-left-$period.log" && left=pass
  loopback_channel "$period" 2 "$output/loopback-right-$period.log" && right=pass
  for _attempt in $(seq 1 20); do as_user jack_lsp >/dev/null; done
  measured=$(as_user jack_samplerate 2>/dev/null || printf '0')
  cpu=$(ps -C jackd -o %cpu= | awk '{sum += $1} END {printf "%.1f", sum + 0}')
  temperature=$(vcgencmd measure_temp | tr -cd '0-9.')
  throttled=$(vcgencmd get_throttled | cut -d= -f2)
  journalctl -u "$service" --since "$started" --no-hostname >"$output/jack-$period.log"
  xruns=$(grep -Eic 'xrun|underrun|overrun' "$output/jack-$period.log" || true)
  printf '%s\t3\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
    "$period" "$measured" "$xruns" "$left" "$right" "$cpu" "$temperature" "$throttled" "$restart" >>"$output/results.tsv"
done

# Exercise an ordinary stop and a fresh start before restoring the accepted default.
systemctl stop "$service"
[[ $(systemctl is-active "$service" || true) == inactive ]] || die 'JACK did not stop normally'
systemctl start "$service"
systemctl is-active --quiet "$service" || die 'JACK did not restart after a normal stop'

python3 - "$output/results.tsv" "$output/results.json" <<'PY'
import csv, json, pathlib, sys
source, destination = map(pathlib.Path, sys.argv[1:])
with source.open() as stream:
    rows = list(csv.DictReader(stream, delimiter="\t"))
for row in rows:
    for key in ("period_frames", "periods", "measured_rate", "xruns"):
        row[key] = int(float(row[key]))
    for key in ("cpu_percent", "temperature_c"):
        row[key] = float(row[key])
destination.write_text(json.dumps({
    "server": "JACK2 1.9.22~dfsg-4",
    "alsa_device": "hw:SHIELDXL",
    "rate_hz": 48000,
    "sample_format": "S16_LE",
    "configurations": rows,
}, indent=2, sort_keys=True) + "\n")
PY
cat "$output/results.json"
