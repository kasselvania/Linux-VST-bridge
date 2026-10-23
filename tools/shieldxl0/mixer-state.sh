#!/usr/bin/env bash

set -Eeuo pipefail

action=${1:-show}
case "$action" in
  apply)
    amixer -c SHIELDXL cset name='Master Playback Volume' 255,255
    amixer -c SHIELDXL cset name='Master Playback Switch' on,on
    amixer -c SHIELDXL cset name='Master Capture Switch' on,on
    amixer -c SHIELDXL cset name='Digital Sidetone Switch' off
    amixer -c SHIELDXL cset name='Soft Ramp Switch' on
    amixer -c SHIELDXL cset name='Zero Cross Switch' on
    amixer -c SHIELDXL cset name='De-emphasis filter' off
    amixer -c SHIELDXL cset name='Popguard Switch' on
    # The pinned CS4270 driver disables Auto-Mute because the hardware state can
    # fail to release after silence. Keep it off across every JACK restart.
    amixer -c SHIELDXL cset name='Auto-Mute Switch' off
    ;;
  show)
    amixer -c SHIELDXL scontents
    ;;
  *)
    printf 'usage: %s [apply|show]\n' "$0" >&2
    exit 2
    ;;
esac
