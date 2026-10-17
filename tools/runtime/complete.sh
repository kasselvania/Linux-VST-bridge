#!/usr/bin/env bash
# Complete the same private source build; no acquisition/publication selection.
set -euo pipefail
if [[ $# != 1 ]]; then
    echo 'Usage: tools/runtime/complete.sh EXISTING_STORAGE2_BUILD_DIRECTORY' >&2
    exit 2
fi
recipe_dir=$(cd -- "$(dirname -- "$0")" && pwd)
output=$(realpath -e -- "$1")
[[ "$output" == /mnt/Storage2/LinuxPluginBridge/?* ]] || {
    echo 'This homelab recipe requires an output under /mnt/Storage2/LinuxPluginBridge' >&2
    exit 1
}
[[ $(findmnt -n -o TARGET -T /mnt/Storage2) == /mnt/Storage2 ]] || {
    echo 'Storage2 is not mounted' >&2; exit 1;
}
exec 9>/mnt/Storage2/LinuxPluginBridge/heavy-build.lock
flock -n 9 || { echo 'The shared builder is busy' >&2; exit 1; }
for arch in i386 x86_64; do
    [[ -f "$output/build/.wine-$arch-build" ]] || {
        echo "The Wine $arch build must complete first" >&2; exit 1;
    }
done
systemd-run --user --scope --unit="lvb-runtime-complete-$$" \
    -p CPUQuota=300% -p MemoryMax=6G -p MemorySwapMax=0 -p TasksMax=512 \
    /bin/bash "$recipe_dir/complete-work.sh" "$output" > "$output/complete.log" 2>&1
echo "Private Proton composition complete: $output/build/redist"
