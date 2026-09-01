#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
# shellcheck source=common.sh
source "$script_dir/common.sh"

usage() {
    printf 'usage: %s --session-id ID\n' "${0##*/}" >&2
}

[[ "${1:-}" == --session-id && -n "${2:-}" && $# -eq 2 ]] || { usage; exit 2; }
session_id="$2"
repo_root="$(hp1_repo_root)"
hp1_session_root "$session_id" >/dev/null

"$script_dir/monitor-instance.py" render-evidence \
    --session-id "$session_id" \
    --output "$repo_root/evidence/hp1-bitwig-admission"

printf 'sanitization=passed\n'
