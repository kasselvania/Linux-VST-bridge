#!/usr/bin/env bash

set -Eeuo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=lib.sh
source "$SCRIPT_DIR/lib.sh"

usage() {
  printf 'usage: %s --board-revision v1.8.1 --power-input pi\n' "$0" >&2
  exit 2
}

board_revision=
power_input=
while [[ $# -gt 0 ]]; do
  case "$1" in
    --board-revision) board_revision=${2:-}; shift 2 ;;
    --power-input) power_input=${2:-}; shift 2 ;;
    *) usage ;;
  esac
done
[[ -n $board_revision && -n $power_input ]] || usage

require_root
require_platform
require_fixture_declaration "$board_revision" "$power_input"
config=$(boot_config_path)
refuse_boot_conflicts "$config"
boot_dir=$(dirname -- "$config")
fragment=$boot_dir/fates0.conf
stage_source=$SCRIPT_DIR/overlays/boot-buses.conf
platform_source=$SCRIPT_DIR/overlays/boot-platform.conf
include_count=$(grep -Fxc 'include fates0.conf' "$config" || true)
[[ $include_count -le 1 ]] || die "duplicate FATES0 include lines in $config"
[[ $include_count -eq 0 || -f $fragment ]] || die 'managed include exists without its FATES0 fragment'

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

if [[ $include_count -eq 0 ]]; then
  mkdir -p "$FATES0_STATE_DIR/rollback"
  backup=$FATES0_STATE_DIR/rollback/config.txt.pre-fates0
  if [[ -e $backup || -L $backup ]]; then
    [[ ! -L $backup && -f $backup && $(sha256_file "$backup") == $(sha256_file "$config") ]] ||
      die "foreign or stale rollback file exists: $backup"
  else
    cp -p "$config" "$backup"
  fi
  printf '\ninclude fates0.conf\n' >>"$config"
fi

mkdir -p "$FATES0_STATE_DIR"
declaration=$FATES0_STATE_DIR/fixture-declaration.json
if [[ -e $declaration || -L $declaration ]]; then
  [[ ! -L $declaration && -f $declaration ]] || die "foreign fixture declaration path exists: $declaration"
  python3 - "$declaration" "$board_revision" "$power_input" <<'PY'
import json, sys
record = json.load(open(sys.argv[1]))
expected = {
    "schema_version": 1,
    "board_revision": sys.argv[2],
    "power_input": sys.argv[3],
    "fates_usb_c_disconnected": True,
    "evidence_class": "operator_declaration",
}
if record != expected:
    raise SystemExit("existing fixture declaration differs; refusing replacement")
PY
else
  python3 - "$declaration" "$board_revision" "$power_input" <<'PY'
import json, pathlib, sys
destination, revision, power = sys.argv[1:]
pathlib.Path(destination).write_text(json.dumps({
    "schema_version": 1,
    "board_revision": revision,
    "power_input": power,
    "fates_usb_c_disconnected": True,
    "evidence_class": "operator_declaration",
}, indent=2, sort_keys=True) + "\n")
PY
fi

note 'stage 1 installed: only I2C is enabled for the bounded 0x1a address admission'
note 'reboot is required before full provisioning; no reboot was initiated'
