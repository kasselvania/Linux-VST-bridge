#!/usr/bin/env bash

set -Eeuo pipefail

image=${1:-}
[[ -f $image ]] || { printf 'usage: %s 2026-09-15-raspios-trixie-arm64-lite.img.xz\n' "$0" >&2; exit 2; }
expected=cdf4f3bfac35ae947b46e4e767f935453810549779ac3290e05a6754aee627e5
if command -v sha256sum >/dev/null 2>&1; then
  observed=$(sha256sum "$image" | awk '{print $1}')
else
  observed=$(shasum -a 256 "$image" | awk '{print $1}')
fi
[[ $observed == "$expected" ]] || {
  printf 'image digest mismatch\nexpected %s\nobserved %s\n' "$expected" "$observed" >&2
  exit 1
}
printf 'verified %s\n' "$expected"
