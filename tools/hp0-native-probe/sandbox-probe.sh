#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
# shellcheck source=common.sh
source "$script_dir/common.sh"

usage() {
    printf 'usage: %s --build-dir PATH [--publication-root PATH] [--expected-module-sha256 HEX] [--log PATH] [--test-mode --test-root PATH] [--test-app-commit HEX] [--test-runtime-commit HEX]\n' "${0##*/}" >&2
}

build_dir=""
publication_root="$(hp0_default_publication_root)"
expected_module_sha256=""
log=""
test_mode=false
test_root=""
test_app_commit=""
test_runtime_commit=""
while (($#)); do
    case "$1" in
        --build-dir) build_dir="${2:-}"; shift 2 ;;
        --publication-root) publication_root="${2:-}"; shift 2 ;;
        --expected-module-sha256) expected_module_sha256="${2:-}"; shift 2 ;;
        --log) log="${2:-}"; shift 2 ;;
        --test-mode) test_mode=true; shift ;;
        --test-root) test_root="${2:-}"; shift 2 ;;
        --test-app-commit) test_app_commit="${2:-}"; shift 2 ;;
        --test-runtime-commit) test_runtime_commit="${2:-}"; shift 2 ;;
        *) usage; exit 2 ;;
    esac
done
[[ -n "$build_dir" ]] || { usage; exit 2; }
if [[ -n "$test_app_commit" || -n "$test_runtime_commit" ]]; then
    "$test_mode" || hp0_die "synthetic fixture identities require --test-mode"
fi

