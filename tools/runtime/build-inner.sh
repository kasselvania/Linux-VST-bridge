#!/usr/bin/env bash
set -euo pipefail
cat > Makefile <<'MAKEFILE'
SRCDIR := /work/proton
BUILD_NAME := lvb-source-comparison
include /work/proton/Makefile.in
MAKEFILE
target=${2:-wine}
case "$target" in wine|redist) ;; *) echo 'Unknown build target' >&2; exit 2 ;; esac
make -j3 J=3 CONTAINER=1 SOURCE_DATE_EPOCH="$1" BASE_SOURCE_DATE_EPOCH="$1" \
    WINEMONO_VER=10.4.1 "$target"
