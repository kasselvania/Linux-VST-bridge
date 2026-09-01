#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
# shellcheck source=common.sh
source "$script_dir/common.sh"

usage() {
    printf 'usage: %s --session-id ID --fixture PATH\n' "${0##*/}" >&2
}

session_id=""
fixture=""
while (($#)); do
    case "$1" in
        --session-id) session_id="${2:-}"; shift 2 ;;
        --fixture) fixture="${2:-}"; shift 2 ;;
        *) usage; exit 2 ;;
    esac
done
[[ -n "$session_id" && -n "$fixture" ]] || { usage; exit 2; }

monitor="$script_dir/monitor-instance.py"
session_root="$(hp1_session_root "$session_id")"
[[ "$fixture" == "$session_root/fixture.before.json" ]] || hp1_die "negative suite requires this session's exact fixture snapshot"
"$monitor" fixture-check --input "$fixture" >/dev/null

cache_root="$($monitor cache-root)"
test_root="$(mktemp -d "$cache_root/hp1-negative-tests.XXXXXX")"
outside_root="$(mktemp -d "$cache_root/hp1-negative-outside.XXXXXX")"
ledger="$test_root/ledger.tsv"
umask 077

cleanup() {
    rm -rf -- "$test_root" "$outside_root"
}
trap cleanup EXIT HUP INT TERM

record_pass() {
    printf '%s\tpassed\n' "$1" >>"$ledger"
    printf '%s=passed\n' "$1"
}

expect_failure() {
    local label="$1"
    local expected="$2"
    shift 2
    local log="$test_root/$label.log"
    if "$@" >"$log" 2>&1; then
        hp1_die "negative case unexpectedly succeeded: $label"
    fi
    grep -Fq "$expected" "$log" || {
        sed -n '1,80p' "$log" >&2
        hp1_die "negative case failed for the wrong reason: $label"
    }
}

mutate_and_refuse() {
    local label="$1"
    local field="$2"
    local value="$3"
    local expected="$4"
    local changed="$test_root/$label.json"
    "$monitor" fixture-mutate --input "$fixture" --output "$changed" --field "$field" --value "$value"
    expect_failure "$label" "$expected" "$monitor" fixture-check --input "$changed"
    record_pass "$label"
}

zeros="0000000000000000000000000000000000000000000000000000000000000000"

mutate_and_refuse wrong_hp0_module_hash hp0.publication.module.sha256 "$zeros" "fixture HP0 module hash differs"
mutate_and_refuse invalid_hp0_publication_receipt hp0.publication.receipt_status invalid "publication receipt is missing or invalid"
mutate_and_refuse wrong_bitwig_app_commit bitwig.app_commit "$zeros" "Bitwig fixture differs for app_commit"
mutate_and_refuse wrong_bitwig_runtime_commit bitwig.runtime_commit "$zeros" "Bitwig fixture differs for runtime_commit"
mutate_and_refuse changed_user_override bitwig.user_override_sha256 "$zeros" "Bitwig fixture differs for user_override_sha256"
mutate_and_refuse changed_system_override bitwig.system_override_sha256 "$zeros" "Bitwig fixture differs for system_override_sha256"

proc_bitwig="$test_root/proc-bitwig"
"$monitor" make-proc-fixture --root "$proc_bitwig" --case bitwig_running >/dev/null
expect_failure bitwig_already_running "Bitwig is already running" "$monitor" guard --proc-root "$proc_bitwig"
record_pass bitwig_already_running

proc_unrelated="$test_root/proc-unrelated"
module_unrelated="$($monitor make-proc-fixture --root "$proc_unrelated" --case unrelated_mapping)"
unrelated_result="$($monitor proc-evaluate --proc-root "$proc_unrelated" --module "$module_unrelated")"
grep -Fq '"classification": "rejected_mapping_without_bitwig_ancestry"' <<<"$unrelated_result" ||
    hp1_die "unrelated mapping was not rejected"
record_pass unrelated_mapping_rejected

