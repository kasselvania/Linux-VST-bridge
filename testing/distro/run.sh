#!/usr/bin/env bash
# Build and test the bridge on one Linux distribution, inside a disposable
# container of that distribution.
#
# Start it as root with the checkout as the working directory. It installs the
# distribution's own build tools and Python (never a pyenv or setup-python
# interpreter), creates an ordinary user that owns the checkout (some tests
# refuse root), and runs every check as that user. Each check runs even if an
# earlier one failed, and the script fails at the end if any did.
# .github/workflows/distro-matrix.yml runs it in CI.
#
# The per-user runtime directory must already be a tmpfs owned by the
# checkout's owner, as a login session provides it. With rootful Docker on
# Linux, from the checkout:
#
#   uid=$(stat -c %u .)
#   docker run --rm --init -v "$PWD:/src" -w /src \
#     --tmpfs "/run/user/$uid:exec,mode=0700,uid=$uid,gid=$uid" \
#     fedora:44 bash testing/distro/run.sh [check ...]
#
# A checkout that is root-owned inside the container, as rootless Docker or
# Podman shows your own files, is refused: its ownership would have to change.
# Copy the checkout into the container instead, or set
# LVB_DISTRO_DISPOSABLE_CHECKOUT=1 when it is a throwaway copy; it is then
# handed to uid 1000 (use --tmpfs /run/user/1000:...). Name checks to run only
# those (see CHECKS below). Cargo's home and target directory live under
# LVB_DISTRO_CACHE (default /cache) so CI can keep them between runs. Versions
# and check times are appended as Markdown to LVB_DISTRO_SUMMARY when it is set.
set -euo pipefail

RUST_TOOLCHAIN=1.95.0 # as ap12-manager.yml and tools/pkg0 (KIT_RUST_TOOLCHAIN)
TEST_USER=lvbtest
CHECKS=(toolchain python build model manager real-service ui supervisor pkg0 root-intake portable)

# shellcheck disable=SC1091
. /etc/os-release
DISTRO="${LVB_DISTRO_NAME:-$PRETTY_NAME}"

install_distribution_packages() {
    case "$ID" in
    cachyos | arch)
        # Arch supports only whole upgrades; this is the system a user has.
        pacman -Syu --noconfirm --needed base-devel git python gnupg libarchive \
            file binutils zstd openssl curl iproute2 procps-ng which util-linux rustup
        ;;
    fedora)
        # systemd and curl are on every Fedora desktop and in the product's
        # declared dependencies on the other distributions. A mirror slower
        # than 256 KiB/s for 20 s is dropped for the next one.
        dnf -y --setopt=install_weak_deps=False --setopt=minrate=262144 \
            --setopt=timeout=20 install gcc make \
            pkgconf-pkg-config git-core python3 gnupg2 bsdtar file binutils \
            zstd openssl sudo curl iproute procps-ng which util-linux \
            shadow-utils findutils tar xz diffutils systemd rustup
        ;;
    *)
        echo "::error title=$DISTRO::No package recipe for distribution '$ID'"
        exit 2
        ;;
    esac
}

