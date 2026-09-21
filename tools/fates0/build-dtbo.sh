#!/usr/bin/env bash

set -Eeuo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
SOURCE=$SCRIPT_DIR/overlays/fates0-overlay.dts
OUTPUT=${1:-$SCRIPT_DIR/overlays/fates0.dtbo}
EXPECTED_DTC='Version: DTC 1.7.2'

command -v dtc >/dev/null 2>&1 || { printf 'device-tree compiler is required\n' >&2; exit 1; }
[[ $(dtc --version) == "$EXPECTED_DTC" ]] || {
  printf 'expected %s, observed %s\n' "$EXPECTED_DTC" "$(dtc --version)" >&2
  exit 1
}

mkdir -p "$(dirname -- "$OUTPUT")"
temporary=$(mktemp "${OUTPUT}.tmp.XXXXXX")
trap 'rm -f "$temporary"' EXIT
dtc -@ -H epapr -I dts -O dtb -o "$temporary" "$SOURCE"
chmod 0644 "$temporary"
mv "$temporary" "$OUTPUT"
trap - EXIT

if command -v sha256sum >/dev/null 2>&1; then
  sha256sum "$SOURCE" "$OUTPUT"
else
  shasum -a 256 "$SOURCE" "$OUTPUT"
fi
