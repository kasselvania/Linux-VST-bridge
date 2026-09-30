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

The corrected runner identity is `managed-ge-proton11-7-slr4-20260805-r3`. Its complete
regular-file digests, symlink targets and directory roster are retained and
verified by the runtime owner. The exact zero-byte SLR lifetime lock is mode
0600 because pressure-vessel opens it read/write; its size and empty digest
remain verified. SLR platform files use canonical mode 0644/0755: pressure-vessel
hard-links those files into a mutable sysroot and normalizes their shared inode
modes. Every containing runtime directory remains private mode 0700; GE and
other support files remain 0400/0500. Full admission checks still verify the
exact bytes, owner, modes and roster. Earlier revisions are retained without
changing environments bound to them. This correction follows
the observed launch error, post-install mode drift and upstream [runtime lock](https://gitlab.steamos.cloud/steamrt/steam-runtime-tools/-/blob/main/pressure-vessel/runtime.c) and
[file-lock implementation](https://gitlab.steamos.cloud/steamrt/steam-runtime-tools/-/blob/main/steam-runtime-tools/file-lock.c).
Acquisition preserves upstream license and
notice files. Archives with changed bytes, duplicate names, paths outside their
declared root, escaping links or unsupported entry types are refused.
No caller-selected URL, executable recipe or ambient latest version is accepted.
An incomplete installation cannot become a runner. Repeating successful setup
returns that exact runner. Existing environments keep their original runners.

Install and execution admission verify all runtime bytes. Read-only projections
reuse a completed byte observation only while every file's device, inode, size,
owner, mode, modification time and change time match. They still enumerate the
exact tree and verify symlinks and critical artifacts. The observation cache
lives outside the runtime installation and cannot authorize execution. Missing or
invalid caches fall back to byte verification; a cold cache is prepared before
projection locks. This avoids repeated multi-gigabyte reads under manager locks.
Earlier native admission used an isolated full-byte verification scope: repeated checks may
reuse only hashes computed in that scope after exact reopened file identities
match. Persisted observation stamps cannot authorize launch, including nested
readback calls; the scope ends before DSP supervisor launch. Installed internal23
reached the Windows module and shared transport, but DAW activation failed at
subsequent state/setup boundaries. See the retained test; no audio/editor or
trial-usability claim follows from successful admission.

The integrated source successor shares byte preparation across native requests
within one manager process. Only successful verification of actual bytes can
populate that private cache; persisted observation stamps never seed it. Every
reuse reopens files without following links and checks device, inode, size, mode,
owner, modification/change times and the exact runtime roster. Changed identities
force hashing and the ordinary exact-digest comparison. A bounded preparation
lock is released before keeper, process and DSP work. A new manager prepares
again; background preparation does not make a plug-in processing-ready. Focused
tests reject changed bytes, poisoned disk observations, symlinks, failed
preparation and expired waits. This correction is source-tested and is absent
from installed internal32; its startup performance remains to be measured.

Internal31's ordinary cold FRAGMENTS live preview was accepted without Reload
in 44.610 seconds: binding verification 24.586 seconds, keeper ready at 44.202.
This exceeds the declared 30-second target. The editor subsequently opened;
complete vendor initialization and repeated/steady-audio acceptance were not
measured by that admission trace.

Explicit installer selection accepts an operator-owned regular file or a
root-owned regular file without group/other write permission. The frontend opens
read-only without following links; the importer checks its descriptor, file
format/size, available space and unchanged identity including owner/mode while
copying and hashing. Execution still uses the private, immutable imported copy,
not the source path. Internal32 refused its own system-packaged reference
installer under the older operator-only check; this source correction must be
retested through the delivered frontend.

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

The internal21 test kit retains the Pure LoFi fixture and original Deck
FRAGMENTS entry, and adds the exact module produced by official FRAGMENTS
1.0.0.2925 installer media (`edb358f1…`). Its
[allow-listed SDK census](../evidence/self-service-delivery/fragments-1.0.0.2925-official-sdk-metadata.json)
comes from normal installed discovery on internal20. A shared version label
does not substitute for the module digest. This census defines the native
presentation, not an audio or vendor-authorization qualification.

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
