#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
# shellcheck source=common.sh
source "$script_dir/common.sh"

usage() {
    printf 'usage: %s --session-id ID --build-dir PATH --preedit-proof PATH < exact-confirmation-line\n' "${0##*/}" >&2
}

session_id=""
build_dir=""
preedit_proof=""
while (($#)); do
    case "$1" in
        --session-id) session_id="${2:-}"; shift 2 ;;
        --build-dir) build_dir="${2:-}"; shift 2 ;;
        --preedit-proof) preedit_proof="${2:-}"; shift 2 ;;
        *) usage; exit 2 ;;
    esac
done
[[ -n "$session_id" && -n "$build_dir" && -n "$preedit_proof" ]] || { usage; exit 2; }

repo_root="$(hp1_repo_root)"
session_root="$(hp1_session_root "$session_id")"
hp1_require_build_dir "$build_dir"
hp1_require_preedit_proof "$preedit_proof"
hp1_assert_no_forbidden_process

IFS= read -r confirmation || hp1_die "exact operator confirmation was not supplied on stdin"
[[ -n "$confirmation" ]] || hp1_die "operator confirmation is empty"
umask 077
printf '%s\n' "$confirmation" >"$session_root/operator-confirmation.input"

"$script_dir/monitor-instance.py" finalize-session \
    --session-id "$session_id" \
    --confirmation-file "$session_root/operator-confirmation.input" \
    --repo-root "$repo_root" \
    --build-dir "$build_dir" \
    --preedit-proof "$preedit_proof"

printf 'session_collection=complete\n'
