#!/usr/bin/env bash
# Internal build-only execution in the acquired SDK; caller owns lock/cgroup.
set -euo pipefail
output=$1
network=$2
shift 2
network_args=()
case "$network" in
    online) ;;
    offline) network_args=(--unshare-net) ;;
    *) echo 'Unknown SDK network mode' >&2; exit 2 ;;
esac
exec bwrap --unshare-user --uid 0 --gid 0 --unshare-pid "${network_args[@]}" --die-with-parent \
    --ro-bind "$output/sdk-rootfs" / --ro-bind "$output/proton" /work/proton \
    --bind "$output/build" /work/build --bind "$output/cargo" /work/cargo \
    --bind "$output/cache" /work/cache \
    --ro-bind "$output/build-inner.sh" /work/build-inner.sh \
    --ro-bind /etc/resolv.conf /etc/resolv.conf --proc /proc --dev /dev --tmpfs /tmp \
    --setenv PATH /usr/lib/ccache:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
    --setenv CARGO_HOME /work/cargo --setenv XDG_CACHE_HOME /work/cache \
    --setenv CCACHE_DISABLE 1 --chdir /work/build "$@"
