#!/bin/sh
# Private ASC fixture: run under the bounded systemd unit and both operation locks.
set -eu
test -n "${INVOCATION_ID:?run under systemd supervision}"
root=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
test "${WINEPREFIX:?select the copied environment}" = "$root/compatdata/pfx"
if systemctl --user is-active --quiet lvb-rpi1-audio-gate.service; then
    exit 65
fi
test "$(cat /sys/class/thermal/thermal_zone0/temp)" -lt 56000
flags=$(vcgencmd get_throttled)
test "$(( ${flags#*=} & 15 ))" -eq 0
cd "$WINEPREFIX/drive_c/Program Files (x86)/Arturia/Arturia Software Center"
sha256sum --check --status <<'HASHES'
188afb698a0d7838bf491d208f56e9614846e56a60c90919d1018857ca3e7e25  Arturia Software Center.exe
HASHES
"$root/launch-asc.sh" 'C:\Program Files (x86)\Arturia\Arturia Software Center\Arturia Software Center.exe' &
runtime_pid=$!
while kill -0 "$runtime_pid" 2>/dev/null; do
    temperature=$(cat /sys/class/thermal/thermal_zone0/temp)
    flags=$(vcgencmd get_throttled)
    printf 'ASC_RESOURCE_SAMPLE temperature_mc=%s %s\n' "$temperature" "$flags"
    if test "$temperature" -ge 75000 || test "$(( ${flags#*=} & 15 ))" -ne 0; then
        printf '%s\n' 'ASC_RESOURCE_BOUND_STOP'
        exit 70
    fi
    sleep 2
done
wait "$runtime_pid"
