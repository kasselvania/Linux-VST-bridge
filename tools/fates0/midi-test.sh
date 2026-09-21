#!/usr/bin/env bash

set -Eeuo pipefail

port=${1:-}
output=${2:-$PWD/fates0-midi-test}
[[ -n $port ]] || { printf 'usage: %s ALSA_CLIENT:PORT [output-directory]\n' "$0" >&2; exit 2; }
[[ ! -e $output ]] || { printf 'destination exists: %s\n' "$output" >&2; exit 1; }
mkdir -m 0700 -p "$output"
aconnect -l >"$output/alsa-sequencer-ports.txt"
jack_lsp -tp >"$output/jack-ports.txt" 2>&1 || true
printf 'Listening for 20 seconds on %s. Generate the explicitly requested physical events only.\n' "$port"
timeout --signal=INT 20s aseqdump -p "$port" >"$output/events.txt" 2>"$output/aseqdump.log" || status=$?
status=${status:-0}
[[ $status -eq 0 || $status -eq 124 || $status -eq 130 ]] || exit "$status"
python3 - "$output/events.txt" "$output/result.json" <<'PY'
import json, pathlib, re, sys
source, destination = map(pathlib.Path, sys.argv[1:])
text = source.read_text(errors="replace")
result = {
    "note_on": bool(re.search(r"Note on.*velocity\s+[1-9]", text, re.I)),
    "note_off": bool(re.search(r"Note off|Note on.*velocity\s+0", text, re.I)),
    "velocity": bool(re.search(r"velocity\s+([1-9][0-9]{0,2})", text, re.I)),
    "sustain_cc64": bool(re.search(r"Controller.*64", text, re.I)),
    "all_notes_off_cc123": bool(re.search(r"Controller.*123", text, re.I)),
}
result["pass_required"] = result["note_on"] and result["note_off"] and result["velocity"]
destination.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
raise SystemExit(0 if result["pass_required"] else 1)
PY
cat "$output/result.json"
