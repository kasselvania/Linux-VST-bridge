#!/usr/bin/env bash
set -euo pipefail

readonly HP0_SDK_REF="org.freedesktop.Sdk/x86_64/25.08"
readonly HP0_SDK_RUN_REF="org.freedesktop.Sdk//25.08"
readonly HP0_SDK_COMMIT="b90ed309cc1d505dea48b6a2121c5dcfac22868120eee643b0596d31f96b9bb8"
readonly HP0_VST3_SDK_COMMIT="3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96"
readonly HP0_PROCESSOR_CLASS_ID="6F4E7A5392E54B54A98AD6F714E0C201"
readonly HP0_CONTROLLER_CLASS_ID="B9C42F0736C34E218E5A71D40C8F1B62"
readonly HP0_GAIN_PARAMETER_ID="0x4C485001"
readonly HP0_BYPASS_PARAMETER_ID="0x4C485002"
readonly HP0_BUNDLE_NAME="LabHostProbe.vst3"
readonly HP0_MODULE_RELATIVE="Contents/x86_64-linux/LabHostProbe.so"
readonly HP0_MODULEINFO_RELATIVE="Contents/Resources/moduleinfo.json"
readonly HP0_OWNER_RELATIVE="Contents/Resources/.linux-vst-bridge-hp0-owner"
readonly HP0_MANIFEST_RELATIVE="Contents/Resources/.linux-vst-bridge-hp0-source.sha256"
readonly HP0_BITWIG_APP_ID="com.bitwig.BitwigStudio"
readonly HP0_BITWIG_APP_REF="app/com.bitwig.BitwigStudio/x86_64/stable"
readonly HP0_BITWIG_APP_VERSION="6.0.11"
readonly HP0_BITWIG_APP_COMMIT="7b66aed37386ffff99e40bbd486f90ceee7ac1d5c59508c7952094122cebf50e"
readonly HP0_BITWIG_RUNTIME_REF="org.freedesktop.Platform/x86_64/25.08"
readonly HP0_BITWIG_RUNTIME_COMMIT="bd44a6230581917d04f89812a4c21090c304d390edb73995af1c2f9fd8abf4e8"
readonly HP0_BITWIG_USER_OVERRIDE_SHA256="1b4a6a6ed688f69dd3c36dcac8db008c5a41ed52170ea3e23dee984b0aae6a1e"
readonly HP0_BITWIG_SYSTEM_OVERRIDE_SHA256="e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
readonly HP0_BITWIG_VST_PATH="/app/extensions/Plugins/vst;=/usr/lib/"
readonly HP0_BUILD_SOURCE_MANIFEST_SCHEMA="linux-vst-bridge-hp0-build-source/v1"
readonly HP0_BUILD_RECEIPT_SCHEMA="linux-vst-bridge-hp0-build/v2"
readonly HP0_PUBLICATION_RECEIPT_SCHEMA="linux-vst-bridge-hp0-publication/v2"

hp0_die() {
    printf 'HP0_ERROR: %s\n' "$*" >&2
    exit 1
}

hp0_repo_root() {
    git rev-parse --show-toplevel 2>/dev/null || hp0_die "not inside the HP0 repository"
}

hp0_default_sdk_root() {
    printf '%s/.cache/linux-vst-bridge/dependencies/vst3sdk/%s\n' "$(hp0_real_home)" "$HP0_VST3_SDK_COMMIT"
}

