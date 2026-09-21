#!/bin/sh
set -eu

if [ "$#" -lt 3 ] || [ "$1" != "--verb=run" ] || [ "$2" != "--" ]; then
    exit 64
fi
shift 2
case "$1" in
    /*/proton) ;;
    *) exit 64 ;;
esac
exec "$@"
