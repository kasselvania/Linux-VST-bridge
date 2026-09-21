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

resolve_usb_identity() {
  local candidate=$1 syspath parent
  USB_VENDOR=
  USB_PRODUCT=
  USB_SERIAL=
  USB_MODEL=
  syspath=$(udevadm info --query=path --name "$candidate")
  [[ $syspath == /devices/* ]] || return 1
  parent=/sys$syspath
  while [[ $parent == /sys/* && $parent != /sys ]]; do
    if [[ -r $parent/idVendor && -r $parent/idProduct ]]; then
      USB_VENDOR=$(tr -d '[:space:]' <"$parent/idVendor")
      USB_PRODUCT=$(tr -d '[:space:]' <"$parent/idProduct")
      [[ -r $parent/serial ]] && USB_SERIAL=$(tr -d '\n' <"$parent/serial")
      [[ -r $parent/product ]] && USB_MODEL=$(tr -d '\n' <"$parent/product")
      [[ $USB_VENDOR =~ ^[0-9A-Fa-f]{4}$ && $USB_PRODUCT =~ ^[0-9A-Fa-f]{4}$ ]] || return 1
      return 0
    fi
    parent=${parent%/*}
  done
  return 1
}

resolve_usb_identity "$device" || die 'device lacks an exact USB VID/PID identity in its parent chain'
vendor=$USB_VENDOR
product=$USB_PRODUCT
serial=$USB_SERIAL
model=$USB_MODEL

matches=0
for candidate in /dev/snd/midiC*D*; do
  [[ -e $candidate ]] || continue
  if resolve_usb_identity "$candidate"; then
    [[ $USB_VENDOR == "$vendor" && $USB_PRODUCT == "$product" ]] && matches=$((matches + 1))
  fi
done
[[ $matches -eq 1 ]] || die "USB VID:PID $vendor:$product is ambiguous across $matches raw MIDI devices"

rule=/etc/udev/rules.d/98-shieldxl0-usb-midi.rules
record=$SHIELDXL0_CONFIG_DIR/usb-midi.json
[[ ! -e $rule && ! -e $record ]] || die 'USB MIDI admission already exists; uninstall or inspect it before replacement'
mkdir -p "$SHIELDXL0_CONFIG_DIR" "$SHIELDXL0_STATE_DIR"
if [[ -n $serial ]]; then
  printf 'SUBSYSTEM=="sound", KERNEL=="midiC*D*", ATTRS{idVendor}=="%s", ATTRS{idProduct}=="%s", ATTRS{serial}=="%s", SYMLINK+="snd/shieldxl-usb-midi", GROUP="audio", MODE="0660", TAG+="uaccess"\n' \
    "$vendor" "$product" "$serial" >"$rule"
  identity_class=usb_vid_pid_serial
else
  printf 'SUBSYSTEM=="sound", KERNEL=="midiC*D*", ATTRS{idVendor}=="%s", ATTRS{idProduct}=="%s", SYMLINK+="snd/shieldxl-usb-midi", GROUP="audio", MODE="0660", TAG+="uaccess"\n' \
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
    "stable_raw_midi_alias": "/dev/snd/shieldxl-usb-midi",
}, indent=2, sort_keys=True) + "\n")
PY
sha256_file "$rule" >"$SHIELDXL0_STATE_DIR/usb-midi-rule.sha256"
udevadm control --reload-rules
udevadm trigger --subsystem-match=sound
note "admitted exactly one USB MIDI identity as /dev/snd/shieldxl-usb-midi ($identity_class)"