repo_root="$(hp0_repo_root)"
[[ "$build_dir" == /* ]] || build_dir="$repo_root/$build_dir"
if "$test_mode"; then
    [[ -n "$test_root" ]] || hp0_die "test sandbox probe requires an explicit --test-root"
    test_root="$(hp0_require_test_root "$test_root")"
    publication_root="$(hp0_require_test_descendant "$test_root" "$publication_root" directory-or-absent "test publication root")"
    build_dir="$(hp0_require_test_descendant "$test_root" "$build_dir" existing-directory "test build root")"
else
    [[ -z "$test_root" ]] || hp0_die "--test-root requires --test-mode"
    [[ "$publication_root" == "$(hp0_default_publication_root)" ]] ||
        hp0_die "non-default publication root requires --test-mode"
    [[ "$build_dir" == "$repo_root"/build/* ]] ||
        hp0_die "production build receipt must be beneath the repository build directory"
fi

hp0_assert_no_bitwig
hp0_require_user_sdk
"$script_dir/verify-dependency.sh" "$(hp0_default_sdk_root)" >/dev/null
hp0_require_build_receipt "$build_dir"

bundle="$publication_root/$HP0_BUNDLE_NAME"
module="$bundle/$HP0_MODULE_RELATIVE"
validator="$HP0_VERIFIED_VALIDATOR"
receipt_module_sha256="$HP0_VERIFIED_BUILD_MODULE_SHA256"
validator_sha256="$HP0_VERIFIED_VALIDATOR_SHA256"
[[ -z "$expected_module_sha256" || "$expected_module_sha256" == "$receipt_module_sha256" ]] ||
    hp0_die "command-line module identity differs from the verified build receipt"
expected_module_sha256="$receipt_module_sha256"
[[ -n "$log" ]] || log="$build_dir/hp0-bitwig-sandbox.log"
if "$test_mode"; then
    log="$(hp0_require_test_descendant "$test_root" "$log" regular-file-or-absent "test sandbox log")"
fi

state_dir="$(mktemp -d "${XDG_CACHE_HOME:-$(hp0_real_home)/.cache}/hp0-sandbox-state.XXXXXX")"
cleanup_state() {
    [[ ! -d "${state_dir:-}" ]] || rm -rf -- "$state_dir"
    hp0_assert_no_bitwig
}
trap cleanup_state EXIT HUP INT TERM

hp0_capture_bitwig_fixture "$state_dir/fixture.before" "$test_app_commit" "$test_runtime_commit"
hp0_require_bitwig_fixture_snapshot "$state_dir/fixture.before"

inspect_arguments=(inspect --publication-root "$publication_root")
if "$test_mode"; then inspect_arguments+=(--test-mode --test-root "$test_root"); fi
"$script_dir/publish.sh" "${inspect_arguments[@]}" >/dev/null
observed_module_sha256="$(sha256sum "$module" | awk '{print $1}')"
[[ "$observed_module_sha256" == "$expected_module_sha256" ]] ||
    hp0_die "published module hash differs from the verified build receipt"
published_manifest_sha256="$(sha256sum "$bundle/$HP0_MANIFEST_RELATIVE" | awk '{print $1}')"
[[ "$published_manifest_sha256" == "$HP0_VERIFIED_BUILD_MANIFEST_SHA256" ]] ||
    hp0_die "published bundle manifest differs from the verified build receipt"
[[ "$(sha256sum "$validator" | awk '{print $1}')" == "$validator_sha256" ]] ||
    hp0_die "validator executable differs from the verified build receipt"

flatpak override --user --show "$HP0_BITWIG_APP_ID" >"$state_dir/user.before"
flatpak override --system --show "$HP0_BITWIG_APP_ID" >"$state_dir/system.before"
user_override_sha256="$(sha256sum "$state_dir/user.before" | awk '{print $1}')"
system_override_sha256="$(sha256sum "$state_dir/system.before" | awk '{print $1}')"
[[ "$user_override_sha256" == "$HP0_BITWIG_USER_OVERRIDE_SHA256" ]] ||
    hp0_die "user Flatpak override identity differs from the exact HP0 fixture"
[[ "$system_override_sha256" == "$HP0_BITWIG_SYSTEM_OVERRIDE_SHA256" ]] ||
    hp0_die "system Flatpak override identity differs from the exact HP0 fixture"

set +e
timeout --signal=TERM --kill-after=5s 180s \
    flatpak run --system --command=sh "$HP0_BITWIG_APP_ID" -eu -c '
        bundle=$1
        module=$2
        validator=$3
        expected_module_hash=$4
        expected_validator_hash=$5
        expected_vst_path=$6
        test -d "$bundle" && test ! -L "$bundle"
        test -r "$module" && test -f "$module" && test ! -L "$module"
        test -r "$validator" && test -f "$validator" && test ! -L "$validator" && test -x "$validator"
        observed_hash=$(sha256sum "$module" | awk "{print \$1}")
        observed_validator_hash=$(sha256sum "$validator" | awk "{print \$1}")
        test "$observed_hash" = "$expected_module_hash"
        test "$observed_validator_hash" = "$expected_validator_hash"
        machine=$(od -An -tx1 -j18 -N2 "$module" | tr -d " \n")
        test "$machine" = "3e00"
        if command -v file >/dev/null 2>&1; then file "$module"; else echo "file=not_installed_in_app_runtime"; fi
        ldd "$module"
        if ldd "$module" | grep -q "not found"; then exit 71; fi
        test "${VST_PATH+x}" = x && test "$VST_PATH" = "$expected_vst_path"
        test "${VST3_PATH+x}" = x && test -z "$VST3_PATH"
        test "${CLAP_PATH+x}" = x && test -z "$CLAP_PATH"
        printf "sandbox_bundle_visible=true\n"
        printf "sandbox_module_readable=true\n"
        printf "sandbox_module_sha256=%s\n" "$observed_hash"
        printf "sandbox_validator_sha256=%s\n" "$observed_validator_hash"
        printf "sandbox_elf_machine=x86_64\n"
        printf "sandbox_vst_path=%s\n" "$VST_PATH"
        printf "sandbox_vst3_path=empty\n"
        printf "sandbox_clap_path=empty\n"
        exec "$validator" "$bundle"
    ' hp0-bitwig-sandbox "$bundle" "$module" "$validator" "$expected_module_sha256" \
      "$validator_sha256" "$HP0_BITWIG_VST_PATH" >"$log" 2>&1
sandbox_rc=$?
set -e
hp0_require_bounded_log "$log"

flatpak override --user --show "$HP0_BITWIG_APP_ID" >"$state_dir/user.after"
flatpak override --system --show "$HP0_BITWIG_APP_ID" >"$state_dir/system.after"
hp0_capture_bitwig_fixture "$state_dir/fixture.after"
hp0_require_bitwig_fixture_snapshot "$state_dir/fixture.after"

cmp -s "$state_dir/fixture.before" "$state_dir/fixture.after" ||
    hp0_die "Bitwig app or runtime fixture identity changed during the sandbox probe"
cmp -s "$state_dir/user.before" "$state_dir/user.after" || hp0_die "user Flatpak override changed during sandbox probe"
cmp -s "$state_dir/system.before" "$state_dir/system.after" || hp0_die "system Flatpak override changed during sandbox probe"
[[ "$(sha256sum "$state_dir/user.after" | awk '{print $1}')" == "$HP0_BITWIG_USER_OVERRIDE_SHA256" ]] ||
    hp0_die "user Flatpak override no longer matches the exact HP0 fixture"
[[ "$(sha256sum "$state_dir/system.after" | awk '{print $1}')" == "$HP0_BITWIG_SYSTEM_OVERRIDE_SHA256" ]] ||
    hp0_die "system Flatpak override no longer matches the exact HP0 fixture"
[[ "$sandbox_rc" -eq 0 ]] || hp0_die "Bitwig app-sandbox validator failed with exit $sandbox_rc; log=$log"

grep -q '^sandbox_bundle_visible=true$' "$log" || hp0_die "sandbox did not prove bundle visibility"
grep -q '^sandbox_elf_machine=x86_64$' "$log" || hp0_die "sandbox did not prove x86_64 ELF architecture"
grep -Fqx "sandbox_validator_sha256=$validator_sha256" "$log" || hp0_die "sandbox validator identity was not retained"
grep -Fqx "sandbox_vst_path=$HP0_BITWIG_VST_PATH" "$log" || hp0_die "sandbox did not retain the exact effective VST_PATH"
grep -q '^sandbox_vst3_path=empty$' "$log" || hp0_die "sandbox did not retain empty VST3_PATH"
grep -q '^sandbox_clap_path=empty$' "$log" || hp0_die "sandbox did not retain empty CLAP_PATH"

printf 'sandbox_status=passed\n'
printf 'sandbox_validator_exit=%s\n' "$sandbox_rc"
printf 'build_receipt_commit=%s\n' "$HP0_VERIFIED_REPOSITORY_COMMIT"
printf 'build_receipt_tree=%s\n' "$HP0_VERIFIED_REPOSITORY_TREE"
printf 'build_source_manifest_schema=%s\n' "$HP0_VERIFIED_BUILD_SOURCE_MANIFEST_SCHEMA"
printf 'build_source_manifest_sha256=%s\n' "$HP0_VERIFIED_BUILD_SOURCE_MANIFEST_SHA256"
printf 'published_module_sha256=%s\n' "$observed_module_sha256"
printf 'published_manifest_sha256=%s\n' "$published_manifest_sha256"
printf 'sandbox_validator_sha256=%s\n' "$validator_sha256"
printf 'bitwig_app_commit=%s\n' "$HP0_BITWIG_APP_COMMIT"
printf 'bitwig_runtime_commit=%s\n' "$HP0_BITWIG_RUNTIME_COMMIT"
printf 'bitwig_scope=system\n'
printf 'effective_vst_path=%s\n' "$HP0_BITWIG_VST_PATH"
printf 'effective_vst3_path=empty\n'
printf 'effective_clap_path=empty\n'
printf 'user_override_sha256=%s\n' "$user_override_sha256"
printf 'system_override_sha256=%s\n' "$system_override_sha256"
printf 'fixture_before_after=byte_identical\n'
printf 'override_before_after=byte_identical\n'
printf 'sandbox_log=%s\n' "$log"

cleanup_state
trap - EXIT HUP INT TERM
hp0_assert_no_bitwig
