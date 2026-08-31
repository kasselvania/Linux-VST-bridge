#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
# shellcheck source=common.sh
source "$script_dir/common.sh"

usage() {
    printf 'usage: %s --build-dir PATH [--bundle PATH] [--log PATH]\n' "${0##*/}" >&2
}

build_dir=""
bundle=""
log=""
while (($#)); do
    case "$1" in
        --build-dir) build_dir="${2:-}"; shift 2 ;;
        --bundle) bundle="${2:-}"; shift 2 ;;
        --log) log="${2:-}"; shift 2 ;;
        *) usage; exit 2 ;;
    esac
done
[[ -n "$build_dir" ]] || { usage; exit 2; }

repo_root="$(hp0_repo_root)"
sdk_root="$(hp0_default_sdk_root)"
[[ "$build_dir" == /* ]] || build_dir="$repo_root/$build_dir"
[[ -n "$bundle" ]] || bundle="$(hp0_find_bundle "$build_dir")"
[[ -n "$log" ]] || log="$build_dir/hp0-validator.log"

hp0_assert_no_bitwig
hp0_require_user_sdk
"$script_dir/verify-dependency.sh" "$sdk_root" >/dev/null
hp0_require_build_receipt "$build_dir"
[[ "$bundle" == "$HP0_VERIFIED_BUILD_BUNDLE" ]] ||
    hp0_die "validator bundle differs from the receipt-bound build bundle"
hp0_require_bundle_shape "$bundle"
validator="$HP0_VERIFIED_VALIDATOR"
module="$bundle/$HP0_MODULE_RELATIVE"

set +e
timeout --signal=TERM --kill-after=5s 180s \
    flatpak run --user --command="$validator" --unshare=network \
        --nofilesystem=host --nofilesystem=home --filesystem="$repo_root:ro" \
        "$HP0_SDK_RUN_REF" "$bundle" >"$log" 2>&1
validator_rc=$?
set -e
hp0_require_bounded_log "$log"
[[ "$validator_rc" -eq 0 ]] || hp0_die "official validator failed with exit $validator_rc; log=$log"

inspection_log="$build_dir/hp0-bundle-inspection.log"
hp0_sdk_run "$repo_root" "$sdk_root" sh -c \
    'file "$1"; readelf -h -d "$1"; ldd "$1"' hp0-inspect "$module" >"$inspection_log" 2>&1
hp0_require_bounded_log "$inspection_log" 1048576
grep -q 'ELF 64-bit.*x86-64' "$inspection_log" || hp0_die "module is not an x86_64 ELF shared object"
if grep -q 'not found' "$inspection_log"; then
    hp0_die "module has an unresolved dynamic dependency"
fi

printf 'validator_status=passed\n'
printf 'validator_exit=%s\n' "$validator_rc"
printf 'validator_sha256=%s\n' "$(sha256sum "$validator" | awk '{print $1}')"
printf 'module_sha256=%s\n' "$(sha256sum "$module" | awk '{print $1}')"
printf 'validator_log=%s\n' "$log"
printf 'inspection_log=%s\n' "$inspection_log"
