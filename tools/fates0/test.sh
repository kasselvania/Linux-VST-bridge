#!/usr/bin/env bash

set -Eeuo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)

python3 -B -m unittest discover -s "$SCRIPT_DIR/tests" -p 'test_*.py'
python3 -B - "$SCRIPT_DIR" <<'PY'
import pathlib, sys
for source in pathlib.Path(sys.argv[1]).glob("*.py"):
    compile(source.read_text(), str(source), "exec")
PY
for script in "$SCRIPT_DIR"/*.sh; do bash -n "$script"; done
for document in \
  "$SCRIPT_DIR/pinned-inputs.json" \
  "$REPO_DIR/evidence/fates0/hardware-contract.json" \
  "$REPO_DIR/evidence/fates0/hardware-contract.schema.json" \
  "$REPO_DIR/evidence/fates0/kernel-driver-admission.json" \
  "$REPO_DIR/evidence/fates0/overlay-base-admission.json"; do
  python3 -m json.tool "$document" >/dev/null
done

rebuilt=$(mktemp "${TMPDIR:-/tmp}/fates0-rebuilt.XXXXXX.dtbo")
trap 'rm -f "$rebuilt"' EXIT
"$SCRIPT_DIR/build-dtbo.sh" "$rebuilt" >/dev/null
cmp "$rebuilt" "$SCRIPT_DIR/overlays/fates0.dtbo"

if [[ -n ${FATES0_BASE_DTB:-} || -n ${FATES0_PROTO_DTBO:-} ]]; then
  [[ -n ${FATES0_BASE_DTB:-} && -n ${FATES0_PROTO_DTBO:-} ]] || {
    printf 'both FATES0_BASE_DTB and FATES0_PROTO_DTBO are required\n' >&2
    exit 1
  }
  "$SCRIPT_DIR/verify-overlay-base.sh" "$FATES0_BASE_DTB" "$FATES0_PROTO_DTBO"
fi

if rg -n -g '*.sh' -g '*.py' -g '!test.sh' \
  '/home/we|chmod[[:space:]]+777|armhf|apt-get.*wine|apt-get.*box64|apt-get.*proton|i2cset' "$SCRIPT_DIR"; then
  printf 'prohibited historical or RPI0 behavior found in executable provisioning files\n' >&2
  exit 1
fi

if command -v shellcheck >/dev/null 2>&1; then
  shellcheck "$SCRIPT_DIR"/*.sh
fi
printf 'FATES0 local deterministic tests passed\n'
