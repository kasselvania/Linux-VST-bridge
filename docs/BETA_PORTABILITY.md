# Beta portability: package and explicit first run

## Source basis and one claim

This owner begins at post-PKG1 canonical main
`3dd598fb46a4fd3de909c658b289246e5e24b1f9`, tree
`dd83bae6201060725ac76c0f7fdac34a7e465881`. It follows
`AGENTS.md` (Mission, Flatpak and SteamOS rules, Security and privacy),
`GOVERNANCE.md` (What evidence means), `docs/ARCHITECTURE.md` (software
generation and rollback), and the reviewed [PKG0](PKG0.md) and
[PKG1](PKG1_NATIVE_KIT.md) package owners.
The branch later merged post-product-controls main
`de70bd29fed59140ff0396cfc1696cf56db7de75` before the graphical
Ubuntu package test.

One exact PKG0/PKG1 payload roster can be placed in an Arch package or an
amd64 Debian package. A fixed system desktop entry launches the packaged
frontend. Package installation does not select user software, run a setup
hook, start a service, create an environment, or change a publication.
The user explicitly adopts a verified immutable generation, then explicitly
starts its exact selected user service. The paired frontend presents those
actions in a fixed first-run screen. The actual graphical result and its
remaining limits are recorded below.

## Distribution boundary

| System | Package route | Evidence here | Product support claim |
| --- | --- | --- | --- |
| SteamOS | Reviewed Arch package source and user-owned PKG0/PKG1 adoption | Source tests only; working Deck unchanged | Existing installed product evidence remains separate |
| Ubuntu 26.04.1 LTS, amd64 | Deterministic `.deb` from the same release manifest and payload | A clean graphical VM installed an internal-test package, opened the Applications entry, explicitly adopted and activated an immutable generation, reopened after a reboot, and truthfully reported that compatibility was unqualified. The official Bitwig 6.1.1 package installed and reached its user-owned EULA | No customer release, plug-in, audio, GPU, vendor-authorization or Ubuntu compatibility qualification |
| Debian 13.7, amd64 | Target-specific `.deb` build from the same source and roster | Synthetic package install/dependency check passed. A Debian 13 target build of the current canonical source produced manager/frontend binaries requiring at most GLIBC 2.39 and Python 3.13 bytecode; manager startup reached the expected missing-state refusal | The Ubuntu Python 3.14 bytecode and frontend must not be reused; no complete Debian package, adopted product, audio or graphical qualification |
| CachyOS 260809 ISO, rolling amd64 | Existing Arch package format from the same roster | Disposable 40 GiB VM reached graphical KDE Plasma 6.7.5 login on Wayland, kernel 7.2.8-1-cachyos. Inert source-owned Arch fixture installed, removed, and reinstalled through pacman; `pacman -Qk` found 17/17 files and the KDE launcher was discoverable. A user-owned marker survived removal and reinstall | No executable launch, adoption, Bitwig, audio or GPU qualification |

The [CachyOS graphical receipt](../evidence/beta-portability/cachyos-graphical-2026-09-28.json)
establishes an installed desktop and inert package/launcher fixture only. The
[Ubuntu graphical receipt](../evidence/beta-portability/ubuntu-first-run-2026-09-29.json)
records the later real-binary internal-test package journey on a clean 40 GiB
VM overlay. It is separate from the earlier container checks. Package parsing,
dependency resolution, application launch and truthful first-run diagnosis are
not first sound. A managed plug-in, audio, editor and project workflow remains
required before qualifying Ubuntu. Debian still needs a target-specific signed
package and graphical installation; CachyOS still needs executable package
first run.
The bounded [container receipt](../evidence/beta-portability/portable-containers-2026-09-28.json)
separates these package facts from product startup.
The [paired-binary receipt](../evidence/beta-portability/paired-binary-startup-2026-09-28.json)
records a separate limited check: the exact Linux manager and frontend had no
unresolved dynamic libraries in those containers, and the manager returned its
expected `package_not_installed` refusal from the fixed activation-status
entry point. The frontend was not launched graphically and neither binary was
adopted as a product generation.

## Package authority

- `tools/pkg0/assemble.py` owns the exact file roster, paired manager/frontend
  source generation, PKG1 preparation kit, external Proton/SLR identity, and
  `RELEASE_MANIFEST.json`. Arch assembly reads both exact packaged supervisor
  bytecode headers and declares a bounded matching `python` minor range in
  `PKGBUILD`. The Arch package verifier requires the same range in `.PKGINFO`;
  mixed, malformed or unrecognized bytecode refuses. The installed target's
  Python magic must still be read back before adoption. A rolling-distribution
  package with a different interpreter needs a target-specific build from the
  same reviewed source, rather than reusing incompatible bytecode.
