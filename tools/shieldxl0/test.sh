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
python3 -m json.tool "$SCRIPT_DIR/pinned-inputs.json" >/dev/null
python3 -m json.tool "$REPO_DIR/evidence/shieldxl0/hardware-contract.json" >/dev/null
python3 -m json.tool "$REPO_DIR/evidence/shieldxl0/hardware-contract.schema.json" >/dev/null
python3 -m json.tool "$REPO_DIR/evidence/shieldxl0/kernel-driver-admission.json" >/dev/null
python3 -m json.tool "$REPO_DIR/evidence/shieldxl0/overlay-base-admission.json" >/dev/null
python3 -m json.tool "$REPO_DIR/evidence/shieldxl0/fixture-admission-failure.json" >/dev/null
python3 -m json.tool "$REPO_DIR/evidence/shieldxl0/platform-physical.json" >/dev/null
python3 -m json.tool "$REPO_DIR/evidence/shieldxl0/audio-jack-physical.json" >/dev/null
python3 -m json.tool "$REPO_DIR/evidence/shieldxl0/controls-physical.json" >/dev/null
python3 -m json.tool "$REPO_DIR/evidence/shieldxl0/midi-physical.json" >/dev/null

python3 - "$SCRIPT_DIR" "$REPO_DIR/evidence/shieldxl0/hardware-contract.json" <<'PY'
import hashlib, json, pathlib, sys
root = pathlib.Path(sys.argv[1])
contract = json.loads(pathlib.Path(sys.argv[2]).read_text())
manifest = json.loads((root / "pinned-inputs.json").read_text())
overlay = manifest["adapted_overlay"]
for key, path_key in (("source_sha256", "source"), ("binary_sha256", "binary")):
    path = root / overlay[path_key]
    observed = hashlib.sha256(path.read_bytes()).hexdigest()
    assert observed == overlay[key], (path, observed, overlay[key])
assert contract["acceptance_status"] == "accepted"
assert contract["audio"]["physical_loopback"] == "pass_both_channels"
assert contract["provisioning"]["physical_verification_completed"] is True
assert "thermally_unqualified" in contract["thermal"]["sustained_performance_classification"]
PY

if rg -n -g '*.sh' -g '*.py' -g '!test.sh' \
  '/home/we|chmod[[:space:]]+777|armhf|apt-get.*wine|apt-get.*box64|apt-get.*proton' "$SCRIPT_DIR"; then
  printf 'prohibited historical or RPI0 behavior found in executable provisioning files\n' >&2
  exit 1
fi

if command -v shellcheck >/dev/null 2>&1; then
  shellcheck "$SCRIPT_DIR"/*.sh
fi
printf 'SHIELDXL0 local deterministic tests passed\n'
