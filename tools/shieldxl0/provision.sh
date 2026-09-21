#!/usr/bin/env bash

set -Eeuo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=lib.sh
source "$SCRIPT_DIR/lib.sh"

usage() {
  printf 'usage: %s --user ORDINARY_USER\n' "$0" >&2
  exit 2
}

target_user=
while [[ $# -gt 0 ]]; do
  case "$1" in
    --user)
      [[ $# -ge 2 ]] || usage
      target_user=$2
      shift 2
      ;;
    *) usage ;;
  esac
done
[[ -n $target_user ]] || usage

require_root
require_platform
user_record=$(getent passwd "$target_user") || die "unknown user: $target_user"
target_uid=$(printf '%s\n' "$user_record" | cut -d: -f3)
[[ $target_uid -ge 1000 && $target_uid -lt 65534 ]] || die 'target must be an ordinary non-root user'
if [[ -e $SHIELDXL0_STATE_DIR/state.json ]]; then
  [[ ! -L $SHIELDXL0_STATE_DIR/state.json && -f $SHIELDXL0_STATE_DIR/state.json ]] || die 'foreign provisioning state path exists'
  recorded_user=$(python3 -c 'import json; print(json.load(open("/var/lib/shieldxl0/state.json"))["target_user"])') ||
    die 'existing provisioning state is invalid'
  [[ $recorded_user == "$target_user" ]] || die "existing provisioning state belongs to a different target user: $recorded_user"
fi

config=$(boot_config_path)
refuse_boot_conflicts "$config"
boot_dir=$(dirname -- "$config")
fragment=$boot_dir/shieldxl0.conf
stage_source=$SCRIPT_DIR/overlays/boot-buses.conf
platform_source=$SCRIPT_DIR/overlays/boot-platform.conf
[[ -f $fragment ]] || die 'managed boot fragment is absent; run prepare-buses.sh and reboot first'
fragment_hash=$(sha256_file "$fragment")
stage_hash=$(sha256_file "$stage_source")
platform_hash=$(sha256_file "$platform_source")
[[ $fragment_hash == "$stage_hash" || $fragment_hash == "$platform_hash" ]] ||
  die 'managed boot fragment has foreign content'
ensure_i2c_character_device 1
mkdir -p "$SHIELDXL0_STATE_DIR/evidence"
i2c_admission=$(mktemp "$SHIELDXL0_STATE_DIR/evidence/.i2c-address-admission.XXXXXX")
build_dir=
cleanup() {
  rm -f "$i2c_admission"
  [[ -z $build_dir ]] || rm -rf "$build_dir"
}
trap cleanup EXIT
if [[ $fragment_hash == "$stage_hash" || ! -e /sys/bus/i2c/devices/1-0048/driver ]]; then
  python3 "$SCRIPT_DIR/i2c_presence.py" >"$i2c_admission" ||
    die 'ShieldXL codec did not acknowledge the bounded address-presence check at I2C 0x48'
else
  printf '{"address":"0x48","bus":1,"present":true,"admission":"bound-kernel-driver"}\n' >"$i2c_admission"
fi

note 'refreshing package metadata and admitting exact package candidates'
apt-get update
declare -A packages=(
  [evtest]='1:1.35-1+b1'
  [i2c-tools]='4.4-2'
  [jackd2]='1.9.22~dfsg-4'
)
declare -A package_hashes=(
  [evtest]='7fee662f317a35b27325b7beaf7c3e82ddabf508b5ab49e39ee83a34343338c4'
  [i2c-tools]='3123635e7da9c00d96daa24be09fda90c2f8e571702e4b515a910ee27b43d062'
  [jackd2]='9dfd3a8d4edfcb9f63be1c522eae2d187151ba1a77e650630ac990edb5318e2e'
)
for package in evtest i2c-tools jackd2; do
  candidate=$(package_candidate "$package")
  [[ $candidate == "${packages[$package]}" ]] ||
    die "package candidate drift for $package: expected ${packages[$package]}, observed $candidate"
done
printf 'jackd2 jackd/tweak_rt_limits boolean false\n' | debconf-set-selections
declare -a download_specs=()
for package in evtest i2c-tools jackd2; do
  installed_version=$(dpkg-query -W -f='${Version}' "$package" 2>/dev/null || true)
  [[ $installed_version == "${packages[$package]}" ]] || download_specs+=("$package=${packages[$package]}")
done
if [[ ${#download_specs[@]} -gt 0 ]]; then
  DEBIAN_FRONTEND=noninteractive apt-get install -y --download-only --no-install-recommends "${download_specs[@]}"
fi
for package_spec in "${download_specs[@]}"; do
  package=${package_spec%%=*}
  admitted_deb=
  while IFS= read -r candidate_deb; do
    if [[ $(dpkg-deb -f "$candidate_deb" Package) == "$package" && $(dpkg-deb -f "$candidate_deb" Version) == "${packages[$package]}" ]]; then
      admitted_deb=$candidate_deb
      break
    fi
  done < <(find /var/cache/apt/archives -maxdepth 1 -type f -name '*.deb' -print)
  [[ -n $admitted_deb ]] || die "downloaded artifact is missing for $package=${packages[$package]}"
  [[ $(sha256_file "$admitted_deb") == "${package_hashes[$package]}" ]] ||
    die "downloaded artifact hash differs for $package=${packages[$package]}"
done
DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
  "evtest=${packages[evtest]}" "i2c-tools=${packages[i2c-tools]}" "jackd2=${packages[jackd2]}"

build_dir=$(mktemp -d /var/tmp/shieldxl0-provision.XXXXXX)
"$SCRIPT_DIR/build-dtbo.sh" "$build_dir/shieldxl0.dtbo" >"$build_dir/dtbo-hashes.txt"
[[ $(sha256_file "$build_dir/shieldxl0.dtbo") == $(sha256_file "$SCRIPT_DIR/overlays/shieldxl0.dtbo") ]] ||
  die 'locally rebuilt DTBO differs from the retained binary'
overlay_destination=$boot_dir/overlays/shieldxl0.dtbo
module_destination=/lib/modules/$SHIELDXL0_KERNEL/updates/shieldxl0/snd-soc-cs4270.ko
install_exact "$SCRIPT_DIR/overlays/shieldxl0.dtbo" "$overlay_destination" 0644
mkdir -p "$SHIELDXL0_STATE_DIR"
if [[ -f $module_destination ]]; then
  [[ -f $SHIELDXL0_STATE_DIR/cs4270-module.sha256 ]] || die "foreign codec module exists: $module_destination"
  [[ $(sha256_file "$module_destination") == $(cat "$SHIELDXL0_STATE_DIR/cs4270-module.sha256") ]] ||
    die "installed codec module differs from its ownership record: $module_destination"
else
  "$SCRIPT_DIR/build-cs4270-module.sh" "$build_dir/snd-soc-cs4270.ko" >"$build_dir/module-hash.txt"
  install -D -m 0644 "$build_dir/snd-soc-cs4270.ko" "$module_destination"
  sha256_file "$module_destination" >"$SHIELDXL0_STATE_DIR/cs4270-module.sha256"
fi
install_exact "$SCRIPT_DIR/config/99-shieldxl0.rules" /etc/udev/rules.d/99-shieldxl0.rules 0644
install_exact "$SCRIPT_DIR/config/99-shieldxl0-alsa.conf" /etc/alsa/conf.d/99-shieldxl0.conf 0644
install_exact "$SCRIPT_DIR/config/shieldxl0-modules.conf" /etc/modules-load.d/shieldxl0.conf 0644
install_exact "$SCRIPT_DIR/config/jack.env" "$SHIELDXL0_CONFIG_DIR/jack.env" 0644
install_exact "$SCRIPT_DIR/config/99-shieldxl0-limits.conf" /etc/security/limits.d/99-shieldxl0.conf 0644
install_exact "$SCRIPT_DIR/systemd/shieldxl-jack@.service" /etc/systemd/system/shieldxl-jack@.service 0644
install_exact "$SCRIPT_DIR/systemd/shieldxl-oled@.service" /etc/systemd/system/shieldxl-oled@.service 0644
for program in controls.py oled_service.py oled_client.py audio_probe.py audio-test.sh mixer-state.sh observed-run.sh thermal-observe.sh; do
  install_exact "$SCRIPT_DIR/$program" "/usr/local/libexec/shieldxl0/$program" 0755
done

for group in audio input spi gpio; do
  getent group "$group" >/dev/null || die "required least-privilege group is missing: $group"
done
usermod -a -G audio,input,spi,gpio "$target_user"

install -m 0644 "$platform_source" "$fragment"
depmod "$SHIELDXL0_KERNEL"
udevadm control --reload-rules
systemctl daemon-reload

mv "$i2c_admission" "$SHIELDXL0_STATE_DIR/evidence/i2c-address-admission.json"
i2c_admission=$SHIELDXL0_STATE_DIR/evidence/.consumed
dpkg-query -W -f='${binary:Package}\t${Version}\t${Architecture}\n' | LC_ALL=C sort >"$SHIELDXL0_STATE_DIR/evidence/package-manifest.tsv"
grep -RhE '^[[:space:]]*(deb|URIs:|Suites:|Components:)' /etc/apt/sources.list /etc/apt/sources.list.d 2>/dev/null >"$SHIELDXL0_STATE_DIR/evidence/package-repositories.txt" || true

provisioning_commit=unknown
if git -C "$SCRIPT_DIR" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  provisioning_commit=$(git -C "$SCRIPT_DIR" rev-parse HEAD)
fi
module_sha256=$(cat "$SHIELDXL0_STATE_DIR/cs4270-module.sha256")
python3 - "$SHIELDXL0_STATE_DIR/state.json" "$target_user" "$provisioning_commit" "$module_sha256" <<'PY'
import json, pathlib, sys
destination, user, commit, module_sha256 = sys.argv[1:]
pathlib.Path(destination).write_text(json.dumps({
    "schema_version": 1,
    "target_user": user,
    "provisioning_commit": commit,
    "cs4270_module_sha256": module_sha256,
    "reboot_required": True,
}, indent=2, sort_keys=True) + "\n")
PY

note 'full platform stage installed; JACK and OLED services remain disabled for ordered ALSA verification'
note 'reboot is required before hardware verification; no reboot was initiated'
