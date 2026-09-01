#!/usr/bin/env bash
set -euo pipefail

readonly HP1_EXPECTED_REPO="/home/deck/code/Linux-VST-bridge"
readonly HP1_EXPECTED_BRANCH="codex/hp1-bitwig-native-discovery-admission"
readonly HP1_EXPECTED_BASIS="7cda2d85eb426c2ed6e4eb3d86e52c114c4aa1c4"
readonly HP1_EXPECTED_TREE="e9534822625ceff0f06549b78c85546050bddf42"
readonly HP1_ACCEPTED_BUILD_DIR="$HP1_EXPECTED_REPO/build/hp0-second-repair-2"

hp1_die() {
    printf 'HP1_ERROR: %s\n' "$*" >&2
    exit 1
}

hp1_script_dir() {
    cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P
}

hp1_monitor() {
    printf '%s/monitor-instance.py\n' "$(hp1_script_dir)"
}

hp1_repo_root() {
    local root
    root="$(git rev-parse --show-toplevel 2>/dev/null)" || hp1_die "not inside the repository"
    [[ "$root" == "$HP1_EXPECTED_REPO" ]] ||
        hp1_die "repository path differs: expected $HP1_EXPECTED_REPO observed $root"
    printf '%s\n' "$root"
}

hp1_session_root() {
    local session_id="$1"
    "$(hp1_monitor)" resolve-session --session-id "$session_id"
}

hp1_require_build_dir() {
    local build_dir="$1"
    [[ "$build_dir" == "$HP1_ACCEPTED_BUILD_DIR" ]] ||
        hp1_die "build directory must equal the accepted HP0 build fixture"
    [[ -d "$build_dir" && ! -L "$build_dir" ]] ||
        hp1_die "accepted HP0 build directory is missing or unsafe"
}

hp1_require_preedit_proof() {
    local proof="$1"
    [[ "$proof" == /* && -d "$proof" && ! -L "$proof" ]] ||
        hp1_die "pre-edit proof must be an absolute non-symlink directory"
}

hp1_assert_no_forbidden_process() {
    "$(hp1_monitor)" guard --proc-root /proc >/dev/null
}