- The package-generated desktop file has a fixed `/usr/bin` frontend target.
  It does not call adoption or start a service.
- `tools/portable/deb.py` verifies that manifest and payload, builds a
  deterministic Debian archive, adds only necessary root-owned parent
  directories, and verifies its complete control and data rosters. It refuses
  maintainer scripts and changed dependencies, including the declared Python
  minor range derived from both packaged supervisor headers. Its build,
  verify and signing paths use PKG0's shared destination, role, component,
  mode, generated-file and preparation-kit roster law; an extra command
  cannot be signed as a document. It does not add a second
  compatibility or software-state authority.
- `tools/portable/release.py` signs a Debian bundle only with an externally
  supplied key. A `release` signature requires the PKG1 kit, exact clean
  source tree, and source-rebuilt native backend. Verification requires a
  separately supplied trusted public key and expected fingerprint; the key
  copied into the bundle is informational. Internal-test signatures have no
  customer release authority. No product release key is selected here.
- Proton/SLR remains an externally installed, exactly verified prerequisite.
  No package in this owner redistributes it, a vendor plug-in, or a license.
- The PKG1 preparation kit contains first-party proxy sources and its exact
  host/backend authority. Proxy construction still requires the pinned VST3 SDK
  checkout and `org.freedesktop.Sdk//25.08` on the target. The current package
  does not install or supply either prerequisite. Clean-machine first sound is
  blocked until their lawful installation and verification become a reviewed
  customer path; manually seeding a development cache is not acceptance.

## First-run service boundary

`/usr/bin/linux-vst-bridge package-adopt` remains the reviewed guarded
generation selection. A later explicit `package-activate` verifies that the
selected immutable generation, six user routes, loaded unit `FragmentPath`
and effective `ExecStart` agree; it refuses pending transactions, active
owners, cleanup uncertainty and ambiguous service readback. It calls bounded
`systemctl --user enable --now` only after the exact checks and then requires
an active matching unit and healthy owner readback. The read-only
`package-activation-status` emits schema-1 JSON with only `active` or
`inactive` and the selected package version; broken authority refuses.
An active exact service is idempotent. The action holds no registry or
service lock while systemd starts the service.

The fixed system frontend uses the separate read-only
`package-bootstrap-status` schema-2 posture for first-run buttons. It calls a
selected generation `legacy_adoptable` only when `software.json`, the original
`session.py`/`ownership.py` layout, the installed `/usr` package intake and
the predecessor route plan all verify. A selected package generation with a
missing or changed generation record is a refusal. For a missing or stale
package-owned route, `repair_active` requires the loaded unit to execute the
exact selected manager and clean idle ownership. The explicit
`package-stop-for-repair` command repeats those checks, stops only that user
unit under a bounded systemd call and verifies it retired before package
adoption may repair routes. A foreign route or loaded service remains a
refusal. Neither status nor page load stops a service.

If installation, adoption, activation or a later package update fails, the
user-owned software record and package journal retain the exact owner of the
next recovery. Package rollback remains separate from a native-proxy
publication rollback. Uninstalling the system package does not remove
user-owned generations, plug-in state, authorization, projects or prefixes.

## Reproducible source checks

Build a staged package with `tools/pkg0/assemble.py` from a clean exact source
checkout and a complete spec. Then run:

```text
python3 tools/portable/deb.py build --staged STAGED --package OUTPUT.deb --epoch SOURCE_EPOCH
python3 tools/portable/deb.py verify --staged STAGED --package OUTPUT.deb
python3 tools/portable/release.py sign --package OUTPUT.deb --staged STAGED --gpg-home ISOLATED_KEY_HOME --fingerprint FINGERPRINT --key-class release --source CLEAN_SOURCE --output SIGNED_BUNDLE
python3 tools/portable/release.py verify --bundle SIGNED_BUNDLE --trusted-key SEPARATE_TRUSTED_KEY --fingerprint FINGERPRINT
```

The output path must use the exact versioned Debian filename. The source
signing key and release distribution decision stay outside this repository.
The internal-test fixture uses inert stand-in executable bytes and cannot
be installed as a customer build. An actual beta package requires real
paired binaries, host, complete kit, native proxy, profile, notices and
license roster from one reviewed source generation.

## Acceptance still required

On Ubuntu and CachyOS clean graphical machines: install the real signed
package, open it from Applications, explicitly adopt/start the selected
generation, see a truthful first-run diagnosis, install and authorize one
owned plug-in, publish through the manager, use it in Bitwig, save/reopen,
reboot/reopen, and retire cleanly. Debian has a separate package/dependency/
startup gate; no Debian audio support follows from it. Physical latency and
GPU behavior need physical hardware evidence. The current Deck is not
changed by this source owner.
