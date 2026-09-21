#!/usr/bin/env bash

set -Eeuo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=lib.sh
source "$SCRIPT_DIR/lib.sh"

require_root
require_platform
[[ -f $FATES0_STATE_DIR/state.json ]] || die 'no FATES0 installation state exists'
target_user=$(python3 -c 'import json; print(json.load(open("/var/lib/fates0/state.json"))["target_user"])')
systemctl disable --now "fates-jack@$target_user.service" "fates-oled@$target_user.service" || true

config=$(boot_config_path)
boot_dir=$(dirname -- "$config")
fragment=$boot_dir/fates0.conf
if [[ -f $fragment ]]; then
  fragment_hash=$(sha256_file "$fragment")
  if [[ $fragment_hash == $(sha256_file "$SCRIPT_DIR/overlays/boot-platform.conf") || $fragment_hash == $(sha256_file "$SCRIPT_DIR/overlays/boot-buses.conf") ]]; then
    rm "$fragment"
  else
    die "refusing to remove modified boot fragment: $fragment"
  fi
fi

python3 - "$config" <<'PY'
import pathlib, sys
path = pathlib.Path(sys.argv[1])
lines = path.read_text().splitlines()
if lines.count("include fates0.conf") > 1:
    raise SystemExit("duplicate managed include lines; refusing edit")
path.write_text("\n".join(line for line in lines if line != "include fates0.conf") + "\n")
PY

declare -A owned=(
  ["$boot_dir/overlays/fates0.dtbo"]="$SCRIPT_DIR/overlays/fates0.dtbo"
  [/etc/udev/rules.d/99-fates0.rules]="$SCRIPT_DIR/config/99-fates0.rules"
  [/etc/alsa/conf.d/99-fates0.conf]="$SCRIPT_DIR/config/99-fates0-alsa.conf"
  [/etc/modules-load.d/fates0.conf]="$SCRIPT_DIR/config/fates0-modules.conf"
  [/etc/security/limits.d/99-fates0.conf]="$SCRIPT_DIR/config/99-fates0-limits.conf"
  [/etc/systemd/system/fates-jack@.service]="$SCRIPT_DIR/systemd/fates-jack@.service"
  [/etc/systemd/system/fates-oled@.service]="$SCRIPT_DIR/systemd/fates-oled@.service"
  ["$FATES0_CONFIG_DIR/jack.env"]="$SCRIPT_DIR/config/jack.env"
)
for destination in "${!owned[@]}"; do
  if [[ -e $destination ]]; then
    [[ $(sha256_file "$destination") == $(sha256_file "${owned[$destination]}") ]] ||
      die "refusing to remove modified file: $destination"
    rm "$destination"
  fi
done

if [[ -f /etc/udev/rules.d/98-fates0-usb-midi.rules ]]; then
  [[ -f $FATES0_STATE_DIR/usb-midi-rule.sha256 ]] || die 'USB MIDI rule has no ownership hash'
  [[ $(sha256_file /etc/udev/rules.d/98-fates0-usb-midi.rules) == $(cat "$FATES0_STATE_DIR/usb-midi-rule.sha256") ]] ||
    die 'refusing to remove modified USB MIDI rule'
  rm /etc/udev/rules.d/98-fates0-usb-midi.rules
fi
rm -f "$FATES0_CONFIG_DIR/usb-midi.json"
for program in controls.py oled_service.py oled_client.py audio_probe.py audio-test.sh mixer-state.sh observed-run.sh thermal-observe.sh; do
  destination="/usr/local/libexec/fates0/$program"
  if [[ -e $destination ]]; then
    [[ ! -L $destination && -f $destination ]] || die "unexpected installed program path type: $destination"
    [[ $(sha256_file "$destination") == $(sha256_file "$SCRIPT_DIR/$program") ]] ||
      die "refusing to remove modified program: $destination"
    rm "$destination"
  fi
done
rmdir /usr/local/libexec/fates0 2>/dev/null || true
rmdir "$FATES0_CONFIG_DIR" 2>/dev/null || true
udevadm control --reload-rules
systemctl daemon-reload

note 'FATES0 boot configuration, aliases, and services were removed'
note 'added Debian packages and user group memberships were retained to avoid deleting shared state'
note 'reboot is required to detach the active overlay; no reboot was initiated'
