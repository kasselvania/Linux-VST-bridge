#!/usr/bin/env bash
# Make sure Ubuntu packages are on a GitHub runner without waiting on a
# stalled mirror: apt-install.sh [apt-get install option...] PACKAGE...
#
# Packages already on the runner image are left as they are; only missing
# ones are downloaded. The runner's mirror list (/etc/apt/apt-mirrors.txt)
# moves to the next mirror only when a request fails, and a download that
# trickles never fails, so each download attempt has a time limit and the
# attempts after the first skip the Azure mirror. Only downloads are retried:
# dpkg runs once, from the local cache, so it is never cut off part-way.
set -euo pipefail
[ "$#" -gt 0 ] || { echo "usage: $0 [apt-get install option...] PACKAGE..." >&2; exit 2; }
export DEBIAN_FRONTEND=noninteractive
apt=(apt-get -q -o Acquire::Retries=2 -o Acquire::http::Timeout=30 -o Acquire::https::Timeout=30
    -o DPkg::Lock::Timeout=60)
mirrors=/etc/apt/apt-mirrors.txt
# The man-db trigger is slow on runners, and nothing here reads manual pages.
sudo rm -f /var/lib/man-db/auto-update
for attempt in 1 2 3; do
    if timeout 60 sudo -E "${apt[@]}" update &&
        timeout 90 sudo -E "${apt[@]}" install -y --no-upgrade --download-only "$@"; then
        exec sudo -E "${apt[@]}" install -y --no-upgrade --no-download "$@"
    fi
    if [ "$attempt" -eq 3 ]; then
        echo "::error title=Ubuntu packages::Three downloads failed or timed out: $*"
        exit 1
    fi
    if [ -f "$mirrors" ] && grep -q 'azure\.archive\.ubuntu\.com' "$mirrors"; then
        sudo sed -i '/azure\.archive\.ubuntu\.com/d' "$mirrors"
    fi
    echo "Package download attempt $attempt failed or timed out; next from $(head -n 1 "$mirrors" 2>/dev/null || echo 'the same sources')."
    sleep 5
done
