#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
# shellcheck source=common.sh
source "$script_dir/common.sh"

sdk_root="${1:-$(hp0_default_sdk_root)}"
[[ "$sdk_root" == /* ]] || hp0_die "SDK root must be absolute"
[[ -e "$sdk_root/.git" && -f "$sdk_root/CMakeLists.txt" && -f "$sdk_root/LICENSE.txt" ]] ||
    hp0_die "SDK root is not a complete Git checkout"

remote_url="$(git -C "$sdk_root" remote get-url origin 2>/dev/null || true)"
[[ "$remote_url" == "https://github.com/steinbergmedia/vst3sdk.git" ]] ||
    hp0_die "unexpected VST3 SDK origin: ${remote_url:-missing}"

root_commit="$(git -C "$sdk_root" rev-parse HEAD)"
[[ "$root_commit" == "$HP0_VST3_SDK_COMMIT" ]] ||
    hp0_die "wrong VST3 SDK commit: expected $HP0_VST3_SDK_COMMIT observed $root_commit"

root_status="$(git -C "$sdk_root" status --porcelain=v1 --untracked-files=all --ignore-submodules=none)"
[[ -z "$root_status" ]] || hp0_die "VST3 SDK checkout is dirty or incomplete"

readonly expected_submodules=$'base|fcf9da0bd27a16f7f03773a3a39822f28f5c8477\ncmake|054c9143cbb8d47fc4694e473f2ee3b4d951a8f5\ndoc|8bfca19d3b76a61d093951ba9297047f544caea1\npluginterfaces|4f547e8e102b47de4a8b8aaf343c73b700786372\npublic.sdk|586dc5e6c8012c3e4b01c79389375cbe96bdb1da\ntutorials|33b73dfbb87f3fde3bce8c0a10cae934dc66ad34\nvstgui4|5db272256172557818b6158cf0bb2c4410bddb25'

mapfile -t declared_paths < <(git -C "$sdk_root" config --file .gitmodules --get-regexp '^submodule\..*\.path$' | awk '{print $2}' | LC_ALL=C sort)
mapfile -t expected_paths < <(printf '%s\n' "$expected_submodules" | cut -d'|' -f1 | LC_ALL=C sort)
[[ "${#declared_paths[@]}" -eq "${#expected_paths[@]}" ]] || hp0_die "unexpected submodule count"
[[ "$(printf '%s\n' "${declared_paths[@]}")" == "$(printf '%s\n' "${expected_paths[@]}")" ]] ||
    hp0_die "VST3 SDK submodule path set differs from the lock"

while IFS='|' read -r path expected_commit; do
    [[ -e "$sdk_root/$path/.git" ]] || hp0_die "required submodule is not initialized: $path"
    observed_commit="$(git -C "$sdk_root/$path" rev-parse HEAD)"
    [[ "$observed_commit" == "$expected_commit" ]] ||
        hp0_die "wrong submodule commit for $path: expected $expected_commit observed $observed_commit"
    submodule_status="$(git -C "$sdk_root/$path" status --porcelain=v1 --untracked-files=all --ignore-submodules=none)"
    [[ -z "$submodule_status" ]] || hp0_die "dirty VST3 SDK submodule: $path"
done <<<"$expected_submodules"

for required in \
    cmake/modules/SMTG_AddVST3Library.cmake \
    public.sdk/samples/vst-hosting/validator/CMakeLists.txt \
    public.sdk/source/vst/vstaudioeffect.h \
    public.sdk/source/vst/vsteditcontroller.h; do
    [[ -f "$sdk_root/$required" ]] || hp0_die "required pinned SDK interface is missing: $required"
done

printf 'dependency_status=verified\n'
printf 'root=%s\n' "$HP0_VST3_SDK_COMMIT"
printf '%s\n' "$expected_submodules"
