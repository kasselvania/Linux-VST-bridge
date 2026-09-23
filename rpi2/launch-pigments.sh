#!/bin/sh
# Copy into the private Pigments comparison environment beside compatdata/.
set -eu
environment=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
runtime=$(CDPATH= cd -- "$environment/.." && pwd)
test "${WINEPREFIX:?supervisor selects the copied prefix}" = "$environment/compatdata/pfx"
test "$#" -gt 0
case "$1" in
  'C:\bridge\bin\wf0-factory-probe-rpi1-e232.exe'|'C:\bridge\bin\wf0-factory-probe-rpi2-state-recheck.exe'|'C:\Program Files (x86)\Arturia\Arturia Software Center\Arturia Software Center.exe') ;;
  *) exit 64;;
esac

cd "$runtime"
sha256sum --check --status <<'HASHES'
c0fe398829c78c82b26d39c20fbeb6b28d1e46941723962d794d7e5f67d402be  runners/GE-Proton11-7-aarch64/proton
ab9beb61c7c614caeb53e70ea0f4a1736d18b71eacb907ab09832db53cf6e46d  data/umu/steamrt4-arm64/_v2-entry-point
HASHES
export PYTHONPATH="$runtime/launcher/usr/lib/python3/dist-packages"
export XDG_DATA_HOME="$runtime/data" XDG_CACHE_HOME="$environment/cache"
export XDG_CONFIG_HOME="$environment/config" TMPDIR="$environment/tmp"
export PROTONPATH="$runtime/runners/GE-Proton11-7-aarch64"
export GAMEID=umu-default PROTONFIXES_DISABLE=1 UMU_RUNTIME_UPDATE=0 PROTON_USE_XALIA=0
exec /usr/bin/python3.13 "$runtime/launcher/usr/bin/umu-run" "$@"
