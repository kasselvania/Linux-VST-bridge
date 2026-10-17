#!/usr/bin/env bash
set -euo pipefail
cat > Makefile <<'MAKEFILE'
SRCDIR := /work/proton
BUILD_NAME := lvb-source-comparison
include /work/proton/Makefile.in
MAKEFILE
make -j3 J=3 CONTAINER=1 SOURCE_DATE_EPOCH="$1" BASE_SOURCE_DATE_EPOCH="$1" wine
