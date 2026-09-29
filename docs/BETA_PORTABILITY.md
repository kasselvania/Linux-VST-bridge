# Beta portability: package and explicit first run

## Source basis and one claim

This owner begins at post-PKG1 canonical main
`3dd598fb46a4fd3de909c658b289246e5e24b1f9`, tree
`dd83bae6201060725ac76c0f7fdac34a7e465881`. It follows
`AGENTS.md` (Mission, Flatpak and SteamOS rules, Security and privacy),
`GOVERNANCE.md` (What evidence means), `docs/ARCHITECTURE.md` (software
generation and rollback), and the reviewed [PKG0](PKG0.md) and
[PKG1](PKG1_NATIVE_KIT.md) package owners.

One exact PKG0/PKG1 payload roster can be placed in an Arch package or an
amd64 Debian package. A fixed system desktop entry launches the packaged
frontend. Package installation does not select user software, run a setup
hook, start a service, create an environment, or change a publication.
The user explicitly adopts a verified immutable generation, then explicitly
starts its exact selected user service. The fixed frontend first-run screen
that presents those actions is separately paired source and is required
before claiming a complete graphical journey.

## Distribution boundary

| System | Package route | Evidence here | Product support claim |
| --- | --- | --- | --- |
| SteamOS | Reviewed Arch package source and user-owned PKG0/PKG1 adoption | Source tests only; working Deck unchanged | Existing installed product evidence remains separate |
| Ubuntu 26.04.1 LTS, amd64 | Deterministic `.deb` from the same release manifest and payload | Disposable synthetic package install, dependency resolution and removal passed; actual paired Linux binaries linked and the manager entered its fixed status command | No adopted product, graphical, Bitwig, plug-in, audio or GPU qualification |
| Debian 13.7, amd64 | Same `.deb` format and exact dependency declaration | Disposable synthetic package install, dependency resolution and removal passed; actual paired Linux binaries linked and the manager entered its fixed status command | No adopted product, audio or graphical qualification |
| CachyOS 260809 ISO, rolling amd64 | Existing Arch package format from the same roster | Disposable 40 GiB VM reached graphical KDE Plasma 6.7.5 login on Wayland, kernel 7.2.8-1-cachyos; no bridge package yet | Package and product not qualified |

The [CachyOS graphical receipt](../evidence/beta-portability/cachyos-graphical-2026-09-28.json)
establishes an installed desktop fixture only. The Ubuntu and Debian observations use container images, not graphical clean
machines. Package parsing, dependency resolution and file ownership must not
be reported as first sound. A clean graphical install and normal frontend
journey remain required on each declared beta platform.
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
  `RELEASE_MANIFEST.json`.
- The package-generated desktop file has a fixed `/usr/bin` frontend target.
  It does not call adoption or start a service.
- `tools/portable/deb.py` verifies that manifest and payload, builds a
  deterministic Debian archive, adds only necessary root-owned parent
  directories, and verifies its complete control and data rosters. It refuses
  maintainer scripts and changed dependencies. Its build, verify and signing
  paths use PKG0's shared destination, role, component, mode, generated-file
  and preparation-kit roster law; an extra command cannot be signed as a
  document. It does not add a second
  compatibility or software-state authority.
- `tools/portable/release.py` signs a Debian bundle only with an externally
  supplied key. A `release` signature requires the PKG1 kit, exact clean
  source tree, and source-rebuilt native backend. Verification requires a
  separately supplied trusted public key and expected fingerprint; the key
  copied into the bundle is informational. Internal-test signatures have no
  customer release authority. No product release key is selected here.
- Proton/SLR remains an externally installed, exactly verified prerequisite.
  No package in this owner redistributes it, a vendor plug-in, or a license.

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
