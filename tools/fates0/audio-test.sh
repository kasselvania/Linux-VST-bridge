#!/usr/bin/env bash

set -Eeuo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
action=${1:-}
output=${2:-$PWD/fates0-audio-test}
[[ -n $action ]] || { printf 'usage: %s silent|tone|capture|loopback|reopen [output-directory]\n' "$0" >&2; exit 2; }
mkdir -p "$output"

python3 "$SCRIPT_DIR/audio_probe.py" generate "$output/silence.wav" --seconds 1 --silent
python3 "$SCRIPT_DIR/audio_probe.py" generate "$output/tone-440hz-minus30dbfs.wav" --seconds 3

case "$action" in
  silent)
    aplay -D fates "$output/silence.wav" 2>"$output/silent-playback.log"
    ;;
  tone)
    aplay -D fates "$output/tone-440hz-minus30dbfs.wav" 2>"$output/tone-playback.log"
    ;;
  capture)
    arecord -D fates -f S16_LE -r 48000 -c 2 -d 3 -t wav "$output/capture.wav" 2>"$output/capture.log"
    ;;
  loopback)
    arecord -D fates -f S16_LE -r 48000 -c 2 -d 4 -t wav "$output/loopback.wav" 2>"$output/loopback-capture.log" &
    recorder=$!
    sleep 0.25
    aplay -D fates "$output/tone-440hz-minus30dbfs.wav" 2>"$output/loopback-playback.log"
    wait "$recorder"
    python3 "$SCRIPT_DIR/audio_probe.py" analyze "$output/loopback.wav" >"$output/loopback-result.json"
    cat "$output/loopback-result.json"
    ;;
  reopen)
    for _attempt in $(seq 1 20); do
      aplay -q -D fates "$output/silence.wav"
      arecord -q -D fates -f S16_LE -r 48000 -c 2 -d 1 -t raw /dev/null
    done
    printf '20 playback and capture open/close cycles completed\n'
    ;;
  *)
    printf 'unknown action: %s\n' "$action" >&2
    exit 2
    ;;
esac
