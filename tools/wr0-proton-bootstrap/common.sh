#!/usr/bin/env bash
set -euo pipefail

wr0_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
wr0_repo_root="$(cd "$wr0_script_dir/../.." && pwd -P)"
wr0_python="$wr0_script_dir/launch.py"

wr0_require_repo() {
    local observed
    observed="$(git -C "$wr0_repo_root" rev-parse --show-toplevel)"
    [[ "$observed" == "$wr0_repo_root" ]] || {
        printf 'WR0_ERROR: repository root is not canonical\n' >&2
        return 1
    }
}

wr0_require_repo
