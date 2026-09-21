#!/usr/bin/env bash

set -Eeuo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
BASE_DTB=${1:-}
PROTO_DTBO=${2:-}
[[ -f $BASE_DTB && -f $PROTO_DTBO ]] || {
  printf 'usage: %s bcm2712-rpi-5-b.dtb proto-codec.dtbo\n' "$0" >&2
  exit 2
}
command -v fdtoverlay >/dev/null 2>&1 || { printf 'fdtoverlay is required\n' >&2; exit 1; }
command -v dtc >/dev/null 2>&1 || { printf 'dtc is required\n' >&2; exit 1; }
[[ $(dtc --version) == 'Version: DTC 1.7.2' ]] || {
  printf 'DTC 1.7.2 is required\n' >&2
  exit 1
}

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | awk '{print $1}';
  else shasum -a 256 "$1" | awk '{print $1}'; fi
}

[[ $(sha256_file "$BASE_DTB") == a5165cb562035f479ea9f5a81cedf7b6596ea32a0598608c4dc3b77ada909c60 ]] || {
  printf 'base Pi 5 DTB hash differs\n' >&2; exit 1;
}
[[ $(sha256_file "$PROTO_DTBO") == 1ec997e22f82880c2540cec822da3d96cebc0a1aebd8626481000ff8d140a754 ]] || {
  printf 'proto-codec DTBO hash differs\n' >&2; exit 1;
}

directory=$(mktemp -d "${TMPDIR:-/tmp}/fates0-overlay.XXXXXX")
trap 'rm -rf "$directory"' EXIT
fdtoverlay -i "$BASE_DTB" -o "$directory/combined.dtb" \
  "$PROTO_DTBO" "$SCRIPT_DIR/overlays/fates0.dtbo"
observed=$(sha256_file "$directory/combined.dtb")
expected=b0a89e0c247fa7a143b2db8a1121099d4666fa9da67d8102d4f5c1ad6443d765
[[ $observed == "$expected" ]] || {
  printf 'combined DTB hash differs\nexpected %s\nobserved %s\n' "$expected" "$observed" >&2
  exit 1
}
printf 'static overlay application verified %s\n' "$observed"
