#!/usr/bin/env bash

set -Eeuo pipefail

action=${1:-show}
case "$action" in
  apply)
    amixer -c sndrpiproto cset name='Line Bypass Switch' off
    amixer -c sndrpiproto cset name='HiFi Playback Switch' on
    amixer -c sndrpiproto cset name='Mic Sidetone Switch' off
    amixer -c sndrpiproto cset name='Input Mux' 'Line In'
    # Raw values avoid ALSA percentage remapping. WM8731 value 100 is about -21 dB;
    # the generated tone is also -30 dBFS, keeping first line tests deliberately low.
    amixer -c sndrpiproto cset name='Master Playback Volume' 100,100
    amixer -c sndrpiproto cset name='Master Playback ZC Switch' on
    amixer -c sndrpiproto cset name='Capture Volume' 23,23
    amixer -c sndrpiproto cset name='Line Capture Switch' on,on
    amixer -c sndrpiproto cset name='Mic Boost Volume' 0
    amixer -c sndrpiproto cset name='Mic Capture Switch' off
    amixer -c sndrpiproto cset name='Sidetone Playback Volume' 0
    amixer -c sndrpiproto cset name='ADC High Pass Filter Switch' on
    amixer -c sndrpiproto cset name='Store DC Offset Switch' off
    amixer -c sndrpiproto cset name='Playback Deemphasis Switch' off
    ;;
  show)
    amixer -c sndrpiproto scontents
    ;;
  *)
    printf 'usage: %s [apply|show]\n' "$0" >&2
    exit 2
    ;;
esac
