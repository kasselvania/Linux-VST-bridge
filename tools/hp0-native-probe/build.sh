#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
# shellcheck source=common.sh
source "$script_dir/common.sh"

usage() {
    printf 'usage: %s [--build-dir PATH] [--sdk-root PATH] [--check-only|--print-source-manifest|--verify-receipt]\n' "${0##*/}" >&2
}

build_dir=""
sdk_root="$(hp0_default_sdk_root)"
check_only=false
print_source_manifest=false
verify_receipt=false
while (($#)); do
    case "$1" in
        --build-dir)
            (($# >= 2)) || { usage; exit 2; }
            build_dir="$2"
            shift 2
            ;;
        --sdk-root)
            (($# >= 2)) || { usage; exit 2; }
            sdk_root="$2"
            shift 2
            ;;
        --check-only)
            check_only=true
            shift
            ;;
        --print-source-manifest)
            print_source_manifest=true
            shift
            ;;
        --verify-receipt)
            verify_receipt=true
            shift
            ;;
        *)
            usage
            exit 2
            ;;
    esac
done

mode_count=0
"$check_only" && ((mode_count += 1))
"$print_source_manifest" && ((mode_count += 1))
"$verify_receipt" && ((mode_count += 1))
((mode_count <= 1)) || hp0_die "select only one build inspection mode"

repo_root="$(hp0_repo_root)"
if "$print_source_manifest"; then
    manifest_tmp="$(mktemp "$(hp0_user_cache_root)/hp0-build-source.XXXXXX")"
    hp0_write_build_source_manifest "$repo_root" "$manifest_tmp"
    cat "$manifest_tmp"
    rm -f -- "$manifest_tmp"
    exit 0
fi

[[ -n "$build_dir" ]] || build_dir="$repo_root/build/hp0"
if [[ "$build_dir" != /* ]]; then
    build_dir="$repo_root/$build_dir"
fi
case "$build_dir" in
    "$repo_root"/build/*) ;;
    *) hp0_die "build directory must be beneath the repository's ignored build/ directory" ;;
esac

hp0_assert_no_bitwig
hp0_require_user_sdk
"$script_dir/verify-dependency.sh" "$sdk_root" >/dev/null

if "$verify_receipt"; then
    hp0_require_build_receipt "$build_dir"
    printf 'build_receipt_status=verified\n'
    printf 'historical_repository_commit=%s\n' "$HP0_VERIFIED_HISTORICAL_REPOSITORY_COMMIT"
    printf 'historical_repository_tree=%s\n' "$HP0_VERIFIED_HISTORICAL_REPOSITORY_TREE"
    printf 'current_repository_commit=%s\n' "$(git -C "$repo_root" rev-parse HEAD)"
    printf 'current_repository_tree=%s\n' "$(git -C "$repo_root" rev-parse 'HEAD^{tree}')"
    printf 'build_source_manifest_schema=%s\n' "$HP0_VERIFIED_BUILD_SOURCE_MANIFEST_SCHEMA"
    printf 'build_source_manifest_sha256=%s\n' "$HP0_VERIFIED_BUILD_SOURCE_MANIFEST_SHA256"
    exit 0
fi

if "$check_only"; then
    printf 'toolchain_status=verified\n'
    printf 'sdk_ref=runtime/%s\n' "$HP0_SDK_REF"
    printf 'sdk_commit=%s\n' "$HP0_SDK_COMMIT"
    exit 0
fi

if [[ -e "$build_dir" ]]; then
    [[ -d "$build_dir" && -z "$(find "$build_dir" -mindepth 1 -maxdepth 1 -print -quit)" ]] ||
        hp0_die "clean build directory must be absent or empty: $build_dir"
else
    mkdir -p "$build_dir"
fi

[[ -z "$(git -C "$repo_root" status --porcelain=v1 --untracked-files=all)" ]] ||
    hp0_die "repository must be clean before a retained build"

build_source_manifest="$build_dir/hp0-build-source.manifest"
hp0_write_build_source_manifest "$repo_root" "$build_source_manifest"
build_source_manifest_sha256="$(hp0_build_source_manifest_sha256 "$build_source_manifest")"

hp0_sdk_run "$repo_root" "$sdk_root" cmake \
    -S "$repo_root" -B "$build_dir" -G Ninja \
    -DCMAKE_BUILD_TYPE=Release -DVST3_SDK_ROOT="$sdk_root"
hp0_sdk_run "$repo_root" "$sdk_root" cmake --build "$build_dir" --target hp0-probe --parallel 4

bundle="$(hp0_find_bundle "$build_dir")"
validator="$(hp0_find_validator "$build_dir")"
hp0_require_bundle_shape "$bundle"
module="$bundle/$HP0_MODULE_RELATIVE"
manifest="$build_dir/hp0-built-bundle.sha256"
hp0_tree_manifest "$bundle" "$manifest"

cat >"$build_dir/hp0-build.receipt" <<EOF
schema=$HP0_BUILD_RECEIPT_SCHEMA
build_source_manifest_schema=$HP0_BUILD_SOURCE_MANIFEST_SCHEMA
build_source_manifest_sha256=$build_source_manifest_sha256
historical_repository_commit=$(git -C "$repo_root" rev-parse HEAD)
historical_repository_tree=$(git -C "$repo_root" rev-parse 'HEAD^{tree}')
sdk_ref=runtime/$HP0_SDK_REF
sdk_commit=$HP0_SDK_COMMIT
vst3_sdk_commit=$HP0_VST3_SDK_COMMIT
compiler=GCC 15.2.0
cmake=4.4.2
ninja=1.13.2
pkg_config=2.5.1
bundle=$bundle
module=$module
module_sha256=$(sha256sum "$module" | awk '{print $1}')
bundle_manifest_sha256=$(sha256sum "$manifest" | awk '{print $1}')
validator=$validator
validator_sha256=$(sha256sum "$validator" | awk '{print $1}')
processor_class_id=$HP0_PROCESSOR_CLASS_ID
controller_class_id=$HP0_CONTROLLER_CLASS_ID
gain_parameter_id=$HP0_GAIN_PARAMETER_ID
bypass_parameter_id=$HP0_BYPASS_PARAMETER_ID
EOF

printf 'build_status=passed\n'
cat "$build_dir/hp0-build.receipt"
