#!/bin/sh
# Private Pi fixture adapter. Copy beside the staged RPI2 runner and prefix.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
test "${WINEPREFIX:?supervisor must select its prefix}" = "$root/prefix-ge-01"
test "$#" -gt 0

# Preserve Windows console output and avoid UMU's Unix-path game launcher.
case "$1" in
    "$WINEPREFIX/drive_c/"*)
        relative=${1#"$WINEPREFIX/drive_c/"}
        windows=$(printf '%s' "$relative" | tr '/' '\\')
        shift
        set -- "C:\\$windows" "$@"
        ;;
    *) printf '%s\n' 'RPI2 executable must be inside its private C: drive' >&2; exit 64;;
esac

cd "$root"
sha256sum --check --status <<'HASHES'
c0fe398829c78c82b26d39c20fbeb6b28d1e46941723962d794d7e5f67d402be  runners/GE-Proton11-7-aarch64/proton
ab9beb61c7c614caeb53e70ea0f4a1736d18b71eacb907ab09832db53cf6e46d  data/umu/steamrt4-arm64/_v2-entry-point
HASHES
export PYTHONPATH="$root/launcher/usr/lib/python3/dist-packages"
export XDG_DATA_HOME="$root/data" XDG_CACHE_HOME="$root/cache" XDG_CONFIG_HOME="$root/config"
export PROTONPATH="$root/runners/GE-Proton11-7-aarch64"
export GAMEID=umu-default PROTONFIXES_DISABLE=1 UMU_RUNTIME_UPDATE=0 PROTON_USE_XALIA=0
export DISPLAY=:1 XAUTHORITY="$HOME/.Xauthority"
exec /usr/bin/python3.13 "$root/launcher/usr/bin/umu-run" "$@"
