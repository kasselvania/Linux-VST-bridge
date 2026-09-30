# Application-owned runtime and prebuilt proxies

This is the implementation selected in tasks 1 and 2 of
[the beta to-do list](SELF_SERVICE_BETA_TODO.md). The clean Ubuntu commercial
journey is the test of this delivery, not an assumed consequence of packaging.

## Runtime setup

Setup offers **Install compatibility runtime (728 MB download)**. That explicit
action downloads two fixed upstream archives over HTTPS, verifies their exact
sizes and SHA-256 values, extracts them into private staging, and selects one
application-owned runtime. Steam, system Wine and customer-installed Proton
are not used to acquire or locate it. Curl is a declared package dependency.

| Component | Upstream archive | SHA-256 |
| --- | --- | --- |
| GE-Proton11-7, x86-64 | [GE release](https://github.com/GloriousEggroll/proton-ge-custom/releases/tag/GE-Proton11-7) | `c5448b76a230384e2d7bc6beb5ccb97bafb7e2c3b6c527cb03a1a546bbcb00a0` |
| Steam Linux Runtime 4, 4.0.20260805.254769 | [Valve archive](https://repo.steampowered.com/steamrt4/images/4.0.20260805.254769/SteamLinuxRuntime_4.tar.xz) | `3226d8234e7c0542ee767837832bfb1dad5e5e2dc944ec97eb221b437f6b9349` |

The runner identity is `managed-ge-proton11-7-slr4-20260805`. Its complete
regular-file digests, symlink targets and directory roster are retained and
verified by the runtime owner. Acquisition preserves upstream license and
notice files. Archives with changed bytes, duplicate names, paths outside their
declared root, escaping links or unsupported entry types are refused.
No caller-selected URL, executable recipe or ambient latest version is accepted.
An incomplete installation cannot become a runner. Repeating successful setup
returns that exact runner. Existing environments keep their original runners.

Install and execution admission verify all runtime bytes. Read-only projections
reuse a completed byte observation only while every file's device, inode, size,
owner, mode, modification time and change time match. They still enumerate the
exact tree and verify symlinks and critical artifacts. The observation cache
lives outside the immutable runtime and cannot authorize execution. Missing or
invalid caches fall back to byte verification; a cold cache is prepared before
projection locks. This avoids repeated multi-gigabyte reads under manager locks.

The project downloads upstream artifacts to the user's machine; this change
does not publish a rehosted runtime or establish all obligations for a future
customer release. Customer signing, complete notices/SBOM and any later runtime
redistribution remain task 4. This selected GE/SLR pair needs its own observed
compatibility results; earlier Valve-Proton fixture profiles do not qualify it.

## Prebuilt publication

Preparation kit schema 3 carries prebuilt native ELF proxies, exact generated
SDK descriptors, a proxy index, paired Windows host/source manifest, source-owned
selection tools, backend/source identities and the pinned SDK's MIT notices.
`tools/mf3/prebuilt_kit.py` builds this kit on the developer's Linux build machine.
Package assembly checks its roster, rebuilds the backend and each proxy from the
declared clean source tree, and compares the resulting bytes before signing.

On the customer's machine, ordinary managed inspection selects an exact
module SHA-256 and Windows class ID. The kit-owned descriptor generator checks
the observed buses, parameters and vendor metadata
against the shipped descriptor. Presentation uses valid SDK-declared defaults.
If the vendor supplies an invalid default, the existing validated readback is
retained as an exact match requirement; a different value refuses this proxy.
The Arturia fixtures expose MIDI helpers with default -1 and observed value 0.
No guessed default or preset substitution is introduced.
Existing class IDs remain stable across builds. Preparation then copies the
verified ELF bytes into the managed candidate; it invokes no compiler, Flatpak
SDK, VST3 SDK or download. An unlisted module or different metadata is refused,
with no source-build fallback. A subsequent kit can add a specifically tested
build; an existing project must never silently receive another class/build.

Historical schema 2 kits remain readable for existing installations. Their
compiler prerequisites do not describe the new schema 3 installation route.
Prebuilt delivery does not imply every plug-in is supported: the exact shipped
index defines the available builds, and compatibility observations still define
what each build has actually demonstrated.

## Verification

Source checks cover changed downloads, duplicate extraction entries, changed
runtime files, unlisted files, proxy/module and metadata mismatches, non-ELF
payloads, and preparation without compiler processes. Retain the installed
package result and the FRAGMENTS trial journey under `evidence/` as they occur.
Keep install, authorization, sound, editor, persistence, reboot and recovery
results separate; leave the beta journey to-do open until all required results
have been observed.
## Package selection dependency

This branch integrates the package-owner, idle-transport and Setup/update
controls from draft PR #198, head `9402a89aecb4cba95791bf7c4b66316743fb1cb7`.
Those controls let an installed successor replace the selected application
through the normal frontend while preserving its exact predecessor. The
original CachyOS observations are not Ubuntu test results. This closes the
package-selection dependency for the delivery test; task 5 still includes
coordinated runner, plug-in and project recovery.