proc_valid="$test_root/proc-valid"
module_valid="$($monitor make-proc-fixture --root "$proc_valid" --case valid_mapping)"
valid_result="$($monitor proc-evaluate --proc-root "$proc_valid" --module "$module_valid")"
grep -Fq '"classification": "accepted_bitwig_descendant_mapping"' <<<"$valid_result" ||
    hp1_die "valid Bitwig-descendant mapping was not accepted"
record_pass valid_descendant_mapping_accepted

proc_stale="$test_root/proc-stale"
module_stale="$($monitor make-proc-fixture --root "$proc_stale" --case stale)"
stale_result="$($monitor proc-evaluate --proc-root "$proc_stale" --module "$module_stale" --expected-root 90:111)"
grep -Fq '"classification": "rejected_stale_process_identity"' <<<"$stale_result" ||
    hp1_die "stale PID/start-time identity was not rejected"
record_pass stale_pid_rejected

proc_reused="$test_root/proc-reused"
module_reused="$($monitor make-proc-fixture --root "$proc_reused" --case reused)"
reused_result="$($monitor proc-evaluate --proc-root "$proc_reused" --module "$module_reused" --previous-root 90:111)"
grep -Fq '"classification": "rejected_reused_process_identity"' <<<"$reused_result" ||
    hp1_die "reused first-session identity was not rejected"
record_pass second_session_identity_reuse_rejected

proc_forbidden="$test_root/proc-forbidden"
module_forbidden="$($monitor make-proc-fixture --root "$proc_forbidden" --case forbidden)"
forbidden_result="$($monitor proc-evaluate --proc-root "$proc_forbidden" --module "$module_forbidden")"
grep -Fq '"classification": "blocked_forbidden_process"' <<<"$forbidden_result" ||
    hp1_die "forbidden yabridge process did not block the session"
record_pass forbidden_process_blocks

nonce_root="$test_root/nonce-session"
mkdir "$nonce_root"
cp "$session_root/session.json" "$nonce_root/session.json"
nonce="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1], encoding="utf-8"))["nonce"])' "$nonce_root/session.json")"
nonce_state="$nonce_root/monitor.state.json"
"$monitor" make-test-monitor-state --output "$nonce_state" --kind success
wrong_confirmation="$nonce_root/wrong-confirmation.txt"
printf 'HP1_GUI_CONFIRM 00000000000000000000000000000000 SESSION1_FOUND INSERTED REMOVED QUIT SESSION2_FOUND_WITHOUT_RESCAN INSERTED REMOVED QUIT GAIN_GENERIC_VIEW=OBSERVED BYPASS_GENERIC_VIEW=NOT_OBSERVED HOST_BYPASS_BINDING=UNKNOWN\n' >"$wrong_confirmation"
expect_failure wrong_nonce "wrong nonce" "$monitor" verify-confirmation \
    --test-session-root "$nonce_root" --confirmation-file "$wrong_confirmation" --test-state "$nonce_state"
good_confirmation="$nonce_root/good-confirmation.txt"
printf 'HP1\\_GUI\\_CONFIRM %s SESSION1\\_FOUND INSERTED REMOVED QUIT SESSION2\\_FOUND\\_WITHOUT\\_RESCAN INSERTED REMOVED QUIT GAIN\\_GENERIC\\_VIEW=OBSERVED BYPASS\\_GENERIC\\_VIEW=NOT\\_OBSERVED HOST\\_BYPASS\\_BINDING=UNKNOWN\n' "$nonce" >"$good_confirmation"
confirmation_preview="$("$monitor" verify-confirmation --test-session-root "$nonce_root" \
    --confirmation-file "$good_confirmation" --test-state "$nonce_state")"
grep -Fq '"confirmation_transport": "markdown_underscore_escaped"' <<<"$confirmation_preview" ||
    hp1_die "uniform Markdown underscore transport was not classified exactly"
