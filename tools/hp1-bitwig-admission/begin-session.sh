#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
# shellcheck source=common.sh
source "$script_dir/common.sh"

usage() {
    printf 'usage: %s --build-dir PATH --preedit-proof PATH\n' "${0##*/}" >&2
}

build_dir=""
preedit_proof=""
while (($#)); do
    case "$1" in
        --build-dir) build_dir="${2:-}"; shift 2 ;;
        --preedit-proof) preedit_proof="${2:-}"; shift 2 ;;
        *) usage; exit 2 ;;
    esac
done
[[ -n "$build_dir" && -n "$preedit_proof" ]] || { usage; exit 2; }

repo_root="$(hp1_repo_root)"
hp1_require_build_dir "$build_dir"
hp1_require_preedit_proof "$preedit_proof"
hp1_assert_no_forbidden_process

session_json="$($script_dir/monitor-instance.py create-session)"
session_id="$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["session_id"])' <<<"$session_json")"
nonce="$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["nonce"])' <<<"$session_json")"
session_root="$(hp1_session_root "$session_id")"

"$script_dir/preflight.sh" \
    --session-id "$session_id" \
    --build-dir "$build_dir" \
    --preedit-proof "$preedit_proof"

"$script_dir/negative-tests.sh" \
    --session-id "$session_id" \
    --fixture "$session_root/fixture.before.json"

hp1_assert_no_forbidden_process
umask 077
nohup "$script_dir/monitor-instance.py" monitor --session-id "$session_id" \
    >"$session_root/monitor.stdout" 2>&1 </dev/null &
monitor_pid=$!
printf '%s\n' "$monitor_pid" >"$session_root/monitor.pid"

monitor_ready=false
for _ in 1 2 3 4 5 6 7 8 9 10; do
    if [[ -f "$session_root/monitor.state.json" ]]; then
        status="$($script_dir/monitor-instance.py status --session-id "$session_id")"
        if grep -Fqx 'monitor_status=waiting_session_1' <<<"$status"; then
            monitor_ready=true
            break
        fi
    fi
    if ! kill -0 "$monitor_pid" 2>/dev/null; then
        sed -n '1,80p' "$session_root/monitor.stdout" >&2
        hp1_die "monitor exited before becoming ready"
    fi
    sleep 0.25
done
"$monitor_ready" || hp1_die "monitor did not reach waiting_session_1"

printf 'checkpoint=HP1_OPERATOR_ACTION_REQUIRED\n'
printf 'session_id=%s\n' "$session_id"
printf 'nonce=%s\n' "$nonce"
printf 'monitor_state=waiting_session_1\n'
printf 'monitor_status_command=%s status --session-id %s\n' "$script_dir/monitor-instance.py" "$session_id"
printf 'forbidden_processes=absent\n'
