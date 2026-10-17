#!/usr/bin/env bash

set -Eeuo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=lib.sh
source "$SCRIPT_DIR/lib.sh"

target_user=${1:-}
[[ -n $target_user ]] || { printf 'usage: %s ORDINARY_USER\n' "$0" >&2; exit 2; }
require_root
require_platform
[[ -f $SHIELDXL0_STATE_DIR/state.json ]] || die 'provisioning state is absent'
recorded_user=$(python3 -c 'import json; print(json.load(open("/var/lib/shieldxl0/state.json"))["target_user"])')
[[ $target_user == "$recorded_user" ]] || die "target user differs from provisioning state: $recorded_user"
grep -Fq SHIELDXL /proc/asound/cards || die 'SHIELDXL ALSA card is absent; refuse audio-server activation'
systemctl enable --now "shieldxl-jack@$target_user.service"
systemctl enable "shieldxl-oled@$target_user.service"
if ! systemctl start "shieldxl-oled@$target_user.service"; then
  note 'OLED service failed independently; JACK remains authoritative and running'
fi
systemctl is-active --quiet "shieldxl-jack@$target_user.service" || die 'JACK service did not become active'
note 'ordinary-user JACK is enabled and active; OLED was handled as an independent service'
