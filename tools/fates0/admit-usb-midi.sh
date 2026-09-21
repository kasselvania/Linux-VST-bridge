#!/usr/bin/env bash

set -Eeuo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=lib.sh
source "$SCRIPT_DIR/lib.sh"

device=${1:-}
[[ -n $device ]] || { printf 'usage: %s /dev/snd/midiC...D...\n' "$0" >&2; exit 2; }
require_root
require_platform
[[ -c $device && $device == /dev/snd/midiC*D* ]] || die "not an ALSA raw MIDI device: $device"

properties=$(udevadm info --query=property --name "$device")
property() {
  local name=$1
  printf '%s\n' "$properties" | sed -n "s/^$name=//p" | head -n 1
}
bus=$(property ID_BUS)
vendor=$(property ID_VENDOR_ID)
product=$(property ID_MODEL_ID)
serial=$(property ID_SERIAL_SHORT)
model=$(property ID_MODEL)
[[ $bus == usb && $vendor =~ ^[0-9A-Fa-f]{4}$ && $product =~ ^[0-9A-Fa-f]{4}$ ]] ||
  die 'device lacks an exact USB VID/PID identity'

matches=0
for candidate in /dev/snd/midiC*D*; do
  [[ -e $candidate ]] || continue
  candidate_properties=$(udevadm info --query=property --name "$candidate")
  candidate_vendor=$(printf '%s\n' "$candidate_properties" | sed -n 's/^ID_VENDOR_ID=//p' | head -n 1)
  candidate_product=$(printf '%s\n' "$candidate_properties" | sed -n 's/^ID_MODEL_ID=//p' | head -n 1)
  [[ $candidate_vendor == "$vendor" && $candidate_product == "$product" ]] && matches=$((matches + 1))
done
[[ $matches -eq 1 ]] || die "USB VID:PID $vendor:$product is ambiguous across $matches raw MIDI devices"

rule=/etc/udev/rules.d/98-fates0-usb-midi.rules
record=$FATES0_CONFIG_DIR/usb-midi.json
[[ ! -e $rule && ! -e $record ]] || die 'USB MIDI admission already exists; uninstall or inspect it before replacement'
mkdir -p "$FATES0_CONFIG_DIR" "$FATES0_STATE_DIR"
if [[ -n $serial ]]; then
  printf 'SUBSYSTEM=="sound", KERNEL=="midiC*D*", ATTRS{idVendor}=="%s", ATTRS{idProduct}=="%s", ATTRS{serial}=="%s", SYMLINK+="snd/fates-usb-midi", GROUP="audio", MODE="0660", TAG+="uaccess"\n' \
    "$vendor" "$product" "$serial" >"$rule"
  identity_class=usb_vid_pid_serial
else
  printf 'SUBSYSTEM=="sound", KERNEL=="midiC*D*", ATTRS{idVendor}=="%s", ATTRS{idProduct}=="%s", SYMLINK+="snd/fates-usb-midi", GROUP="audio", MODE="0660", TAG+="uaccess"\n' \
    "$vendor" "$product" >"$rule"
  identity_class=unique_usb_vid_pid
fi
chmod 0644 "$rule"
python3 - "$record" "$identity_class" "$vendor" "$product" "$serial" "$model" <<'PY'
import json, pathlib, sys
destination, identity_class, vendor, product, serial, model = sys.argv[1:]
pathlib.Path(destination).write_text(json.dumps({
    "identity_class": identity_class,
    "vendor_id": vendor.lower(),
    "product_id": product.lower(),
    "serial_present": bool(serial),
    "model": model,
    "stable_raw_midi_alias": "/dev/snd/fates-usb-midi",
}, indent=2, sort_keys=True) + "\n")
PY
sha256_file "$rule" >"$FATES0_STATE_DIR/usb-midi-rule.sha256"
udevadm control --reload-rules
udevadm trigger --subsystem-match=sound
note "admitted exactly one USB MIDI identity as /dev/snd/fates-usb-midi ($identity_class)"
