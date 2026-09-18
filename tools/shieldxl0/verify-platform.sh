#!/usr/bin/env bash

set -Eeuo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=lib.sh
source "$SCRIPT_DIR/lib.sh"

OUTPUT=${1:-$PWD/shieldxl0-platform-verification}
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

check 'CS4270 module is installed for the running kernel' modinfo snd_soc_cs4270
check 'CS4270 driver is loaded' grep -Fq snd_soc_cs4270 /proc/modules
check 'CS4270 is bound at I2C 1-0048' test -e /sys/bus/i2c/devices/1-0048/driver
check 'SHIELDXL ALSA card exists' grep -Fq SHIELDXL /proc/asound/cards
check 'SHIELDXL playback PCM opens in enumeration' sh -c 'aplay -l | grep -Fq SHIELDXL'
check 'SHIELDXL capture PCM opens in enumeration' sh -c 'arecord -l | grep -Fq SHIELDXL'
for index in 1 2 3; do
  check "encoder $index stable alias exists" test -e "/dev/input/shieldxl-encoder-$index"
  check "button $index stable alias exists" test -e "/dev/input/shieldxl-button-$index"
done
check 'OLED stable SPI alias exists' test -e /dev/shieldxl-oled-spi

timeout --signal=INT 1s aplay --dump-hw-params -D shieldxl -f S16_LE -r 48000 -c 2 /dev/zero >"$OUTPUT/playback-hw-params.txt" 2>&1 || true
arecord --dump-hw-params -D shieldxl -f S16_LE -r 48000 -c 2 -d 1 -t raw /dev/null >"$OUTPUT/capture-hw-params.txt" 2>&1 || true
amixer -c SHIELDXL scontents >"$OUTPUT/mixer-state.txt" 2>&1 || true
journalctl -b -k --no-hostname | grep -Ei 'shieldxl|cs4270|bcm2835.*i2s|simple.card|xrun|underrun|overrun' >"$OUTPUT/kernel-audio-log.txt" || true
systemctl --no-pager --full status "shieldxl-jack@$(python3 -c 'import json; print(json.load(open("/var/lib/shieldxl0/state.json"))["target_user"])').service" >"$OUTPUT/jack-service.txt" 2>&1 || true
systemctl --no-pager --full status "shieldxl-oled@$(python3 -c 'import json; print(json.load(open("/var/lib/shieldxl0/state.json"))["target_user"])').service" >"$OUTPUT/oled-service.txt" 2>&1 || true

if [[ ${#failures[@]} -gt 0 ]]; then
  printf 'platform verification failed: %s\n' "${failures[*]}" >&2
  exit 1
fi
note "platform enumeration verified; retained output is $OUTPUT"
