#!/usr/bin/env bash

set -Eeuo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)

usage() {
  printf 'usage: %s --seconds 60..3600 --board-airflow obstructed|not-obstructed|unknown --cooling none|external-airflow|active-cooler --output DIRECTORY\n' "$0" >&2
  exit 2
}

duration=
airflow=
cooling=
output=
while [[ $# -gt 0 ]]; do
  case "$1" in
    --seconds) duration=${2:-}; shift 2 ;;
    --board-airflow) airflow=${2:-}; shift 2 ;;
    --cooling) cooling=${2:-}; shift 2 ;;
    --output) output=${2:-}; shift 2 ;;
    *) usage ;;
  esac
done
[[ $duration =~ ^[0-9]+$ && $duration -ge 60 && $duration -le 3600 ]] || usage
[[ -n $airflow && -n $cooling && -n $output ]] || usage
systemctl --quiet is-active "fates-jack@$(id -un).service" || {
  printf 'ordinary-user JACK service is not active\n' >&2
  exit 1
}

exec "$SCRIPT_DIR/observed-run.sh" \
  --label native-jack-sustained \
  --board-airflow "$airflow" \
  --cooling "$cooling" \
  --output "$output" \
  -- sleep "$duration"
