#!/usr/bin/env bash

set -Eeuo pipefail

FATES0_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
FATES0_KERNEL=6.18.50+rpt-rpi-2712
FATES0_MODEL='Raspberry Pi 5 Model B'
FATES0_PAGE_SIZE=16384
FATES0_RAM_KIB_MIN=7500000
FATES0_RAM_KIB_MAX=8500000
FATES0_STATE_DIR=/var/lib/fates0
FATES0_CONFIG_DIR=/etc/fates0

die() { printf 'FATES0: %s\n' "$*" >&2; exit 1; }
note() { printf 'FATES0: %s\n' "$*"; }

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | awk '{print $1}';
  else shasum -a 256 "$1" | awk '{print $1}'; fi
}

require_root() { [[ ${EUID:-$(id -u)} -eq 0 ]] || die 'run this provisioning step as root'; }
read_model() { tr -d '\000' </proc/device-tree/model 2>/dev/null || true; }
read_revision() { awk -F: '/^Revision[[:space:]]*:/ {gsub(/[[:space:]]/, "", $2); print $2; exit}' /proc/cpuinfo; }

require_fixture_declaration() {
  local board_revision=$1 power_input=$2
  [[ $board_revision == v1.8.1 ]] || die "unsupported Fates revision: expected v1.8.1, observed '$board_revision'"
  [[ $power_input == pi ]] || die 'power declaration must be pi; Fates USB-C must remain disconnected'
}

refuse_prohibited_runtimes() {
  local prohibited
  for prohibited in box64 FEXInterpreter wine wine64 proton; do
    command -v "$prohibited" >/dev/null 2>&1 && die "prohibited software '$prohibited' is present"
  done
  return 0
}

require_platform() {
  local model architecture release codename page_size ram_kib
  [[ -r /proc/device-tree/model ]] || die 'device-tree model is unavailable'
  model=$(read_model)
  [[ $model == "$FATES0_MODEL"* ]] || die "wrong board: expected Pi 5 Model B, observed '$model'"
  architecture=$(dpkg --print-architecture)
  [[ $architecture == arm64 ]] || die "wrong architecture: expected arm64, observed '$architecture'"
  release=$(uname -r)
  [[ $release == "$FATES0_KERNEL" ]] || die "unsupported kernel: expected $FATES0_KERNEL, observed $release"
  # shellcheck disable=SC1091
  source /etc/os-release
  codename=${VERSION_CODENAME:-}
  [[ ${ID:-} == debian && $codename == trixie ]] || die "unsupported OS: ${ID:-unknown}/${codename:-unknown}"
  grep -Fqx 'Raspberry Pi reference 2026-09-15' /etc/rpi-issue || die 'image identity differs from the pinned image'
  page_size=$(getconf PAGESIZE)
  [[ $page_size == "$FATES0_PAGE_SIZE" ]] || die "unsupported page size: expected $FATES0_PAGE_SIZE, observed $page_size"
  ram_kib=$(awk '/^MemTotal:/ {print $2; exit}' /proc/meminfo)
  [[ $ram_kib =~ ^[0-9]+$ && $ram_kib -ge $FATES0_RAM_KIB_MIN && $ram_kib -le $FATES0_RAM_KIB_MAX ]] ||
    die "wrong RAM fixture: expected Pi 5 8 GB, observed ${ram_kib:-unknown} KiB"
  [[ -n $(read_revision) ]] || die 'Pi board revision is unavailable'
  refuse_prohibited_runtimes
}

boot_config_path() {
  [[ -f /boot/firmware/config.txt && ! -L /boot/firmware/config.txt ]] || die 'pinned boot config is missing or redirected'
  printf '%s\n' /boot/firmware/config.txt
}

refuse_boot_conflicts() {
  local config=$1 line overlay section=all
  if grep -Eiq '^[[:space:]]*(kernel=|dtoverlay=(rpi-proto|proto-codec|fates[^,[:space:]]*|shieldxl0|monome-snd-4270|norns-buttons-encoders|ssd1322-spi|midi-uart0)|include[[:space:]]+(shieldxl0|fates)\.conf([[:space:]]|$)|(arm|core|gpu|v3d|isp|hevc)_freq(_min)?=|over_voltage(_min|_delta)?=|force_turbo=)' "$config"; then
    die "unexpected conflicting boot configuration in $config"
  fi
  while IFS= read -r line || [[ -n $line ]]; do
    line=${line%%#*}; [[ $line =~ ^[[:space:]]*$ ]] && continue
    if [[ $line =~ ^[[:space:]]*\[([^]]+)\][[:space:]]*$ ]]; then section=${BASH_REMATCH[1]}; continue; fi
    [[ $line =~ ^[[:space:]]*dtoverlay=([^[:space:]]+)[[:space:]]*$ ]] || continue
    overlay=${BASH_REMATCH[1]}
    case "$section:$overlay" in
      all:vc4-kms-v3d | cm5:dwc2,dr_mode=host | pi5:nospi10) ;;
      *) die "unadmitted existing device-tree overlay in $config: [$section] $overlay" ;;
    esac
  done <"$config"
}

ensure_i2c_character_device() {
  local bus=${1:-1} adapter=${2:-/sys/bus/i2c/devices/i2c-$bus} device=${3:-/dev/i2c-$bus}
  [[ -e $adapter ]] || die "I2C adapter $bus is absent after reboot"
  [[ -e $device ]] || modprobe i2c-dev || die 'failed to load i2c-dev'
  [[ -e $device ]] || die "I2C character device is absent: $device"
}

install_exact() {
  local source=$1 destination=$2 mode=$3
  if [[ -e $destination || -L $destination ]]; then
    [[ ! -L $destination && -f $destination && $(sha256_file "$source") == $(sha256_file "$destination") ]] || die "foreign replacement file exists: $destination"
    chmod "$mode" "$destination"; return
  fi
  install -D -m "$mode" "$source" "$destination"
}

package_candidate() { apt-cache policy "$1" | awk '/Candidate:/ {print $2; exit}'; }