hp0_real_home() {
    local real_home
    real_home="$(getent passwd "$(id -u)" | cut -d: -f6)"
    [[ -n "$real_home" && "$real_home" == /* && -d "$real_home" ]] ||
        hp0_die "cannot resolve the real user home"
    printf '%s\n' "$real_home"
}

hp0_default_publication_root() {
    printf '%s/.vst3/linux-vst-bridge\n' "$(hp0_real_home)"
}

hp0_require_absolute_unambiguous_path() {
    local path="$1"
    local label="${2:-path}"
    local relative component
    local -a components=()

    [[ "$path" == /* && "$path" != / && "$path" != */ ]] ||
        hp0_die "$label must be an absolute path without a trailing slash"
    [[ "$path" != *//* ]] || hp0_die "$label contains an empty path component"
    relative="${path#/}"
    IFS='/' read -r -a components <<<"$relative"
    for component in "${components[@]}"; do
        [[ -n "$component" && "$component" != . && "$component" != .. ]] ||
            hp0_die "$label contains an unsafe path component"
    done
}

hp0_require_no_symlink_ancestors() {
    local path="$1"
    local final_kind="${2:-any}"
    local label="${3:-path}"
    local relative current component index
    local -a components=()

    hp0_require_absolute_unambiguous_path "$path" "$label"
    relative="${path#/}"
    IFS='/' read -r -a components <<<"$relative"
    current=""
    for index in "${!components[@]}"; do
        component="${components[$index]}"
        current="$current/$component"
        [[ ! -L "$current" ]] || hp0_die "$label has a symlinked path component"
        if [[ -e "$current" && "$index" -lt $((${#components[@]} - 1)) ]]; then
            [[ -d "$current" ]] || hp0_die "$label has a non-directory ancestor"
        fi
    done

    case "$final_kind" in
        any) ;;
        directory-or-absent)
            [[ ! -e "$path" || -d "$path" ]] || hp0_die "$label must be a directory or absent"
            ;;
        existing-directory)
            [[ -d "$path" ]] || hp0_die "$label must be an existing directory"
            ;;
        regular-file-or-absent)
            [[ ! -e "$path" || -f "$path" ]] || hp0_die "$label must be a regular file or absent"
            ;;
        *) hp0_die "unsupported path-kind guard: $final_kind" ;;
    esac
}

hp0_user_cache_root() {
    local real_home cache_root cache_real
    real_home="$(hp0_real_home)"
    cache_root="${XDG_CACHE_HOME:-$real_home/.cache}"
    hp0_require_no_symlink_ancestors "$cache_root" existing-directory "user cache root"
    cache_real="$(realpath -e -- "$cache_root")"
    [[ "$cache_real" == "$cache_root" ]] || hp0_die "user cache root is not canonical"
    printf '%s\n' "$cache_real"
}

hp0_default_receipt_path() {
    printf '%s/linux-vst-bridge/hp0-publication.receipt\n' "$(hp0_user_cache_root)"
}

hp0_require_test_root() {
    local test_root="$1"
    local cache_root test_real
    cache_root="$(hp0_user_cache_root)"
    hp0_require_no_symlink_ancestors "$test_root" existing-directory "test root"
    test_real="$(realpath -e -- "$test_root")"
    [[ "$test_real" == "$test_root" ]] || hp0_die "test root is not canonical"
    case "$test_real" in
        "$cache_root"/*) ;;
        *) hp0_die "test root must be a canonical descendant of the user cache" ;;
    esac
    printf '%s\n' "$test_real"
}

hp0_require_test_descendant() {
    local test_root="$1"
    local path="$2"
    local final_kind="${3:-any}"
    local label="${4:-test path}"
    local test_real path_real
    test_real="$(hp0_require_test_root "$test_root")"
    hp0_require_no_symlink_ancestors "$path" "$final_kind" "$label"
    path_real="$(realpath -m -- "$path")"
    [[ "$path_real" == "$path" ]] || hp0_die "$label is not canonical"
    case "$path_real" in
        "$test_real"/*) ;;
        *) hp0_die "$label must be a canonical descendant of the exact test root" ;;
    esac
    printf '%s\n' "$path_real"
}

hp0_prepare_test_directory() {
    local test_root="$1"
    local directory="$2"
    local label="${3:-test directory}"
    local test_real relative current component
    local -a components=()

    test_real="$(hp0_require_test_root "$test_root")"
    hp0_require_test_descendant "$test_real" "$directory" directory-or-absent "$label" >/dev/null
    relative="${directory#"$test_real"/}"
    current="$test_real"
    IFS='/' read -r -a components <<<"$relative"
    for component in "${components[@]}"; do
        current="$current/$component"
        [[ ! -L "$current" ]] || hp0_die "$label has a symlinked path component"
        if [[ -e "$current" ]]; then
            [[ -d "$current" ]] || hp0_die "$label has a non-directory component"
        else
            mkdir -- "$current"
        fi
    done
    hp0_require_test_descendant "$test_real" "$directory" existing-directory "$label" >/dev/null
}

hp0_prepare_test_file_parent() {
    local test_root="$1"
    local path="$2"
    local label="${3:-test file}"
    local test_real parent
    test_real="$(hp0_require_test_root "$test_root")"
    hp0_require_test_descendant "$test_real" "$path" regular-file-or-absent "$label" >/dev/null
    parent="$(dirname -- "$path")"
    if [[ "$parent" != "$test_real" ]]; then
        hp0_prepare_test_directory "$test_real" "$parent" "$label parent"
    fi
    hp0_require_test_descendant "$test_real" "$path" regular-file-or-absent "$label" >/dev/null
}

hp0_prepare_normal_receipt_path() {
    local receipt="$1"
    local expected cache_root project_root
    expected="$(hp0_default_receipt_path)"
    [[ "$receipt" == "$expected" ]] ||
        hp0_die "ordinary receipt path must equal the declared HP0 project receipt path"
    cache_root="$(hp0_user_cache_root)"
    project_root="$cache_root/linux-vst-bridge"
    hp0_require_no_symlink_ancestors "$project_root" directory-or-absent "HP0 project cache"
    if [[ ! -e "$project_root" ]]; then
        mkdir -- "$project_root"
    fi
    hp0_require_no_symlink_ancestors "$project_root" existing-directory "HP0 project cache"
    hp0_require_no_symlink_ancestors "$receipt" regular-file-or-absent "HP0 publication receipt"
}

hp0_assert_no_bitwig() {
    if pgrep -x BitwigStudio >/dev/null 2>&1 || pgrep -x bitwig-studio >/dev/null 2>&1; then
        hp0_die "Bitwig process is running"
    fi
}

hp0_require_user_sdk() {
    command -v flatpak >/dev/null 2>&1 || hp0_die "flatpak command is not installed"
    local observed_commit
    if ! observed_commit="$(flatpak info --user --show-commit "$HP0_SDK_REF" 2>/dev/null)"; then
        hp0_die "required user-scope Flatpak SDK is missing: runtime/$HP0_SDK_REF"
    fi
    [[ "$observed_commit" == "$HP0_SDK_COMMIT" ]] ||
        hp0_die "wrong user-scope Flatpak SDK commit: expected $HP0_SDK_COMMIT observed $observed_commit"
}

hp0_sdk_run() {
    local repo_root="$1"
    local sdk_root="$2"
    shift 2
    flatpak run --user --command=sh --unshare=network \
        --nofilesystem=host --nofilesystem=home \
        --filesystem="$repo_root" --filesystem="$sdk_root:ro" \
        "$HP0_SDK_RUN_REF" -c 'exec "$@"' hp0-sdk "$@"
}

hp0_find_bundle() {
    local build_dir="$1"
    local -a bundles=()
    mapfile -d '' bundles < <(find "$build_dir" -mindepth 1 -maxdepth 7 -type d \
        -name "$HP0_BUNDLE_NAME" -print0)
    [[ "${#bundles[@]}" -eq 1 ]] ||
        hp0_die "expected exactly one $HP0_BUNDLE_NAME under the bounded build directory; observed ${#bundles[@]}"
    printf '%s\n' "${bundles[0]}"
}

hp0_find_validator() {
    local build_dir="$1"
    local -a validators=()
    mapfile -d '' validators < <(find "$build_dir" -mindepth 1 -maxdepth 8 -type f \
        -path '*/bin/Release/validator' -perm -u+x -print0)
    [[ "${#validators[@]}" -eq 1 ]] ||
        hp0_die "expected exactly one official validator under the bounded build directory; observed ${#validators[@]}"
    printf '%s\n' "${validators[0]}"
}

hp0_require_bundle_shape() {
    local bundle="$1"
    [[ -d "$bundle" && ! -L "$bundle" ]] || hp0_die "bundle is missing or is a symlink: $bundle"
    [[ "$(basename "$bundle")" == "$HP0_BUNDLE_NAME" ]] || hp0_die "unexpected bundle name"
    local module="$bundle/$HP0_MODULE_RELATIVE"
    [[ -f "$module" && ! -L "$module" ]] || hp0_die "expected regular module is missing: $HP0_MODULE_RELATIVE"
    local moduleinfo="$bundle/$HP0_MODULEINFO_RELATIVE"
    [[ -f "$moduleinfo" && ! -L "$moduleinfo" ]] ||
        hp0_die "official module metadata is missing: $HP0_MODULEINFO_RELATIVE"
    [[ "$(grep -Fc "\"CID\": \"$HP0_PROCESSOR_CLASS_ID\"" "$moduleinfo")" -eq 1 ]] ||
        hp0_die "module metadata does not contain the one owned processor class ID"
    [[ "$(grep -Fc "\"CID\": \"$HP0_CONTROLLER_CLASS_ID\"" "$moduleinfo")" -eq 1 ]] ||
        hp0_die "module metadata does not contain the one owned controller class ID"
    grep -Fq '"Name": "LAB Host Probe"' "$moduleinfo" ||
        hp0_die "module metadata does not identify LAB Host Probe"
    grep -Fq '"Vendor": "Kasselvania Research"' "$moduleinfo" ||
        hp0_die "module metadata does not identify the owned vendor"
    if find "$bundle" -mindepth 1 -maxdepth 8 -type l -print -quit | grep -q .; then
        hp0_die "bundle contains a symlink"
    fi
}

hp0_tree_manifest() {
    local bundle="$1"
    local output="$2"
    hp0_require_bundle_shape "$bundle"
    (
        cd "$bundle"
        find . -mindepth 1 -maxdepth 8 -type f -print0 | LC_ALL=C sort -z |
            xargs -0 -r sha256sum
    ) >"$output"
    [[ -s "$output" ]] || hp0_die "bundle manifest is empty"
}

hp0_source_tree_manifest() {
    local bundle="$1"
    local output="$2"
    hp0_require_bundle_shape "$bundle"
    (
        cd "$bundle"
        find . -mindepth 1 -maxdepth 8 -type f \
            ! -path "./$HP0_OWNER_RELATIVE" \
            ! -path "./$HP0_MANIFEST_RELATIVE" -print0 |
            LC_ALL=C sort -z | xargs -0 -r sha256sum
    ) >"$output"
    [[ -s "$output" ]] || hp0_die "source bundle manifest is empty"
}

hp0_expected_build_source_paths() {
    cat <<'EOF'
CMakeLists.txt
cmake/HP0ModernGcc.cmake
cmake/HP0ToolchainGuard.cmake
cmake/HP0Vst3SdkLock.cmake
docs/HP0_DEPENDENCY_LOCK.md
native-probe/CMakeLists.txt
native-probe/README.md
native-probe/include/lab_host_probe/ids.h
native-probe/include/lab_host_probe/state.h
native-probe/source/controller.cpp
native-probe/source/controller.h
native-probe/source/factory.cpp
native-probe/source/processor.cpp
native-probe/source/processor.h
native-probe/source/version.h
tools/hp0-native-probe/build.sh
tools/hp0-native-probe/common.sh
tools/hp0-native-probe/validate.sh
tools/hp0-native-probe/verify-dependency.sh
EOF
}

hp0_write_build_source_manifest() {
    local repo_root="$1"
    local output="$2"
    local repo_real top_real expected_paths actual_directory_paths expected_directory_paths
    local path mode blob stage entry_path worktree_blob
    local temporary
    local -a stage_entries=()

    hp0_require_no_symlink_ancestors "$repo_root" existing-directory "repository root"
    repo_real="$(realpath -e -- "$repo_root")"
    top_real="$(realpath -e -- "$(git -C "$repo_root" rev-parse --show-toplevel 2>/dev/null)")"
    [[ "$repo_real" == "$top_real" ]] || hp0_die "build-source repository root is not the Git top level"

    expected_paths="$(hp0_expected_build_source_paths)"
    [[ -n "$expected_paths" ]] || hp0_die "build-source path set is empty"
    [[ "$expected_paths" == "$(printf '%s\n' "$expected_paths" | LC_ALL=C sort -u)" ]] ||
        hp0_die "build-source path set is not unique and sorted"

    actual_directory_paths="$(git -C "$repo_root" ls-files -- cmake native-probe | LC_ALL=C sort)"
    expected_directory_paths="$(printf '%s\n' "$expected_paths" | sed -n '/^cmake\//p; /^native-probe\//p')"
    [[ "$actual_directory_paths" == "$expected_directory_paths" ]] ||
        hp0_die "unexpected or missing tracked path beneath a declared build-source directory"

    temporary="$(mktemp "$(dirname -- "$output")/.hp0-build-source-manifest.XXXXXX")"
    {
        printf 'schema=%s\n' "$HP0_BUILD_SOURCE_MANIFEST_SCHEMA"
        while IFS= read -r path; do
            [[ -n "$path" ]] || hp0_die "build-source path set contains an empty path"
            mapfile -t stage_entries < <(git -C "$repo_root" ls-files --stage -- "$path")
            [[ "${#stage_entries[@]}" -eq 1 ]] ||
                hp0_die "expected build-source path is missing, untracked, duplicated, or unmerged: $path"
            read -r mode blob stage entry_path <<<"${stage_entries[0]}"
            [[ "$stage" == 0 && "$entry_path" == "$path" ]] ||
                hp0_die "build-source index entry is malformed or unmerged: $path"
            case "$mode" in
                100644|100755) ;;
                *) hp0_die "build-source path is not a supported regular tracked file: $path" ;;
            esac
            [[ -f "$repo_root/$path" && ! -L "$repo_root/$path" ]] ||
                hp0_die "build-source path is missing, non-regular, or a symlink: $path"
            git -C "$repo_root" diff --quiet -- "$path" ||
                hp0_die "build-source path has an unstaged change: $path"
            git -C "$repo_root" diff --cached --quiet -- "$path" ||
                hp0_die "build-source path has a staged change: $path"
            worktree_blob="$(git -C "$repo_root" hash-object -- "$path")"
            [[ "$worktree_blob" == "$blob" ]] ||
                hp0_die "build-source content differs from its tracked blob: $path"
            printf '%s\t%s\t%s\n' "$mode" "$blob" "$path"
        done <<<"$expected_paths"
    } >"$temporary"
    mv -T -- "$temporary" "$output"
}

hp0_build_source_manifest_sha256() {
    local manifest="$1"
    [[ -f "$manifest" && ! -L "$manifest" ]] || hp0_die "build-source manifest is missing or is a symlink"
    [[ "$(sed -n '1p' "$manifest")" == "schema=$HP0_BUILD_SOURCE_MANIFEST_SCHEMA" ]] ||
        hp0_die "build-source manifest schema differs"
    [[ "$(tail -n +2 "$manifest" | wc -l)" -eq 19 ]] || hp0_die "build-source manifest path count differs"
    sha256sum "$manifest" | awk '{print $1}'
}

hp0_read_kv_value() {
    local file="$1"
    local key="$2"
    local count value
    [[ -f "$file" && ! -L "$file" ]] || hp0_die "identity receipt is missing or is a symlink: $file"
    count="$(awk -F= -v key="$key" '$1 == key { count += 1 } END { print count + 0 }' "$file")"
    [[ "$count" == 1 ]] || hp0_die "identity receipt must contain exactly one $key field"
    value="$(awk -v prefix="$key=" 'index($0, prefix) == 1 { print substr($0, length(prefix) + 1) }' "$file")"
    printf '%s\n' "$value"
}

hp0_require_build_receipt() {
    local build_dir="$1"
    local repo_root receipt expected_keys observed_keys
    local historical_commit historical_tree receipt_bundle receipt_module receipt_validator
    local receipt_module_sha receipt_manifest_sha receipt_validator_sha
    local receipt_source_schema receipt_source_sha built_source_manifest recomputed_source_manifest
    local discovered_bundle discovered_validator built_manifest recomputed_manifest

    repo_root="$(hp0_repo_root)"
    [[ "$build_dir" == /* && -d "$build_dir" && ! -L "$build_dir" ]] ||
        hp0_die "build receipt directory must be an existing absolute non-symlink directory"
    receipt="$build_dir/hp0-build.receipt"
    [[ -f "$receipt" && ! -L "$receipt" ]] || hp0_die "verified HP0 build receipt is missing"
    if ! awk 'index($0, "=") == 0 { exit 1 }' "$receipt"; then
        hp0_die "HP0 build receipt contains a malformed line"
    fi

    expected_keys=$'build_source_manifest_schema\nbuild_source_manifest_sha256\nbundle\nbundle_manifest_sha256\nbypass_parameter_id\ncmake\ncompiler\ncontroller_class_id\ngain_parameter_id\nhistorical_repository_commit\nhistorical_repository_tree\nmodule\nmodule_sha256\nninja\npkg_config\nprocessor_class_id\nschema\nsdk_commit\nsdk_ref\nvalidator\nvalidator_sha256\nvst3_sdk_commit'
    observed_keys="$(cut -d= -f1 "$receipt" | LC_ALL=C sort)"
    [[ "$observed_keys" == "$expected_keys" ]] || hp0_die "HP0 build receipt key set differs from the v2 schema"

    [[ "$(hp0_read_kv_value "$receipt" schema)" == "$HP0_BUILD_RECEIPT_SCHEMA" ]] ||
        hp0_die "unknown HP0 build receipt schema"
    historical_commit="$(hp0_read_kv_value "$receipt" historical_repository_commit)"
    historical_tree="$(hp0_read_kv_value "$receipt" historical_repository_tree)"
    [[ "$historical_commit" =~ ^[0-9a-f]{40}$ && "$historical_tree" =~ ^[0-9a-f]{40}$ ]] ||
        hp0_die "build receipt historical repository provenance is malformed"
    [[ -z "$(git -C "$repo_root" status --porcelain=v1 --untracked-files=all)" ]] ||
        hp0_die "repository must be clean while binding the HP0 build receipt"

    receipt_source_schema="$(hp0_read_kv_value "$receipt" build_source_manifest_schema)"
    receipt_source_sha="$(hp0_read_kv_value "$receipt" build_source_manifest_sha256)"
    [[ "$receipt_source_schema" == "$HP0_BUILD_SOURCE_MANIFEST_SCHEMA" ]] ||
        hp0_die "build receipt names an unknown build-source manifest schema"
    [[ "$receipt_source_sha" =~ ^[0-9a-f]{64}$ ]] ||
        hp0_die "build receipt contains a malformed build-source manifest SHA-256"
    built_source_manifest="$build_dir/hp0-build-source.manifest"
    [[ -f "$built_source_manifest" && ! -L "$built_source_manifest" ]] ||
        hp0_die "receipt-bound build-source manifest is missing or is a symlink"
    [[ "$(hp0_build_source_manifest_sha256 "$built_source_manifest")" == "$receipt_source_sha" ]] ||
        hp0_die "receipt-bound build-source manifest digest differs"
    recomputed_source_manifest="$(mktemp "${TMPDIR:-/tmp}/hp0-build-source-current.XXXXXX")"
    hp0_write_build_source_manifest "$repo_root" "$recomputed_source_manifest"
    [[ "$(hp0_build_source_manifest_sha256 "$recomputed_source_manifest")" == "$receipt_source_sha" ]] || {
        rm -f -- "$recomputed_source_manifest"
        hp0_die "current build-affecting source manifest differs from the build receipt"
    }
    if ! cmp -s "$built_source_manifest" "$recomputed_source_manifest"; then
        rm -f -- "$recomputed_source_manifest"
        hp0_die "current build-affecting source path/mode/blob manifest differs from the build receipt"
    fi
    rm -f -- "$recomputed_source_manifest"

    [[ "$(hp0_read_kv_value "$receipt" sdk_ref)" == "runtime/$HP0_SDK_REF" ]] ||
        hp0_die "build receipt names the wrong Freedesktop SDK ref"
    [[ "$(hp0_read_kv_value "$receipt" sdk_commit)" == "$HP0_SDK_COMMIT" ]] ||
        hp0_die "build receipt names the wrong Freedesktop SDK commit"
    [[ "$(hp0_read_kv_value "$receipt" vst3_sdk_commit)" == "$HP0_VST3_SDK_COMMIT" ]] ||
        hp0_die "build receipt names the wrong VST3 SDK commit"
    [[ "$(hp0_read_kv_value "$receipt" compiler)" == "GCC 15.2.0" ]] || hp0_die "build receipt compiler identity differs"
    [[ "$(hp0_read_kv_value "$receipt" cmake)" == "4.4.2" ]] || hp0_die "build receipt CMake identity differs"
    [[ "$(hp0_read_kv_value "$receipt" ninja)" == "1.13.2" ]] || hp0_die "build receipt Ninja identity differs"
    [[ "$(hp0_read_kv_value "$receipt" pkg_config)" == "2.5.1" ]] || hp0_die "build receipt pkg-config identity differs"
    [[ "$(hp0_read_kv_value "$receipt" processor_class_id)" == "$HP0_PROCESSOR_CLASS_ID" ]] ||
        hp0_die "build receipt processor class ID differs"
    [[ "$(hp0_read_kv_value "$receipt" controller_class_id)" == "$HP0_CONTROLLER_CLASS_ID" ]] ||
        hp0_die "build receipt controller class ID differs"
    [[ "$(hp0_read_kv_value "$receipt" gain_parameter_id)" == "$HP0_GAIN_PARAMETER_ID" ]] ||
        hp0_die "build receipt gain parameter ID differs"
    [[ "$(hp0_read_kv_value "$receipt" bypass_parameter_id)" == "$HP0_BYPASS_PARAMETER_ID" ]] ||
        hp0_die "build receipt bypass parameter ID differs"

    discovered_bundle="$(hp0_find_bundle "$build_dir")"
    discovered_validator="$(hp0_find_validator "$build_dir")"
    receipt_bundle="$(hp0_read_kv_value "$receipt" bundle)"
    receipt_module="$(hp0_read_kv_value "$receipt" module)"
    receipt_validator="$(hp0_read_kv_value "$receipt" validator)"
    [[ "$receipt_bundle" == "$discovered_bundle" ]] || hp0_die "build receipt bundle path differs from the bounded build result"
    [[ "$receipt_module" == "$discovered_bundle/$HP0_MODULE_RELATIVE" ]] || hp0_die "build receipt module path differs"
    [[ "$receipt_validator" == "$discovered_validator" ]] || hp0_die "build receipt validator path differs from the bounded build result"
    hp0_require_bundle_shape "$receipt_bundle"
    [[ -f "$receipt_validator" && ! -L "$receipt_validator" && -x "$receipt_validator" ]] ||
        hp0_die "receipt-bound validator is not a regular executable"

    receipt_module_sha="$(hp0_read_kv_value "$receipt" module_sha256)"
    receipt_manifest_sha="$(hp0_read_kv_value "$receipt" bundle_manifest_sha256)"
    receipt_validator_sha="$(hp0_read_kv_value "$receipt" validator_sha256)"
    [[ "$receipt_module_sha" =~ ^[0-9a-f]{64}$ && "$receipt_manifest_sha" =~ ^[0-9a-f]{64}$ &&
       "$receipt_validator_sha" =~ ^[0-9a-f]{64}$ ]] || hp0_die "build receipt contains a malformed SHA-256"
    [[ "$(sha256sum "$receipt_module" | awk '{print $1}')" == "$receipt_module_sha" ]] ||
        hp0_die "build receipt module SHA-256 does not match the build product"
    [[ "$(sha256sum "$receipt_validator" | awk '{print $1}')" == "$receipt_validator_sha" ]] ||
        hp0_die "build receipt validator SHA-256 does not match the executable"

    built_manifest="$build_dir/hp0-built-bundle.sha256"
    [[ -f "$built_manifest" && ! -L "$built_manifest" ]] || hp0_die "build bundle manifest is missing or is a symlink"
    [[ "$(sha256sum "$built_manifest" | awk '{print $1}')" == "$receipt_manifest_sha" ]] ||
        hp0_die "build receipt bundle-manifest SHA-256 differs"
    recomputed_manifest="$(mktemp "${TMPDIR:-/tmp}/hp0-build-manifest.XXXXXX")"
    hp0_tree_manifest "$receipt_bundle" "$recomputed_manifest"
    if ! cmp -s "$built_manifest" "$recomputed_manifest"; then
        rm -f -- "$recomputed_manifest"
        hp0_die "build bundle roster or file hashes differ from the receipt-bound manifest"
    fi
    rm -f -- "$recomputed_manifest"

    HP0_VERIFIED_BUILD_RECEIPT="$receipt"
    HP0_VERIFIED_HISTORICAL_REPOSITORY_COMMIT="$historical_commit"
    HP0_VERIFIED_HISTORICAL_REPOSITORY_TREE="$historical_tree"
    HP0_VERIFIED_REPOSITORY_COMMIT="$historical_commit"
    HP0_VERIFIED_REPOSITORY_TREE="$historical_tree"
    HP0_VERIFIED_BUILD_SOURCE_MANIFEST_SCHEMA="$receipt_source_schema"
    HP0_VERIFIED_BUILD_SOURCE_MANIFEST_SHA256="$receipt_source_sha"
    HP0_VERIFIED_BUILD_BUNDLE="$receipt_bundle"
    HP0_VERIFIED_BUILD_MODULE="$receipt_module"
    HP0_VERIFIED_BUILD_MODULE_SHA256="$receipt_module_sha"
    HP0_VERIFIED_BUILD_MANIFEST_SHA256="$receipt_manifest_sha"
    HP0_VERIFIED_VALIDATOR="$receipt_validator"
    HP0_VERIFIED_VALIDATOR_SHA256="$receipt_validator_sha"
}

hp0_capture_bitwig_fixture() {
    local output="$1"
    local test_app_commit="${2:-}"
    local test_runtime_commit="${3:-}"
    local app_ref app_commit app_runtime app_version runtime_commit app_info

    command -v flatpak >/dev/null 2>&1 || hp0_die "flatpak command is not installed"
    if flatpak info --user --show-ref "$HP0_BITWIG_APP_ID" >/dev/null 2>&1; then
        hp0_die "user-scope Bitwig installation would shadow the required system fixture"
    fi
    app_ref="$(flatpak info --system --show-ref "$HP0_BITWIG_APP_ID" 2>/dev/null)" ||
        hp0_die "required system-scope Bitwig fixture is not installed"
    app_commit="$(flatpak info --system --show-commit "$HP0_BITWIG_APP_ID")"
    app_runtime="$(flatpak info --system --show-runtime "$HP0_BITWIG_APP_ID")"
    app_info="$(LC_ALL=C flatpak info --system "$HP0_BITWIG_APP_ID")"
    app_version="$(printf '%s\n' "$app_info" | sed -n 's/^[[:space:]]*Version:[[:space:]]*//p')"
    runtime_commit="$(flatpak info --system --show-commit "$HP0_BITWIG_RUNTIME_REF" 2>/dev/null)" ||
        hp0_die "required system-scope Bitwig runtime fixture is not installed"
    [[ -z "$test_app_commit" ]] || app_commit="$test_app_commit"
    [[ -z "$test_runtime_commit" ]] || runtime_commit="$test_runtime_commit"

    cat >"$output" <<EOF
schema=linux-vst-bridge-hp0-bitwig-fixture/v1
app_id=$HP0_BITWIG_APP_ID
app_ref=$app_ref
app_version=$app_version
app_commit=$app_commit
app_scope=system
user_app_installation=absent
runtime_ref=$app_runtime
runtime_commit=$runtime_commit
runtime_scope=system
EOF
}

hp0_require_bitwig_fixture_snapshot() {
    local snapshot="$1"
    [[ "$(hp0_read_kv_value "$snapshot" schema)" == "linux-vst-bridge-hp0-bitwig-fixture/v1" ]] ||
        hp0_die "unknown Bitwig fixture snapshot schema"
    [[ "$(hp0_read_kv_value "$snapshot" app_id)" == "$HP0_BITWIG_APP_ID" ]] || hp0_die "wrong Bitwig app ID"
    [[ "$(hp0_read_kv_value "$snapshot" app_ref)" == "$HP0_BITWIG_APP_REF" ]] || hp0_die "wrong Bitwig branch or architecture"
    [[ "$(hp0_read_kv_value "$snapshot" app_version)" == "$HP0_BITWIG_APP_VERSION" ]] || hp0_die "wrong Bitwig version"
    [[ "$(hp0_read_kv_value "$snapshot" app_commit)" == "$HP0_BITWIG_APP_COMMIT" ]] || hp0_die "wrong Bitwig app commit"
    [[ "$(hp0_read_kv_value "$snapshot" app_scope)" == system ]] || hp0_die "Bitwig is not system-scope"
    [[ "$(hp0_read_kv_value "$snapshot" user_app_installation)" == absent ]] || hp0_die "user Bitwig shadowing is present"
    [[ "$(hp0_read_kv_value "$snapshot" runtime_ref)" == "$HP0_BITWIG_RUNTIME_REF" ]] || hp0_die "wrong Bitwig runtime ref"
    [[ "$(hp0_read_kv_value "$snapshot" runtime_commit)" == "$HP0_BITWIG_RUNTIME_COMMIT" ]] || hp0_die "wrong Bitwig runtime commit"
    [[ "$(hp0_read_kv_value "$snapshot" runtime_scope)" == system ]] || hp0_die "Bitwig runtime is not system-scope"
}

hp0_require_bounded_log() {
    local log="$1"
    local max_bytes="${2:-4194304}"
    [[ -f "$log" ]] || hp0_die "expected log was not created: $log"
    local bytes
    bytes="$(stat -c '%s' "$log")"
    ((bytes <= max_bytes)) || hp0_die "bounded log exceeded $max_bytes bytes: $log"
}
