#!/usr/bin/env bash

set -Eeuo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=lib.sh
source "$SCRIPT_DIR/lib.sh"

usage() {
  printf 'usage: %s --user ORDINARY_USER --board-revision v1.8.1 --power-input pi\n' "$0" >&2
  exit 2
}

target_user=
board_revision=
power_input=
while [[ $# -gt 0 ]]; do
  case "$1" in
    --user)
      [[ $# -ge 2 ]] || usage
      target_user=$2
      shift 2
      ;;
    --board-revision) board_revision=${2:-}; shift 2 ;;
    --power-input) power_input=${2:-}; shift 2 ;;
    *) usage ;;
  esac
done
[[ -n $target_user && -n $board_revision && -n $power_input ]] || usage

require_root
require_platform
require_fixture_declaration "$board_revision" "$power_input"
[[ -f $FATES0_STATE_DIR/fixture-declaration.json ]] || die 'fixture declaration is absent; run prepare-buses.sh first'
python3 - "$FATES0_STATE_DIR/fixture-declaration.json" "$board_revision" "$power_input" <<'PY'
import json, sys
record = json.load(open(sys.argv[1]))
if record.get("board_revision") != sys.argv[2] or record.get("power_input") != sys.argv[3]:
    raise SystemExit("fixture declaration differs from stage 1")
PY
user_record=$(getent passwd "$target_user") || die "unknown user: $target_user"
target_uid=$(printf '%s\n' "$user_record" | cut -d: -f3)
[[ $target_uid -ge 1000 && $target_uid -lt 65534 ]] || die 'target must be an ordinary non-root user'
if [[ -e $FATES0_STATE_DIR/state.json ]]; then
  [[ ! -L $FATES0_STATE_DIR/state.json && -f $FATES0_STATE_DIR/state.json ]] || die 'foreign provisioning state path exists'
  recorded_user=$(python3 -c 'import json; print(json.load(open("/var/lib/fates0/state.json"))["target_user"])') ||
    die 'existing provisioning state is invalid'
  [[ $recorded_user == "$target_user" ]] || die "existing provisioning state belongs to a different target user: $recorded_user"
fi

config=$(boot_config_path)
refuse_boot_conflicts "$config"
boot_dir=$(dirname -- "$config")
fragment=$boot_dir/fates0.conf
stage_source=$SCRIPT_DIR/overlays/boot-buses.conf
platform_source=$SCRIPT_DIR/overlays/boot-platform.conf
[[ -f $fragment ]] || die 'managed boot fragment is absent; run prepare-buses.sh and reboot first'
[[ $(grep -Fxc 'include fates0.conf' "$config" || true) -eq 1 ]] || die 'managed boot include must appear exactly once'
fragment_hash=$(sha256_file "$fragment")
stage_hash=$(sha256_file "$stage_source")
platform_hash=$(sha256_file "$platform_source")
[[ $fragment_hash == "$stage_hash" || $fragment_hash == "$platform_hash" ]] ||
  die 'managed boot fragment has foreign content'
ensure_i2c_character_device 1
mkdir -p "$FATES0_STATE_DIR/evidence"
i2c_admission=$(mktemp "$FATES0_STATE_DIR/evidence/.i2c-address-admission.XXXXXX")
build_dir=
cleanup() {
  rm -f "$i2c_admission"
  [[ -z $build_dir ]] || rm -rf "$build_dir"
}
trap cleanup EXIT
if [[ $fragment_hash == "$stage_hash" || ! -e /sys/bus/i2c/devices/1-001a/driver ]]; then
  python3 "$SCRIPT_DIR/i2c_presence.py" >"$i2c_admission" ||
    die 'Fates codec did not acknowledge the bounded address-presence check at I2C 0x1a'
else
  printf '{"address":"0x1a","bus":1,"present":true,"admission":"bound-kernel-driver"}\n' >"$i2c_admission"
fi

note 'refreshing package metadata and admitting exact package candidates'
apt-get update
declare -A packages=(
  [evtest]='1:1.35-1+b1'
  [i2c-tools]='4.4-2'
  [jackd2]='1.9.22~dfsg-4'
  [jack-example-tools]='4-4'
)
declare -A package_hashes=(
  [evtest]='7fee662f317a35b27325b7beaf7c3e82ddabf508b5ab49e39ee83a34343338c4'
  [i2c-tools]='3123635e7da9c00d96daa24be09fda90c2f8e571702e4b515a910ee27b43d062'
  [jackd2]='9dfd3a8d4edfcb9f63be1c522eae2d187151ba1a77e650630ac990edb5318e2e'
  [jack-example-tools]='81ea08ad2768cf75dd45ee678f1e9d5580e82642f17867303777b7d727043212'
)
for package in evtest i2c-tools jackd2 jack-example-tools; do
  candidate=$(package_candidate "$package")
  [[ $candidate == "${packages[$package]}" ]] ||
    die "package candidate drift for $package: expected ${packages[$package]}, observed $candidate"