as_root() {
    echo "::group::Install $DISTRO packages"
    install_distribution_packages
    echo "::endgroup::"

    local uid
    uid=$(stat -c %u .)
    if [[ $uid == 0 ]]; then
        if [[ ${LVB_DISTRO_DISPOSABLE_CHECKOUT:-} != 1 ]]; then
            echo "::error title=$DISTRO::The checkout is root-owned here, as rootless Docker or Podman shows a bind-mounted checkout. Copy it into the container, or set LVB_DISTRO_DISPOSABLE_CHECKOUT=1 if it is a throwaway copy that may be handed to uid 1000."
            exit 2
        fi
        uid=1000
        chown -R "$uid:$uid" .
    fi
    getent group "$TEST_USER" >/dev/null || groupadd -o -g "$uid" "$TEST_USER"
    getent passwd "$TEST_USER" >/dev/null ||
        useradd -o -m -u "$uid" -g "$uid" -s /bin/bash "$TEST_USER"
    # tools/pkg0/root_intake_fixture.py sets up root-owned fixtures with sudo,
    # as on a developer machine or a GitHub runner.
    echo "$TEST_USER ALL=(ALL) NOPASSWD: ALL" >"/etc/sudoers.d/$TEST_USER"
    chmod 0440 "/etc/sudoers.d/$TEST_USER"

    local runtime="/run/user/$uid"
    if [[ $(stat -f -c %T "$runtime" 2>/dev/null) != tmpfs || $(stat -c %u "$runtime") != "$uid" ]]; then
        echo "::error title=$DISTRO::$runtime must be a tmpfs owned by uid $uid (docker run --tmpfs \"$runtime:exec,mode=0700,uid=$uid,gid=$uid\")"
        exit 2
    fi

    local cache="${LVB_DISTRO_CACHE:-/cache}"
    install -d -o "$uid" -g "$uid" "$cache" "$cache/cargo" "$cache/target"
    local home
    home=$(getent passwd "$TEST_USER" | cut -d: -f6)
    exec runuser -u "$TEST_USER" -- env -i \
        HOME="$home" USER="$TEST_USER" LOGNAME="$TEST_USER" SHELL=/bin/bash \
        LANG=C.UTF-8 XDG_RUNTIME_DIR="$runtime" \
        PATH="$cache/cargo/bin:/usr/local/bin:/usr/bin:/bin:/usr/local/sbin:/usr/sbin:/sbin" \
        CARGO_HOME="$cache/cargo" CARGO_TARGET_DIR="$cache/target" RUSTUP_HOME="$home/.rustup" \
        CARGO_INCREMENTAL=0 CARGO_TERM_COLOR=always CI="${CI:-}" \
        LVB_DISTRO_NAME="$DISTRO" LVB_DISTRO_SUMMARY="${LVB_DISTRO_SUMMARY:-}" \
        bash "$0" --as-user "$@"
}

# rustup is the distribution's: Arch packages rustup itself, and Fedora
# packages rustup-init, which installs it into this user's CARGO_HOME.
check_toolchain() {
    if ! command -v rustup >/dev/null; then
        rustup-init -y --no-modify-path --profile minimal --default-toolchain none
    fi
    rustup toolchain install "$RUST_TOOLCHAIN" --profile minimal
    rustup default "$RUST_TOOLCHAIN"
}

# The product's own admission probe (crates/service/src/supervisor.rs,
# verify_host_python): the embedded bootstrap run by /usr/bin/python3.
check_python() {
    local range version probe errors status=0
    range=$(sed -n 's/^pub const SOURCE_PYTHON_RANGE: &str = "\(.*\)";$/\1/p' crates/service/src/supervisor.rs)
    version=$(/usr/bin/python3 -c 'import platform; print(platform.python_implementation(), platform.python_version())')
    errors=$(mktemp)
    probe=$(/usr/bin/python3 -I -S -B -c "$(cat bridge-manager/runtime/bootstrap.py)" --probe 2>"$errors") || status=$?
    if [[ $status == 0 && $probe == cpython-source-v1 ]]; then
        echo "$version at /usr/bin/python3 is inside the supervisor's declared range, $range."
        rm -f "$errors"
        return 0
    fi
    local reason
    reason=$(head -c 400 "$errors" | tr '\n' ' ')
    rm -f "$errors"
    [[ -n $reason ]] || reason="it printed \"$probe\""
    echo "::error title=$DISTRO ships an unsupported host Python::$DISTRO ships $version at /usr/bin/python3, outside the supervisor's declared range ($range). The product refuses to start plug-in hosts on it (probe exit $status): $reason"
    return 1
}

check_build() {
    cargo build --release --locked --manifest-path crates/cli/Cargo.toml --bin linux-vst-bridge
    cargo build --release --locked --manifest-path manager-ui/Cargo.toml --bins
}

