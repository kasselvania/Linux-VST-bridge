# Catalog reliability physical-test handoff

Approved source: `f164048f81045c76711e8ce1886a9dcd6f30dee7`, tree
`2855d972cfbc9c9939627a15b3fd1fae96296723`.

This is the frozen source for the next Deck test. The separate manager-clarity
branch must not be substituted into its first installation package.

## Offline package status

An off-device package directory was prepared at
`/private/tmp/catalog-reliability-f164-package`. It contains exact-source Linux
manager/frontend builds, the exact supervisor and ownership sources, the retained
Blackhole/Kontakt Windows host and source manifest, and the retained preparation
kit. `PACKAGE-INCOMPLETE.json` records every byte identity.

It is deliberately not installable yet. AP12 published no downloadable build
artifact, and the current installed `installer-launch.exe` exists only in the
Deck's private immutable Software record. Explicit setup packages do not retain
an omitted installer adapter, so an older off-device adapter must not be guessed
or substituted.

The prepared Linux artifacts are:

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `linux-vst-bridge` | 6,330,328 | `9c7de287970ab818f7df64113ea44cefd7cf69ac12c662e0d355faa11ffae439` |
| `linux-audio-compatibility-manager` | 9,985,088 | `60570fac5dc50eae142c6a875c3c64aee87af89a58cf0a1be7e61676f5c95777` |
| `session.py` | 281,417 | `307a07449d5fa62e1a80536cc3c13838854deda1974d95abf5660fe911878923` |
| `ownership.py` | 29,249 | `cc81d05876043ccf96c1540c556a790e62f57f4a6e72aabbd820a5d133311add` |

The package manifest also binds the retained host, source manifest and
preparation kit. Their private paths do not become part of the installation
authority; `setup PACKAGE` copies and verifies the supplied bytes into the new
immutable Software generation.

At the Deck, before stopping anything:

1. Read `~/.local/share/linux-vst-bridge/managed/software.json`.
2. Verify every referenced artifact, including the optional installer adapter.
3. Copy the exact current `installer_launch.path` into the prepared package as
   `installer-launch.exe` and record its SHA-256 and byte count.
4. Preserve a private rollback package containing every artifact referenced by
   the current Software record and retain the exact current manager path.
5. Remove `PACKAGE-INCOMPLETE.json` only after the complete package roster has
   been rehashed and reviewed.

Installation remains an operator-controlled Deck action. Close Bitwig and the
manager, confirm zero DSP/maintenance owners and no cleanup block, stop the user
service, invoke the candidate manager's existing `setup PACKAGE` operation, then
verify the new Software record, service, keepers and capacity before opening
Bitwig. Do not install the manager-clarity branch for this first test.

Rollback uses the preserved prior immutable manager executable with its complete
prior package through the same stopped-service `setup PACKAGE` path. Merely
repointing one command or restoring `software.json` is not a complete rollback.

## Existing observations for the physical test

Before the instance launch, retain private output from:

```text
linux-vst-bridge status
linux-vst-bridge capacity
linux-vst-bridge operator snapshot
linux-vst-bridge capture status
```

The approved backend can arm exact managed-experimental capture through the
existing `linux-vst-bridge capture arm SELECTION` command. Obtain `SELECTION`
from the fresh exact `status` output; do not hard-code a historical product
number. Confirm the arm with `capture status` before creating the fresh Bitwig
processing instance. The frozen graphical manager can still display the stale
ordinary-publication disabled reason; the separate clarity candidate repairs
that presentation.

After the single mouse/touch comparison, refresh the graphical manager and
retain fresh `capacity`, `operator snapshot`, and `capture status` output before
any retry. `active_sessions` distinguishes active state from current and retained
editor, Windows-host and transport failure. A completed detailed incident may be
exported with the existing sanitized report action; private incident data and raw
paths remain private.

`tools/uio3/session.py` is not a generic catalog observer: it binds the exact
ordinary Pigments class/profile and its private observer package. `tools/uir2`
uses a source-owned disposable touch fixture rather than the vendor editor.
Neither may be relabeled as a Blackhole/Kontakt observation. For the first test,
the existing catalog/IF1/capture facts answer whether the editor or Windows host
failed and whether cleanup completed. If those facts do not identify the first
touch boundary, stop with that exact gap instead of adding speculative input
instrumentation during the session.

The operator separately reports whether the editor remained visible and whether
audio continued. Moonlight pointer input is not physical Deck touchscreen input.