done
printf 'jackd2 jackd/tweak_rt_limits boolean false\n' | debconf-set-selections
declare -a download_specs=()
for package in evtest i2c-tools jackd2 jack-example-tools; do
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
  "evtest=${packages[evtest]}" "i2c-tools=${packages[i2c-tools]}" \
  "jackd2=${packages[jackd2]}" "jack-example-tools=${packages[jack-example-tools]}"

build_dir=$(mktemp -d /var/tmp/fates0-provision.XXXXXX)
"$SCRIPT_DIR/build-dtbo.sh" "$build_dir/fates0.dtbo" >"$build_dir/dtbo-hashes.txt"
[[ $(sha256_file "$build_dir/fates0.dtbo") == $(sha256_file "$SCRIPT_DIR/overlays/fates0.dtbo") ]] ||
  die 'locally rebuilt DTBO differs from the retained binary'
overlay_destination=$boot_dir/overlays/fates0.dtbo
install_exact "$SCRIPT_DIR/overlays/fates0.dtbo" "$overlay_destination" 0644
mkdir -p "$FATES0_STATE_DIR"
proto_overlay=$boot_dir/overlays/proto-codec.dtbo
[[ -f $proto_overlay && ! -L $proto_overlay ]] || die 'distribution proto-codec overlay is missing or redirected'
[[ $(sha256_file "$proto_overlay") == 1ec997e22f82880c2540cec822da3d96cebc0a1aebd8626481000ff8d140a754 ]] ||
  die 'distribution proto-codec overlay differs from the pinned kernel package'
modinfo snd_soc_wm8731 >/dev/null || die 'distribution WM8731 codec module is unavailable'
modinfo snd_soc_wm8731_i2c >/dev/null || die 'distribution WM8731 I2C module is unavailable'
modinfo snd_soc_rpi_proto >/dev/null || die 'distribution Raspberry Pi PROTO machine driver is unavailable'
install_exact "$SCRIPT_DIR/config/99-fates0.rules" /etc/udev/rules.d/99-fates0.rules 0644
install_exact "$SCRIPT_DIR/config/99-fates0-alsa.conf" /etc/alsa/conf.d/99-fates0.conf 0644
install_exact "$SCRIPT_DIR/config/fates0-modules.conf" /etc/modules-load.d/fates0.conf 0644
install_exact "$SCRIPT_DIR/config/jack.env" "$FATES0_CONFIG_DIR/jack.env" 0644
install_exact "$SCRIPT_DIR/config/99-fates0-limits.conf" /etc/security/limits.d/99-fates0.conf 0644
install_exact "$SCRIPT_DIR/systemd/fates-jack@.service" /etc/systemd/system/fates-jack@.service 0644
install_exact "$SCRIPT_DIR/systemd/fates-oled@.service" /etc/systemd/system/fates-oled@.service 0644
for program in controls.py oled_service.py oled_client.py audio_probe.py audio-test.sh mixer-state.sh observed-run.sh thermal-observe.sh; do
  install_exact "$SCRIPT_DIR/$program" "/usr/local/libexec/fates0/$program" 0755
done

for group in audio input spi gpio; do
  getent group "$group" >/dev/null || die "required least-privilege group is missing: $group"
done
usermod -a -G audio,input,spi,gpio "$target_user"

install -m 0644 "$platform_source" "$fragment"
udevadm control --reload-rules
systemctl daemon-reload

mv "$i2c_admission" "$FATES0_STATE_DIR/evidence/i2c-address-admission.json"
i2c_admission=$FATES0_STATE_DIR/evidence/.consumed
dpkg-query -W -f='${binary:Package}\t${Version}\t${Architecture}\n' | LC_ALL=C sort >"$FATES0_STATE_DIR/evidence/package-manifest.tsv"
grep -RhE '^[[:space:]]*(deb|URIs:|Suites:|Components:)' /etc/apt/sources.list /etc/apt/sources.list.d 2>/dev/null >"$FATES0_STATE_DIR/evidence/package-repositories.txt" || true

provisioning_commit=unknown
if git -C "$SCRIPT_DIR" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  provisioning_commit=$(git -C "$SCRIPT_DIR" rev-parse HEAD)
fi
python3 - "$FATES0_STATE_DIR/state.json" "$target_user" "$provisioning_commit" "$board_revision" "$power_input" <<'PY'
import json, pathlib, sys
destination, user, commit, revision, power = sys.argv[1:]
pathlib.Path(destination).write_text(json.dumps({
    "schema_version": 1,
    "target_user": user,
    "provisioning_commit": commit,
    "board_revision": revision,
    "power_input": power,
    "audio_driver_source": "distribution",
    "audio_overlay": "proto-codec",
    "reboot_required": True,
}, indent=2, sort_keys=True) + "\n")
PY

note 'full platform stage installed; JACK and OLED services remain disabled for ordered ALSA verification'
note 'reboot is required before hardware verification; no reboot was initiated'