mixed_confirmation="$nonce_root/mixed-confirmation.txt"
printf 'HP1\\_GUI_CONFIRM %s SESSION1_FOUND INSERTED REMOVED QUIT SESSION2_FOUND_WITHOUT_RESCAN INSERTED REMOVED QUIT GAIN_GENERIC_VIEW=OBSERVED BYPASS_GENERIC_VIEW=NOT_OBSERVED HOST_BYPASS_BINDING=UNKNOWN\n' "$nonce" >"$mixed_confirmation"
expect_failure mixed_confirmation_transport "mixes raw and Markdown-escaped underscores" "$monitor" verify-confirmation \
    --test-session-root "$nonce_root" --confirmation-file "$mixed_confirmation" --test-state "$nonce_state"
"$monitor" verify-confirmation --test-session-root "$nonce_root" \
    --confirmation-file "$good_confirmation" --test-state "$nonce_state" --consume >/dev/null
expect_failure reused_nonce "already consumed" "$monitor" verify-confirmation \
    --test-session-root "$nonce_root" --confirmation-file "$good_confirmation" --test-state "$nonce_state" --consume
record_pass wrong_or_reused_nonce_rejected

scan_root="$test_root/state-scan-root"
mkdir "$scan_root"
printf 'one\n' >"$scan_root/plugin-log-1"
printf 'two\n' >"$scan_root/plugin-log-2"
printf 'three\n' >"$scan_root/plugin-log-3"
scan_output="$test_root/state-cap.json"
"$monitor" scan-state --test-root "$test_root" --root "$scan_root" \
    --max-candidates 2 --output "$scan_output" >/dev/null
python3 -c 'import json,sys; d=json.load(open(sys.argv[1], encoding="utf-8")); assert d["status"] == "search_incomplete" and d["candidate_cap_exhausted"] is True' "$scan_output" ||
    hp1_die "candidate cap did not propagate to search_incomplete"
record_pass state_candidate_cap_propagates_unknown

printf 'escaped sentinel\n' >"$outside_root/sentinel"
ln -s "$outside_root" "$test_root/symlink-component"
expect_failure symlinked_evidence_ancestor "symlinked path component" "$monitor" path-check \
    --path "$test_root/symlink-component/evidence" --under-root "$test_root" --kind directory-or-absent
grep -Fxq 'escaped sentinel' "$outside_root/sentinel" || hp1_die "symlink escape target changed"
record_pass symlinked_session_or_evidence_ancestor_rejected

override_after="$test_root/override-after.json"
"$monitor" fixture-mutate --input "$fixture" --output "$override_after" \
    --field bitwig.user_override_bytes --value changed
expect_failure override_before_after_mismatch "before/after fixture preservation failed" "$monitor" fixture-compare \
    --before "$fixture" --after "$override_after"
record_pass override_before_after_mismatch_invalidates

serum_after="$test_root/serum-after.json"
"$monitor" fixture-mutate --input "$fixture" --output "$serum_after" \
    --field serum.0.mtime --value '2026-01-01 00:00:00.000000000 +0000'
expect_failure serum_identity_drift "Serum artifact identity differs" "$monitor" fixture-check --input "$serum_after"
record_pass serum_hash_size_mtime_drift_invalidates

[[ "$(wc -l <"$ledger")" -eq 17 ]] || hp1_die "negative-test ledger does not contain exactly 17 cases"
python3 - "$ledger" "$session_root/negative-tests.json" <<'PY'
import datetime as dt
import json
import pathlib
import sys

ledger = pathlib.Path(sys.argv[1])
output = pathlib.Path(sys.argv[2])
cases = []
for line in ledger.read_text(encoding="utf-8").splitlines():
    name, result = line.split("\t", 1)
    cases.append({"case": name, "result": result})
value = {
    "schema": "linux-vst-bridge-hp1-negative-tests/v1",
    "classification": "passed",
    "captured_at": dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
    "passed_count": len(cases),
    "cases": cases,
}
temporary = output.with_name("." + output.name + ".tmp")
temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
temporary.replace(output)
PY

printf 'negative_test_status=passed\n'
printf 'negative_test_count=17\n'
