#!/usr/bin/env bash

set -Eeuo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=lib.sh
source "$SCRIPT_DIR/lib.sh"

require_root
require_platform
config=$(boot_config_path)
refuse_boot_conflicts "$config"
boot_dir=$(dirname -- "$config")
fragment=$boot_dir/shieldxl0.conf
stage_source=$SCRIPT_DIR/overlays/boot-buses.conf
platform_source=$SCRIPT_DIR/overlays/boot-platform.conf

if [[ -f $fragment ]]; then
  fragment_hash=$(sha256_file "$fragment")
  if [[ $fragment_hash == $(sha256_file "$platform_source") ]]; then
    note 'full platform stage is already installed; refusing to downgrade it'
    exit 0
  fi
  [[ $fragment_hash == $(sha256_file "$stage_source") ]] ||
    die "foreign boot fragment exists: $fragment"
else
  install -m 0644 "$stage_source" "$fragment"
fi

include_count=$(grep -Fxc 'include shieldxl0.conf' "$config" || true)
[[ $include_count -le 1 ]] || die "duplicate SHIELDXL0 include lines in $config"
if [[ $include_count -eq 0 ]]; then
  mkdir -p "$SHIELDXL0_STATE_DIR/rollback"
  cp -p "$config" "$SHIELDXL0_STATE_DIR/rollback/config.txt.pre-shieldxl0"
  printf '\ninclude shieldxl0.conf\n' >>"$config"
fi

note 'stage 1 installed: only I2C is enabled for the bounded 0x48 address admission'
note 'reboot is required before full provisioning; no reboot was initiated'
