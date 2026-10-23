#!/usr/bin/env bash
# Get Ubuntu packages on a GitHub runner without waiting on a stalled mirror:
#   apt-install.sh [apt-get install option...] PACKAGE...
#   apt-install.sh --download PACKAGE...   (their .deb files into the current
#                                          directory; nothing is installed)
#
# Packages already on the runner image are left as they are; only missing
# ones are downloaded. Only Ubuntu's own index is refreshed: these are Ubuntu
# packages, and a third-party repository on the image once served
# inconsistent index hashes. The runner's mirror list (/etc/apt/apt-mirrors.txt)
# moves to the next mirror only when a request fails, and a download that
# trickles never fails, so each download attempt has a time limit and the
# attempts after the first skip the Azure mirror. Only downloads are retried:
# dpkg runs once, from the local cache, so it is never cut off part-way.
set -euo pipefail
download=false
if [ "${1:-}" = --download ]; then
    download=true
    shift
fi
[ "$#" -gt 0 ] || {
    echo "usage: $0 [--download | apt-get install option...] PACKAGE..." >&2
    exit 2
}
export DEBIAN_FRONTEND=noninteractive
apt=(apt-get -q -o Acquire::Retries=2 -o Acquire::http::Timeout=30 -o Acquire::https::Timeout=30
    -o DPkg::Lock::Timeout=60)
if [ ! -f /etc/apt/sources.list.d/ubuntu.sources ]; then
    echo "::error title=Ubuntu packages::/etc/apt/sources.list.d/ubuntu.sources is missing; this helper refreshes only Ubuntu's own sources"
    exit 1
fi
update=("${apt[@]}" -o Dir::Etc::sourcelist=sources.list.d/ubuntu.sources -o Dir::Etc::sourceparts=-
    -o APT::Get::List-Cleanup=0)
mirrors=/etc/apt/apt-mirrors.txt
# The man-db trigger is slow on runners, and nothing here reads manual pages.
sudo rm -f /var/lib/man-db/auto-update
for attempt in 1 2 3; do
    if timeout -k 10 60 sudo -E "${update[@]}" update; then
        if $download; then
            timeout -k 10 90 "${apt[@]}" download "$@" && exit 0
        elif timeout -k 10 90 sudo -E "${apt[@]}" install -y --no-upgrade --download-only "$@"; then
            exec sudo -E "${apt[@]}" install -y --no-upgrade --no-download "$@"
        fi
    fi
    if [ "$attempt" -eq 3 ]; then
        echo "::error title=Ubuntu packages::Three downloads failed or timed out: $*"
        exit 1
    fi
    # Never leave the list empty: without another entry, use Ubuntu's own archive.
    if [ -f "$mirrors" ] && grep -q 'azure\.archive\.ubuntu\.com' "$mirrors"; then
        if grep -v 'azure\.archive\.ubuntu\.com' "$mirrors" | grep -q '[^[:space:]]'; then
            sudo sed -i '/azure\.archive\.ubuntu\.com/d' "$mirrors"
        else
            printf 'http://archive.ubuntu.com/ubuntu/\tpriority:1\n' | sudo tee "$mirrors" >/dev/null
        fi
    fi
    echo "Package download attempt $attempt failed or timed out; next from $(head -n 1 "$mirrors" 2>/dev/null || echo 'the same sources')."
    sleep 5
done