# --no-fail-fast: one failing test binary must not hide the next one's results.
check_model() {
    cargo test --locked --no-fail-fast --manifest-path crates/model/Cargo.toml
    cargo test --locked --no-fail-fast --manifest-path crates/view-model/Cargo.toml
}

check_manager() {
    cargo test --locked --no-fail-fast --manifest-path crates/service/Cargo.toml --lib
    cargo test --locked --no-fail-fast --manifest-path crates/cli/Cargo.toml --bins
}

# The real `serve` on a fixture state, with the stand-in systemd-run and
# systemctl the test itself installs (crates/cli/tests/real_service).
check_real_service() {
    cargo test --locked --no-fail-fast --manifest-path crates/cli/Cargo.toml --features service-executor-fixture --test real_service
}

check_ui() {
    cargo test --locked --no-fail-fast --manifest-path manager-ui/Cargo.toml
}

check_supervisor() {
    python3 -m unittest discover -s bridge-manager/runtime -v
}

check_pkg0() {
    # The signing verifier rebuilds the backend offline from its own lockfile.
    cargo fetch --manifest-path native-vst3-proxy/backend/Cargo.toml --locked --target x86_64-unknown-linux-gnu
    python3 -m unittest discover -s tools/pkg0 -v
}

check_root_intake() {
    python3 tools/pkg0/root_intake_fixture.py
}

check_portable() {
    python3 -m unittest discover -s tools/portable -v
}

summary() {
    [[ -n ${LVB_DISTRO_SUMMARY:-} ]] && printf '%s\n' "$@" >>"$LVB_DISTRO_SUMMARY"
    return 0
}

first_line() {
    "$@" 2>&1 | head -n 1 || true
}

as_user() {
    local selected=("$@")
    [[ ${#selected[@]} -gt 0 ]] || selected=("${CHECKS[@]}")
    local name failed=() results=()
    for name in "${selected[@]}"; do
        [[ " ${CHECKS[*]} " == *" $name "* ]] || { echo "unknown check '$name'" >&2; exit 2; }
    done

    summary "### $DISTRO" "" "| | |" "|---|---|" \
        "| Distribution | $PRETTY_NAME (${VERSION_ID:-${BUILD_ID:-}}), $(uname -m) |" \
        "| Kernel (runner) | $(uname -r) |" \
        "| Python | $(first_line /usr/bin/python3 --version) |" \
        "| C compiler | $(first_line gcc --version) |" \
        "| C library | $(first_line ldd --version) |" \
        "| OpenSSL / GnuPG | $(first_line openssl version) / $(first_line gpg --version) |"

    for name in "${selected[@]}"; do
        local started status=0
        started=$(date +%s)
        echo "::group::$name"
        # A child shell, so that errexit holds inside the check.
        bash "$0" --check "$name" || status=$?
        echo "::endgroup::"
        local took=$(($(date +%s) - started))
        if [[ $status == 0 ]]; then
            results+=("| $name | passed | ${took}s |")
            echo "$name: passed in ${took}s"
        else
            failed+=("$name")
            results+=("| $name | **failed** (exit $status) | ${took}s |")
            echo "::error title=$DISTRO: $name failed::$name exited with $status after ${took}s"
        fi
        if [[ $name == toolchain && $status == 0 ]]; then
            summary "| Rust | $(first_line rustc -V), $(first_line cargo -V) |"
        fi
    done

    summary "" "| Check | Result | Time |" "|---|---|---|" "${results[@]}" ""
    if [[ ${#failed[@]} -gt 0 ]]; then
        echo "Failed on $DISTRO: ${failed[*]}"
        exit 1
    fi
    echo "Every check passed on $DISTRO."
}

if [[ ${1:-} == --check ]]; then
    "check_${2//-/_}"
elif [[ ${1:-} == --as-user ]]; then
    shift
    as_user "$@"
elif [[ $(id -u) == 0 ]]; then
    as_root "$@"
else
    echo "Start as root in a disposable container; see the header of $0." >&2
    exit 2
fi
