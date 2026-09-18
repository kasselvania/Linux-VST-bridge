#!/usr/bin/env bash

set -Eeuo pipefail

action=${1:-show}
case "$action" in
  apply)
    amixer -c SHIELDXL sset 'Master Playback Volume' 255,255
    amixer -c SHIELDXL sset 'Master Playback Switch' on,on
    amixer -c SHIELDXL sset 'Master Capture Switch' on,on
    amixer -c SHIELDXL sset 'Digital Sidetone Switch' off
    amixer -c SHIELDXL sset 'Soft Ramp Switch' on
    amixer -c SHIELDXL sset 'Zero Cross Switch' on
    amixer -c SHIELDXL sset 'De-emphasis filter' off
    amixer -c SHIELDXL sset 'Popguard Switch' on
    amixer -c SHIELDXL sset 'Auto-Mute Switch' on
    ;;
  show)
    amixer -c SHIELDXL scontents
    ;;
  *)
    printf 'usage: %s [apply|show]\n' "$0" >&2
    exit 2
    ;;
esac
