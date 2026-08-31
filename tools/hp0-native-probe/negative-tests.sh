#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
# shellcheck source=common.sh
source "$script_dir/common.sh"

usage() { printf 'usage: %s --build-dir PATH\n' "${0##*/}" >&2; }
[[ "${1:-}" == --build-dir && -n "${2:-}" && $# -eq 2 ]] || { usage; exit 2; }

repo_root="$(hp0_repo_root)"
build_dir="$2"
[[ "$build_dir" == /* ]] || build_dir="$repo_root/$build_dir"
hp0_require_build_receipt "$build_dir"
built_bundle="$HP0_VERIFIED_BUILD_BUNDLE"
built_validator="$HP0_VERIFIED_VALIDATOR"
expected_module_sha256="$HP0_VERIFIED_BUILD_MODULE_SHA256"
sdk_root="$(hp0_default_sdk_root)"
cache_root="$(hp0_user_cache_root)"
test_root="$(mktemp -d "$cache_root/hp0-negative-tests.XXXXXX")"
dirty_marker="$sdk_root/.hp0-dirty-negative-test.$$"
bitwig_pid=""
wrong_sdk=""
escape_root=""
outside_test_root=""
source_worktree_result=""
declare -a source_worktrees=()
zeros_sha256="0000000000000000000000000000000000000000000000000000000000000000"

cleanup() {
    rm -f -- "$dirty_marker"
    if [[ -n "$bitwig_pid" ]]; then
        kill "$bitwig_pid" >/dev/null 2>&1 || true
        wait "$bitwig_pid" 2>/dev/null || true
    fi
    if [[ -n "$wrong_sdk" && -e "$wrong_sdk" ]]; then
        git -C "$sdk_root" worktree remove --force "$wrong_sdk" >/dev/null 2>&1 || true
        git -C "$sdk_root" worktree prune >/dev/null 2>&1 || true
    fi
    for source_worktree in "${source_worktrees[@]}"; do
        git -C "$repo_root" worktree remove --force "$source_worktree" >/dev/null 2>&1 || true
    done
    git -C "$repo_root" worktree prune >/dev/null 2>&1 || true
    [[ -z "$escape_root" || ! -e "$escape_root" ]] || rm -rf -- "$escape_root"
    [[ -z "$outside_test_root" || ! -e "$outside_test_root" ]] || rm -rf -- "$outside_test_root"
    rm -rf -- "$test_root"
}
trap cleanup EXIT HUP INT TERM

expect_failure() {
    local label="$1"
    local expected_text="$2"
    shift 2
    local log="$test_root/$label.log"
    if "$@" >"$log" 2>&1; then
        hp0_die "negative case unexpectedly succeeded: $label"
    fi
    grep -Fq "$expected_text" "$log" || hp0_die "negative case failed for the wrong reason: $label"
    printf '%s=passed\n' "$label"
}

snapshot_publication() {
    local publication_root="$1"
    local output="$2"
    hp0_tree_manifest "$publication_root/$HP0_BUNDLE_NAME" "$output"
}

assert_publication_unchanged() {
    local before="$1"
    local publication_root="$2"
    local receipt_path="${3:-}"
    local after="$test_root/after.$$.sha256"
    local -a inspect_args=(inspect --publication-root "$publication_root" --test-mode --test-root "$test_root")
    [[ -z "$receipt_path" ]] || inspect_args+=(--receipt "$receipt_path")
    "$script_dir/publish.sh" "${inspect_args[@]}" >/dev/null
    snapshot_publication "$publication_root" "$after"
    cmp -s "$before" "$after" || hp0_die "complete prior publication roster or hashes changed"
    rm -f -- "$after"
}

make_build_fixture() {
    local source_build="$1"
    local target_build="$2"
    local source_bundle source_validator
    source_bundle="$(hp0_find_bundle "$source_build")"
    source_validator="$(hp0_find_validator "$source_build")"
    mkdir -p "$target_build/VST3/Release" "$target_build/bin/Release"
    cp -a "$source_bundle" "$target_build/VST3/Release/"
    cp -p "$source_validator" "$target_build/bin/Release/validator"
    cp -p "$source_build/hp0-built-bundle.sha256" "$target_build/hp0-built-bundle.sha256"
    cp -p "$source_build/hp0-build-source.manifest" "$target_build/hp0-build-source.manifest"
    sed "s|$source_build|$target_build|g" "$source_build/hp0-build.receipt" >"$target_build/hp0-build.receipt"
}

new_source_worktree() {
    local label="$1"
    source_worktree_result="$test_root/source-repository-$label"
    git -C "$repo_root" worktree add --detach "$source_worktree_result" HEAD >/dev/null
    source_worktrees+=("$source_worktree_result")
}

commit_source_test_change() {
    local worktree="$1"
    local path="$2"
    git -C "$worktree" add -- "$path"
    git -C "$worktree" -c user.name='HP0 deterministic test' \
        -c user.email='hp0@invalid.example' commit -m 'HP0 deterministic source identity test' >/dev/null
}

generate_source_manifest_in_worktree() {
    local worktree="$1"
    local output="$2"
    (cd "$worktree" && tools/hp0-native-probe/build.sh --print-source-manifest) >"$output"
}

verify_receipt_in_worktree() {
    local worktree="$1"
    local fixture="$2"
    (cd "$worktree" && tools/hp0-native-probe/build.sh --verify-receipt --build-dir "$fixture")
}

assert_manifest_changed() {
    local label="$1"
    local worktree="$2"
    local output="$test_root/$label.manifest"
    generate_source_manifest_in_worktree "$worktree" "$output"
    ! cmp -s "$test_root/build-source.baseline" "$output" ||
        hp0_die "build-source manifest did not change: $label"
    printf '%s=passed\n' "$label"
}

assert_no_transaction_debris() {
    local root="$1"
    if find "$root" -mindepth 1 -maxdepth 5 \
        \( -name '.hp0-stage.*' -o -name '.LabHostProbe.vst3.previous.*' \
           -o -name '.hp0-receipt-stage.*' -o -name '.hp0-receipt-previous.*' \) \
        -print -quit | grep -q .; then
        hp0_die "publication transaction left a stage or backup sibling"
    fi
}

flatpak override --user --show "$HP0_BITWIG_APP_ID" >"$test_root/user-override.before"
flatpak override --system --show "$HP0_BITWIG_APP_ID" >"$test_root/system-override.before"
[[ "$(sha256sum "$test_root/user-override.before" | awk '{print $1}')" == "$HP0_BITWIG_USER_OVERRIDE_SHA256" ]] ||
    hp0_die "negative-test precondition has the wrong user override identity"
[[ "$(sha256sum "$test_root/system-override.before" | awk '{print $1}')" == "$HP0_BITWIG_SYSTEM_OVERRIDE_SHA256" ]] ||
    hp0_die "negative-test precondition has the wrong system override identity"

"$script_dir/build.sh" --print-source-manifest >"$test_root/build-source.baseline"
baseline_source_sha="$(hp0_build_source_manifest_sha256 "$test_root/build-source.baseline")"

new_source_worktree evidence-only
evidence_worktree="$source_worktree_result"
printf '\nHP0 deterministic evidence-only identity fixture.\n' >> \
    "$evidence_worktree/evidence/hp0-native-vst3-bitwig-sandbox/BASIS.md"
commit_source_test_change "$evidence_worktree" evidence/hp0-native-vst3-bitwig-sandbox/BASIS.md
generate_source_manifest_in_worktree "$evidence_worktree" "$test_root/build-source.evidence-only"
cmp -s "$test_root/build-source.baseline" "$test_root/build-source.evidence-only" ||
    hp0_die "evidence-only commit changed the build-source manifest"
evidence_fixture="$evidence_worktree/build/receipt-fixture"
make_build_fixture "$build_dir" "$evidence_fixture"
verify_receipt_in_worktree "$evidence_worktree" "$evidence_fixture" >/dev/null
printf 'evidence_only_manifest_stable=passed\n'
printf 'evidence_only_receipt_verification=passed\n'

new_source_worktree native-source
native_worktree="$source_worktree_result"
printf '\n// HP0 deterministic source-manifest negative fixture.\n' >> \
    "$native_worktree/native-probe/source/processor.cpp"
commit_source_test_change "$native_worktree" native-probe/source/processor.cpp
assert_manifest_changed native_source_manifest_changed "$native_worktree"
native_fixture="$native_worktree/build/receipt-fixture"
make_build_fixture "$build_dir" "$native_fixture"
expect_failure native_source_receipt_rejected "current build-affecting source manifest differs" \
    verify_receipt_in_worktree "$native_worktree" "$native_fixture"
native_head="$(git -C "$native_worktree" rev-parse HEAD)"
native_tree="$(git -C "$native_worktree" rev-parse 'HEAD^{tree}')"
sed -i \
    -e "s/^historical_repository_commit=.*/historical_repository_commit=$native_head/" \
    -e "s/^historical_repository_tree=.*/historical_repository_tree=$native_tree/" \
    "$native_fixture/hp0-build.receipt"
expect_failure stale_receipt_provenance_edit_refused "current build-affecting source manifest differs" \
    verify_receipt_in_worktree "$native_worktree" "$native_fixture"

new_source_worktree top-level-cmake
top_cmake_worktree="$source_worktree_result"
printf '\n# HP0 deterministic top-level CMake manifest fixture.\n' >>"$top_cmake_worktree/CMakeLists.txt"
commit_source_test_change "$top_cmake_worktree" CMakeLists.txt
assert_manifest_changed top_level_cmake_manifest_changed "$top_cmake_worktree"

new_source_worktree cmake-directory
cmake_worktree="$source_worktree_result"
printf '\n# HP0 deterministic CMake-directory manifest fixture.\n' >> \
    "$cmake_worktree/cmake/HP0ToolchainGuard.cmake"
commit_source_test_change "$cmake_worktree" cmake/HP0ToolchainGuard.cmake
assert_manifest_changed cmake_directory_manifest_changed "$cmake_worktree"

new_source_worktree dependency-lock
dependency_worktree="$source_worktree_result"
printf '\nHP0 deterministic dependency-lock manifest fixture.\n' >> \
    "$dependency_worktree/docs/HP0_DEPENDENCY_LOCK.md"
commit_source_test_change "$dependency_worktree" docs/HP0_DEPENDENCY_LOCK.md
assert_manifest_changed dependency_lock_manifest_changed "$dependency_worktree"

new_source_worktree unexpected-source
unexpected_worktree="$source_worktree_result"
printf 'unexpected tracked source fixture\n' >"$unexpected_worktree/native-probe/unexpected-source.txt"
commit_source_test_change "$unexpected_worktree" native-probe/unexpected-source.txt
expect_failure unexpected_tracked_build_source "unexpected or missing tracked path beneath a declared build-source directory" \
    generate_source_manifest_in_worktree "$unexpected_worktree" "$test_root/build-source.unexpected"

test_build_dir="$test_root/build-fixture"
make_build_fixture "$build_dir" "$test_build_dir"
hp0_require_build_receipt "$test_build_dir"

wrong_sdk="$test_root/wrong-sdk"
git -C "$sdk_root" worktree add --detach "$wrong_sdk" HEAD^ >/dev/null
expect_failure wrong_sdk_commit "wrong VST3 SDK commit" "$script_dir/verify-dependency.sh" "$wrong_sdk"
git -C "$sdk_root" worktree remove --force "$wrong_sdk"
git -C "$sdk_root" worktree prune
wrong_sdk=""

touch "$dirty_marker"
expect_failure dirty_sdk_checkout "dirty or incomplete" "$script_dir/verify-dependency.sh" "$sdk_root"
rm -f -- "$dirty_marker"
"$script_dir/verify-dependency.sh" "$sdk_root" >/dev/null

mkdir -p "$test_root/empty-flatpak-data" "$test_root/empty-flatpak-config"
expect_failure missing_freedesktop_sdk "required user-scope Flatpak SDK is missing" env \
    XDG_DATA_HOME="$test_root/empty-flatpak-data" XDG_CONFIG_HOME="$test_root/empty-flatpak-config" \
    "$script_dir/build.sh" --build-dir "$repo_root/build/hp0-missing-sdk" --check-only

real_publication_root="$(hp0_default_publication_root)"
real_receipt="$(hp0_default_receipt_path)"
"$script_dir/publish.sh" inspect >/dev/null
snapshot_publication "$real_publication_root" "$test_root/real-publication.before"
real_receipt_sha="$(sha256sum "$real_receipt" | awk '{print $1}')"

ordinary_cache="$test_root/ordinary-cache"
mkdir "$ordinary_cache"
mkdir "$ordinary_cache/linux-vst-bridge"
unrelated_receipt="$ordinary_cache/linux-vst-bridge/hp0-publication.receipt"
printf 'unrelated ordinary cache file\n' >"$unrelated_receipt"
unrelated_receipt_sha="$(sha256sum "$unrelated_receipt" | awk '{print $1}')"
unrelated_receipt_stat="$(stat -c '%a:%s:%Y:%Z' "$unrelated_receipt")"
expect_failure unrelated_ordinary_receipt "refusing unknown receipt destination" env \
    XDG_CACHE_HOME="$ordinary_cache" "$script_dir/publish.sh" publish --bundle "$built_bundle"
[[ "$(sha256sum "$unrelated_receipt" | awk '{print $1}')" == "$unrelated_receipt_sha" ]] ||
    hp0_die "unrelated ordinary receipt bytes changed"
[[ "$(stat -c '%a:%s:%Y:%Z' "$unrelated_receipt")" == "$unrelated_receipt_stat" ]] ||
    hp0_die "unrelated ordinary receipt metadata changed"
snapshot_publication "$real_publication_root" "$test_root/real-publication.after-unrelated"
cmp -s "$test_root/real-publication.before" "$test_root/real-publication.after-unrelated" ||
    hp0_die "unrelated ordinary receipt refusal changed the real publication"
[[ "$(sha256sum "$real_receipt" | awk '{print $1}')" == "$real_receipt_sha" ]] ||
    hp0_die "unrelated ordinary receipt refusal changed the real receipt"

alternate_ordinary_receipt="$ordinary_cache/alternate.receipt"
expect_failure alternate_ordinary_receipt "ordinary receipt path must equal the declared HP0 project receipt path" env \
    XDG_CACHE_HOME="$ordinary_cache" "$script_dir/publish.sh" publish --bundle "$built_bundle" \
    --receipt "$alternate_ordinary_receipt"
[[ ! -e "$alternate_ordinary_receipt" ]] || hp0_die "alternate ordinary receipt path was created"
snapshot_publication "$real_publication_root" "$test_root/real-publication.after-alternate"
cmp -s "$test_root/real-publication.before" "$test_root/real-publication.after-alternate" ||
    hp0_die "alternate ordinary receipt refusal changed the real publication"

path_safety_root="$test_root/path-safety-publication"
path_safety_receipt="$test_root/path-safety.receipt"
"$script_dir/publish.sh" publish --bundle "$built_bundle" --publication-root "$path_safety_root" \
    --receipt "$path_safety_receipt" --test-mode --test-root "$test_root" >/dev/null
"$script_dir/publish.sh" publish --bundle "$built_bundle" --publication-root "$path_safety_root" \
    --receipt "$path_safety_receipt" --test-mode --test-root "$test_root" >/dev/null
"$script_dir/publish.sh" inspect --publication-root "$path_safety_root" --receipt "$path_safety_receipt" \
    --test-mode --test-root "$test_root" >/dev/null
printf 'accepted_prior_hp0_receipt_replacement=passed\n'
snapshot_publication "$path_safety_root" "$test_root/path-safety.before"

outside_test_root="$(mktemp -d "$cache_root/hp0-negative-outside.XXXXXX")"
outside_receipt="$outside_test_root/outside.receipt"
expect_failure test_receipt_outside_declared_root "test receipt must be a canonical descendant of the exact test root" \
    "$script_dir/publish.sh" publish --bundle "$built_bundle" --publication-root "$path_safety_root" \
    --receipt "$outside_receipt" --test-mode --test-root "$test_root"
[[ ! -e "$outside_receipt" ]] || hp0_die "out-of-root test receipt was created"
assert_publication_unchanged "$test_root/path-safety.before" "$path_safety_root" "$path_safety_receipt"

outside_build="$outside_test_root/build-fixture"
mkdir "$outside_build"
expect_failure test_build_outside_declared_root "test build root must be a canonical descendant of the exact test root" \
    "$script_dir/sandbox-probe.sh" --build-dir "$outside_build" --publication-root "$path_safety_root" \
    --test-mode --test-root "$test_root"

escape_root="$(mktemp -d "${TMPDIR:-/tmp}/hp0-negative-escape.XXXXXX")"
printf 'escaped target sentinel\n' >"$escape_root/sentinel"
mkdir "$escape_root/escaped-test-root" "$escape_root/escaped-build"
ln -s "$escape_root" "$test_root/symlink-cache-component"
symlinked_test_root="$test_root/symlink-cache-component/escaped-test-root"
expect_failure symlinked_test_root_refused "test root has a symlinked path component" \
    "$script_dir/publish.sh" publish --bundle "$built_bundle" \
    --publication-root "$symlinked_test_root/publication" --receipt "$symlinked_test_root/receipt" \
    --test-mode --test-root "$symlinked_test_root"
grep -Fxq 'escaped target sentinel' "$escape_root/sentinel" || hp0_die "symlinked test root changed escaped storage"

ln -s "$escape_root" "$test_root/symlink-publication-component"
expect_failure symlinked_publication_ancestor_refused "test publication root has a symlinked path component" \
    "$script_dir/publish.sh" publish --bundle "$built_bundle" \
    --publication-root "$test_root/symlink-publication-component/escaped-publication" \
    --receipt "$test_root/symlink-publication.receipt" --test-mode --test-root "$test_root"
grep -Fxq 'escaped target sentinel' "$escape_root/sentinel" || hp0_die "symlinked publication root changed escaped storage"

ln -s "$escape_root" "$test_root/symlink-build-component"
expect_failure symlinked_test_build_ancestor_refused "test build root has a symlinked path component" \
    "$script_dir/sandbox-probe.sh" --build-dir "$test_root/symlink-build-component/escaped-build" \
    --publication-root "$path_safety_root" --test-mode --test-root "$test_root"
grep -Fxq 'escaped target sentinel' "$escape_root/sentinel" || hp0_die "symlinked test build root changed escaped storage"

unknown_root="$test_root/unknown-publication"
mkdir -p "$unknown_root/$HP0_BUNDLE_NAME"
printf 'not owned\n' >"$unknown_root/$HP0_BUNDLE_NAME/unrelated"
expect_failure unknown_publication_destination "refusing unknown publication destination" \
    "$script_dir/publish.sh" publish --bundle "$built_bundle" \
    --publication-root "$unknown_root" --receipt "$test_root/unknown.receipt" --test-mode --test-root "$test_root"
grep -Fxq 'not owned' "$unknown_root/$HP0_BUNDLE_NAME/unrelated" || hp0_die "unknown destination was changed"

interrupted_root="$test_root/interrupted-publication"
interrupted_receipt="$test_root/interrupted.receipt"
"$script_dir/publish.sh" publish --bundle "$built_bundle" --publication-root "$interrupted_root" \
    --receipt "$interrupted_receipt" --test-mode --test-root "$test_root" >/dev/null
snapshot_publication "$interrupted_root" "$test_root/interrupted.before"
interrupted_receipt_sha="$(sha256sum "$interrupted_receipt" | awk '{print $1}')"
expect_failure interrupted_staged_publication "simulated interruption after verified staging" \
    "$script_dir/publish.sh" publish --bundle "$built_bundle" --publication-root "$interrupted_root" \
    --receipt "$interrupted_receipt" --test-mode --test-root "$test_root" --simulate-after-stage
assert_publication_unchanged "$test_root/interrupted.before" "$interrupted_root" "$interrupted_receipt"
[[ "$(sha256sum "$interrupted_receipt" | awk '{print $1}')" == "$interrupted_receipt_sha" ]] ||
    hp0_die "staged interruption changed the prior receipt"
assert_no_transaction_debris "$interrupted_root"

post_swap_root="$test_root/post-swap-publication"
post_swap_receipt="$test_root/post-swap.receipt"
"$script_dir/publish.sh" publish --bundle "$built_bundle" --publication-root "$post_swap_root" \
    --receipt "$post_swap_receipt" --test-mode --test-root "$test_root" >/dev/null
snapshot_publication "$post_swap_root" "$test_root/post-swap.before"
post_swap_receipt_sha="$(sha256sum "$post_swap_receipt" | awk '{print $1}')"
expect_failure post_swap_prior_rollback "simulated failure after destination swap" \
    "$script_dir/publish.sh" publish --bundle "$built_bundle" --publication-root "$post_swap_root" \
    --receipt "$post_swap_receipt" --test-mode --test-root "$test_root" --simulate-after-swap
assert_publication_unchanged "$test_root/post-swap.before" "$post_swap_root" "$post_swap_receipt"
[[ "$(sha256sum "$post_swap_receipt" | awk '{print $1}')" == "$post_swap_receipt_sha" ]] ||
    hp0_die "post-swap rollback changed the prior receipt"
assert_no_transaction_debris "$post_swap_root"

receipt_prior_root="$test_root/receipt-prior-publication"
receipt_prior_receipt="$test_root/receipt-prior.receipt"
"$script_dir/publish.sh" publish --bundle "$built_bundle" --publication-root "$receipt_prior_root" \
    --receipt "$receipt_prior_receipt" --test-mode --test-root "$test_root" >/dev/null
snapshot_publication "$receipt_prior_root" "$test_root/receipt-prior.before"
receipt_prior_sha="$(sha256sum "$receipt_prior_receipt" | awk '{print $1}')"
expect_failure receipt_failure_with_prior "simulated receipt staging failure" \
    "$script_dir/publish.sh" publish --bundle "$built_bundle" --publication-root "$receipt_prior_root" \
    --receipt "$receipt_prior_receipt" --test-mode --test-root "$test_root" --simulate-receipt-failure
assert_publication_unchanged "$test_root/receipt-prior.before" "$receipt_prior_root" "$receipt_prior_receipt"
[[ "$(sha256sum "$receipt_prior_receipt" | awk '{print $1}')" == "$receipt_prior_sha" ]] ||
    hp0_die "receipt failure did not preserve the prior receipt"
assert_no_transaction_debris "$receipt_prior_root"

receipt_absent_root="$test_root/receipt-absent-publication"
receipt_absent_receipt="$test_root/receipt-absent.receipt"
expect_failure receipt_failure_without_prior "simulated receipt staging failure" \
    "$script_dir/publish.sh" publish --bundle "$built_bundle" --publication-root "$receipt_absent_root" \
    --receipt "$receipt_absent_receipt" --test-mode --test-root "$test_root" --simulate-receipt-failure
[[ ! -e "$receipt_absent_root/$HP0_BUNDLE_NAME" ]] || hp0_die "failed first publication did not restore absence"
[[ ! -e "$receipt_absent_receipt" ]] || hp0_die "failed first publication left a successful-looking receipt"
assert_no_transaction_debris "$receipt_absent_root"

receipt_commit_root="$test_root/receipt-commit-publication"
receipt_commit_receipt="$test_root/receipt-commit.receipt"
"$script_dir/publish.sh" publish --bundle "$built_bundle" --publication-root "$receipt_commit_root" \
    --receipt "$receipt_commit_receipt" --test-mode --test-root "$test_root" >/dev/null
snapshot_publication "$receipt_commit_root" "$test_root/receipt-commit.before"
receipt_commit_sha="$(sha256sum "$receipt_commit_receipt" | awk '{print $1}')"
expect_failure post_receipt_commit_rollback "simulated failure after atomic receipt commit" \
    "$script_dir/publish.sh" publish --bundle "$built_bundle" --publication-root "$receipt_commit_root" \
    --receipt "$receipt_commit_receipt" --test-mode --test-root "$test_root" --simulate-after-receipt-commit
assert_publication_unchanged "$test_root/receipt-commit.before" "$receipt_commit_root" "$receipt_commit_receipt"
[[ "$(sha256sum "$receipt_commit_receipt" | awk '{print $1}')" == "$receipt_commit_sha" ]] ||
    hp0_die "post-receipt-commit rollback did not restore the prior receipt"
assert_no_transaction_debris "$receipt_commit_root"

receipt_commit_absent_root="$test_root/receipt-commit-absent-publication"
receipt_commit_absent_receipt="$test_root/receipt-commit-absent.receipt"
expect_failure post_receipt_commit_absence "simulated failure after atomic receipt commit" \
    "$script_dir/publish.sh" publish --bundle "$built_bundle" \
    --publication-root "$receipt_commit_absent_root" --receipt "$receipt_commit_absent_receipt" \
    --test-mode --test-root "$test_root" --simulate-after-receipt-commit
[[ ! -e "$receipt_commit_absent_root/$HP0_BUNDLE_NAME" ]] || hp0_die "post-receipt-commit first publication did not restore absence"
[[ ! -e "$receipt_commit_absent_receipt" ]] || hp0_die "post-receipt-commit rollback left a receipt"
assert_no_transaction_debris "$receipt_commit_absent_root"

symlink_receipt_root="$test_root/symlink-receipt-publication"
symlink_receipt_target="$test_root/symlink-receipt-target"
symlink_receipt="$test_root/symlink-publication.receipt"
printf 'receipt target sentinel\n' >"$symlink_receipt_target"
ln -s "$symlink_receipt_target" "$symlink_receipt"
expect_failure symlink_receipt_refused "test receipt has a symlinked path component" \
    "$script_dir/publish.sh" publish --bundle "$built_bundle" --publication-root "$symlink_receipt_root" \
    --receipt "$symlink_receipt" --test-mode --test-root "$test_root"
grep -Fxq 'receipt target sentinel' "$symlink_receipt_target" || hp0_die "receipt symlink target changed"
[[ ! -e "$symlink_receipt_root/$HP0_BUNDLE_NAME" ]] || hp0_die "symlink receipt refusal changed publication state"

expect_failure wrong_bitwig_app_commit "wrong Bitwig app commit" \
    "$script_dir/sandbox-probe.sh" --build-dir "$test_build_dir" --publication-root "$interrupted_root" \
    --test-mode --test-root "$test_root" --test-app-commit "$zeros_sha256"
expect_failure wrong_bitwig_runtime_commit "wrong Bitwig runtime commit" \
    "$script_dir/sandbox-probe.sh" --build-dir "$test_build_dir" --publication-root "$interrupted_root" \
    --test-mode --test-root "$test_root" --test-runtime-commit "$zeros_sha256"

validator_build="$test_root/modified-validator-build"
mkdir -p "$validator_build/VST3/Release" "$validator_build/bin/Release"
cp -a "$built_bundle" "$validator_build/VST3/Release/"
cp -p "$built_validator" "$validator_build/bin/Release/validator"
cp -p "$build_dir/hp0-built-bundle.sha256" "$validator_build/hp0-built-bundle.sha256"
cp -p "$build_dir/hp0-build-source.manifest" "$validator_build/hp0-build-source.manifest"
sed "s|$build_dir|$validator_build|g" "$build_dir/hp0-build.receipt" >"$validator_build/hp0-build.receipt"
printf '\0' >>"$validator_build/bin/Release/validator"
expect_failure modified_validator "validator SHA-256 does not match" \
    "$script_dir/sandbox-probe.sh" --build-dir "$validator_build" --publication-root "$interrupted_root" --test-mode --test-root "$test_root"

expect_failure missing_bundle "owned publication is missing" \
    "$script_dir/sandbox-probe.sh" --build-dir "$test_build_dir" \
    --publication-root "$test_root/missing-publication" --expected-module-sha256 "$expected_module_sha256" --test-mode --test-root "$test_root"

modified_root="$test_root/modified-publication"
"$script_dir/publish.sh" publish --bundle "$built_bundle" --publication-root "$modified_root" \
    --receipt "$test_root/modified-publication.receipt" --test-mode --test-root "$test_root" >/dev/null
printf '\0' >>"$modified_root/$HP0_BUNDLE_NAME/$HP0_MODULE_RELATIVE"
expect_failure modified_published_module "published source manifest verification failed" \
    "$script_dir/sandbox-probe.sh" --build-dir "$test_build_dir" \
    --publication-root "$modified_root" --expected-module-sha256 "$expected_module_sha256" --test-mode --test-root "$test_root"

wrong_arch_root="$test_root/wrong-architecture"
"$script_dir/publish.sh" publish --bundle "$built_bundle" --publication-root "$wrong_arch_root" \
    --receipt "$test_root/wrong-architecture.receipt" --test-mode --test-root "$test_root" >/dev/null
wrong_arch_bundle="$wrong_arch_root/$HP0_BUNDLE_NAME"
wrong_arch_module="$wrong_arch_bundle/$HP0_MODULE_RELATIVE"
printf '\003\000' | dd of="$wrong_arch_module" bs=1 seek=18 count=2 conv=notrunc status=none
hp0_source_tree_manifest "$wrong_arch_bundle" "$test_root/wrong-arch-manifest"
cp "$test_root/wrong-arch-manifest" "$wrong_arch_bundle/$HP0_MANIFEST_RELATIVE"
expect_failure wrong_architecture "published module hash differs from the verified build receipt" \
    "$script_dir/sandbox-probe.sh" --build-dir "$test_build_dir" --publication-root "$wrong_arch_root" --test-mode --test-root "$test_root"

python3 -c 'import ctypes,time; ctypes.CDLL(None).prctl(15,b"BitwigStudio",0,0,0); time.sleep(30)' &
bitwig_pid=$!
for _ in 1 2 3 4 5; do
    pgrep -x BitwigStudio >/dev/null 2>&1 && break
    sleep 0.1
done
pgrep -x BitwigStudio >/dev/null 2>&1 || hp0_die "could not establish synthetic Bitwig-running precondition"
expect_failure bitwig_running_precondition "Bitwig process is running" \
    "$script_dir/publish.sh" inspect --publication-root "$interrupted_root" --test-mode --test-root "$test_root"
kill "$bitwig_pid"
wait "$bitwig_pid" 2>/dev/null || true
bitwig_pid=""
hp0_assert_no_bitwig

flatpak override --user --show "$HP0_BITWIG_APP_ID" >"$test_root/user-override.after"
flatpak override --system --show "$HP0_BITWIG_APP_ID" >"$test_root/system-override.after"
cmp -s "$test_root/user-override.before" "$test_root/user-override.after" || hp0_die "user override changed during negative tests"
cmp -s "$test_root/system-override.before" "$test_root/system-override.after" || hp0_die "system override changed during negative tests"

printf 'unknown_destination_unchanged=true\n'
printf 'build_source_manifest_schema=%s\n' "$HP0_BUILD_SOURCE_MANIFEST_SCHEMA"
printf 'build_source_manifest_sha256=%s\n' "$baseline_source_sha"
printf 'evidence_only_manifest_and_receipt=passed\n'
printf 'covered_source_change_refusal=passed\n'
printf 'unexpected_tracked_build_source_refusal=passed\n'
printf 'stale_receipt_provenance_edit_refusal=passed\n'
printf 'unrelated_ordinary_receipt_refused_unchanged=true\n'
printf 'alternate_ordinary_receipt_refused=true\n'
printf 'accepted_prior_hp0_receipt_replacement=true\n'
printf 'test_receipt_outside_root_refused=true\n'
printf 'test_build_outside_root_refused=true\n'
printf 'symlinked_test_root_and_descendants_refused=true\n'
printf 'escaped_targets_unchanged=true\n'
printf 'complete_prior_publication_roster_restored=true\n'
printf 'prior_receipt_restored=true\n'
printf 'failed_first_publication_restored_absence=true\n'
printf 'receipt_commit_is_transactional=true\n'
printf 'receipt_symlink_refused=true\n'
printf 'wrong_bitwig_app_commit_refused=true\n'
printf 'wrong_bitwig_runtime_commit_refused=true\n'
printf 'modified_validator_failed_before_execution=true\n'
printf 'missing_bundle_failed_before_validator=true\n'
printf 'modified_module_failed_before_validator=true\n'
printf 'wrong_architecture_acceptance_failed=true\n'
printf 'bitwig_running_refused=true\n'
printf 'override_before_after=byte_identical\n'
printf 'negative_test_status=passed\n'
