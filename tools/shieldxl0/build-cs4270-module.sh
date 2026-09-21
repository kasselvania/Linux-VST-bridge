#!/usr/bin/env bash

set -Eeuo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=lib.sh
source "$SCRIPT_DIR/lib.sh"

OUTPUT=${1:-$PWD/snd-soc-cs4270.ko}
require_platform
[[ $(sha256_file "$SCRIPT_DIR/kernel/cs4270.c") == ec028f382ec93a795d5c6abaf40acb47e9c0ff5eccc0a1de934d581b49bc0029 ]] ||
  die 'vendored upstream CS4270 source hash differs from the pinned v6.18 input'
declare -A build_packages=(
  [binutils]='2.44-3'
  [build-essential]='12.12'
  [gcc]='4:14.2.0-1'
  ["$SHIELDXL0_HEADERS_PACKAGE"]="$SHIELDXL0_KERNEL_PACKAGE_VERSION"
  [make]='4.4.1-2'
)
for package in binutils build-essential gcc "$SHIELDXL0_HEADERS_PACKAGE" make; do
  observed=$(dpkg-query -W -f='${Version}' "$package" 2>/dev/null || true)
  [[ $observed == "${build_packages[$package]}" ]] ||
    die "build package drift for $package: expected ${build_packages[$package]}, observed ${observed:-absent}"
done
[[ -d /lib/modules/$SHIELDXL0_KERNEL/build ]] || die "kernel headers are missing for $SHIELDXL0_KERNEL"
command -v strip >/dev/null 2>&1 || die 'GNU strip is required for a deterministic module payload'

build_dir=$(mktemp -d /var/tmp/shieldxl0-cs4270.XXXXXX)
trap 'rm -rf "$build_dir"' EXIT
cp "$SCRIPT_DIR/kernel/cs4270.c" "$SCRIPT_DIR/kernel/Makefile" "$build_dir/"
make -C "/lib/modules/$SHIELDXL0_KERNEL/build" M="$build_dir" \
  KBUILD_BUILD_TIMESTAMP='2026-09-15T00:00:00Z' \
  KBUILD_BUILD_USER=shieldxl0 KBUILD_BUILD_HOST=reproducible \
  KCFLAGS="-fdebug-prefix-map=$build_dir=." modules
strip --strip-debug "$build_dir/snd-soc-cs4270.ko"
vermagic=$(modinfo -F vermagic "$build_dir/snd-soc-cs4270.ko")
[[ $vermagic == "$SHIELDXL0_KERNEL "* ]] || die "module vermagic mismatch: $vermagic"
install -m 0644 "$build_dir/snd-soc-cs4270.ko" "$OUTPUT"
sha256_file "$OUTPUT"
