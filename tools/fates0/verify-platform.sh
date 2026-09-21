#!/usr/bin/env bash

set -Eeuo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=lib.sh
source "$SCRIPT_DIR/lib.sh"

OUTPUT=${1:-$PWD/fates0-platform-verification}
[[ ! -e $OUTPUT ]] || die "verification destination already exists: $OUTPUT"
mkdir -m 0700 -p "$OUTPUT"
require_platform

failures=()
check() {
  local description=$1
  shift
  if "$@"; then
    printf 'PASS\t%s\n' "$description" | tee -a "$OUTPUT/checks.tsv"
  else
    printf 'FAIL\t%s\n' "$description" | tee -a "$OUTPUT/checks.tsv"
    failures+=("$description")
  fi
}

check 'distribution WM8731 module is installed' modinfo snd_soc_wm8731
check 'distribution WM8731 I2C module is installed' modinfo snd_soc_wm8731_i2c
check 'distribution PROTO machine driver is installed' modinfo snd_soc_rpi_proto
check 'WM8731 driver is loaded' grep -Fq snd_soc_wm8731 /proc/modules
check 'PROTO machine driver is loaded' grep -Fq snd_soc_rpi_proto /proc/modules
check 'WM8731 is bound at I2C 1-001a' test -e /sys/bus/i2c/devices/1-001a/driver
check 'sndrpiproto ALSA card exists' grep -Fq sndrpiproto /proc/asound/cards
check 'sndrpiproto playback PCM exists' sh -c 'aplay -l | grep -Fq sndrpiproto'
check 'sndrpiproto capture PCM exists' sh -c 'arecord -l | grep -Fq sndrpiproto'
for index in 1 2 3; do
  check "encoder $index stable alias exists" test -e "/dev/input/fates-encoder-$index"
  check "button $index stable alias exists" test -e "/dev/input/fates-button-$index"
done
check 'OLED stable SPI alias exists' test -e /dev/fates-oled-spi

timeout --signal=INT 1s aplay --dump-hw-params -D fates -f S16_LE -r 48000 -c 2 /dev/zero >"$OUTPUT/playback-hw-params.txt" 2>&1 || true
arecord --dump-hw-params -D fates -f S16_LE -r 48000 -c 2 -d 1 -t raw /dev/null >"$OUTPUT/capture-hw-params.txt" 2>&1 || true
amixer -c sndrpiproto scontents >"$OUTPUT/mixer-state.txt" 2>&1 || true
journalctl -b -k --no-hostname | grep -Ei 'fates|wm8731|rpi.proto|designware.*i2s|rp1.*i2s|xrun|underrun|overrun' >"$OUTPUT/kernel-audio-log.txt" || true
systemctl --no-pager --full status "fates-jack@$(python3 -c 'import json; print(json.load(open("/var/lib/fates0/state.json"))["target_user"])').service" >"$OUTPUT/jack-service.txt" 2>&1 || true
systemctl --no-pager --full status "fates-oled@$(python3 -c 'import json; print(json.load(open("/var/lib/fates0/state.json"))["target_user"])').service" >"$OUTPUT/oled-service.txt" 2>&1 || true

if [[ ${#failures[@]} -gt 0 ]]; then
  printf 'platform verification failed: %s\n' "${failures[*]}" >&2
  exit 1
fi
note "platform enumeration verified; retained output is $OUTPUT"
