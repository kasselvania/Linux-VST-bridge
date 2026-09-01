#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
# shellcheck source=common.sh
source "$script_dir/common.sh"

exec python3 "$wr0_python" inspect-runner "$@"
