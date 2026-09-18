#!/usr/bin/env bash

set -Eeuo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=lib.sh
source "$SCRIPT_DIR/lib.sh"

OUTPUT=${1:-$PWD/shieldxl0-private-inventory}
[[ ! -e $OUTPUT ]] || die "inventory destination already exists: $OUTPUT"
mkdir -m 0700 -p "$OUTPUT"

capture() {
  local name=$1
  shift
  { "$@" || true; } >"$OUTPUT/$name.txt" 2>&1
}

tr -d '\000' </proc/device-tree/model >"$OUTPUT/model.txt"
read_revision >"$OUTPUT/board-revision.txt"
capture uname uname -a
capture os-release sed -n '1,160p' /etc/os-release
capture architecture dpkg --print-architecture
capture page-size getconf PAGESIZE
capture ram grep -E '^(MemTotal|MemAvailable):' /proc/meminfo
capture boot-config-locations find /boot /boot/firmware -maxdepth 1 -name config.txt -type f -print
capture boot-firmware vcgencmd version
capture boot-eeprom rpi-eeprom-update
capture modules lsmod
capture alsa-cards cat /proc/asound/cards
capture alsa-playback aplay -l
capture alsa-capture arecord -l
capture i2c-buses i2cdetect -l
if [[ -e /dev/i2c-1 ]]; then
  capture i2c-address-0x48 python3 "$SCRIPT_DIR/i2c_presence.py"
fi
capture spi-devices sh -c 'find /dev -maxdepth 1 -name "spidev*" -print'
capture input-devices cat /proc/bus/input/devices
capture serial-aliases sh -c 'find /dev -maxdepth 1 \( -name "serial*" -o -name "ttyAMA*" -o -name "ttyS*" \) -print'
capture gpio-consumers gpioinfo
capture pipewire-status systemctl --user status pipewire.service
capture jack-processes pgrep -a jackd
capture temperature vcgencmd measure_temp
capture throttling vcgencmd get_throttled
capture cpu-governor sh -c 'for f in /sys/devices/system/cpu/cpufreq/policy*/scaling_governor; do printf "%s " "$f"; cat "$f"; done'
capture cpu-clock-range sh -c 'for f in /sys/devices/system/cpu/cpufreq/policy*/cpuinfo_{min,max}_freq; do printf "%s " "$f"; cat "$f"; done'
capture repositories sh -c 'grep -RhE "^[[:space:]]*(deb|URIs:|Suites:|Components:)" /etc/apt/sources.list /etc/apt/sources.list.d 2>/dev/null'
capture packages dpkg-query -W -f='${binary:Package}\t${Version}\t${Architecture}\n'

cat >"$OUTPUT/NOTICE.txt" <<'EOF'
Private pre-change inventory. It intentionally excludes hostname, network addresses,
credentials, and arbitrary codec-register reads. Review and sanitize before committing.
EOF
note "private inventory written to $OUTPUT"
