# Kontakt 8.13.1: fix the MSI reader before replacing the installer

Investigation date: 2026-09-18, America/Los_Angeles.

## What failed

The working Native Access session downloaded and launched the vendor package.
Its VC++ prerequisite succeeded, but no final Kontakt executable or VST3 appeared.
The incomplete uninstall entry and success notification did not prove installation.
Read-only inspection during this investigation reconfirmed both final targets absent
from the managed NI prefix.

The initial MSI defect was real: InstallAware's rewritten database changed the
string-pool metadata from two-byte to three-byte string references while retaining
some narrow table streams. The pool grew from 57,162 to 85,744 entries. Stock Wine
rejected `_Tables`, `Directory`, and `File` with 1615.

Two upstream changes repaired the obvious extent mismatches:

- [24bbf46: table-stream padding](https://github.com/ValveSoftware/wine/commit/24bbf46c6f055077df428341595873fe23475a60)
- [e013097: mismatched string-reference widths](https://github.com/ValveSoftware/wine/commit/e0130972d5ffe578a4a25d75f4c1d0229c880a8c)

The previous probe only fetched the first row of five tables. It did not validate
the rows or the feature/component joins used by installation. The previous patched
wrapper timed out after 30 minutes without final deployment. The separate original
MSI experiment returned 1603 from custom action `ACTEZ` (`ACTEX!ActProc`); that is a
different path from the rewritten wrapper MSI and is not a repair recipe.

## Newly identified defect in the previous patched runner

The retained patched-wrapper trace reaches `MsiInstallProductA`, then
`CostInitialize`. It loads `Component` as **11,820 rows** and stops advancing while
loading the second feature's component relationships.

The MSI `Component` stream is **200,940 bytes**. Its schema contains five string
references and one two-byte integer:

| Interpretation | Row bytes | Rows | Cached references outside the string pool |
| --- | ---: | ---: | ---: |
| Correct two-byte references | 12 | 16,745 | 0 |
| Incorrect three-byte references | 17 | 11,820 | 58,976 |

Both divisions have zero remainder. Wine's upstream fallback is guarded by
`rawsize % row_size`; it never checks the alternative for this table. Of the
11,820 alleged wide primary keys, only 22 point to nonempty strings. The narrow
interpretation has 16,745 valid unique primary keys. This was independently
checked against the original and rewritten MSI streams, without exporting vendor
payload or table contents into the repository.

Consequently, the earlier statement that the table fixes made the MSI readable
enough to execute was overbroad. Successful API status concealed a misdecoded
component table. The retained evidence establishes this specific corruption;
the full installer result must establish whether correcting it is sufficient.

## Narrow correction

`kontakt-msi-ambiguous-strrefs.patch` applies after the two upstream commits to
Wine `dc26e61847081a1b5cb0733dc30feba6ee575482`, the Wine revision pinned by
Proton `proton-11.0-2c` (`5b89db940e0ebe3a137a6009a3589232fe084c09`).

Before accepting the advertised table width, inspect its string references. If
the advertised interpretation contains invalid references, also consider the
narrow interpretation, even when the extent is divisible by both row sizes.
Select narrow only when its extent and all string references are valid. Binary
stream-reference columns are excluded from string validation. Valid wide tables
keep their original interpretation.

This is a candidate compatibility correction inside Wine's MSI implementation.
It leaves file placement, custom actions, registration, and installer completion
with the vendor installer. It does not intercept `MsiInstallProduct` or manually
manufacture NI product records. Broader upstream review and malformed-table
coverage remain necessary before treating it as a general Wine fix.

The patch modifies LGPL-2.1-or-later Wine code. Preserve Wine's corresponding
source and notices with any later binary distribution; no Wine or vendor binary
is included here.

## Actual corrected-run result

The ordinary unmodified vendor setup ran with `/s` in a fresh disposable prefix.
The reader reported that `Component` uses two-byte references, then progressed
through the feature relationships, `File`, and `Media`. It exited after **232.22
seconds**, without hitting the 300-second limit.

Observed after all disposable processes exited:

- `Kontakt 8.exe`: 178,397,680 bytes, SHA-256
  `965b47d7d15125c862c541a0f1c30bb1e8fab222e6815a5d0acdf77a42dcf60c`.
- `Kontakt 8.vst3`: 178,699,760 bytes, SHA-256
  `b4a3d79b194adfc307e5741059c05bdbd7d8e3cbfed4ca7a8e8e8cf755d06563`.
- Both match the corresponding retained vendor FileBag files exactly.
- Six application files and 8,138 files under the NI common-support root.
- Vendor-written uninstall registration has display name, version `8.13.1.0`,
  installation directory, and uninstall command. The NI product key has its
  application and content paths.
- No `err:msi` records in the selected log; no disposable processes remained.
- Managed NI `system.reg`, `user.reg`, and `userdef.reg` hashes remained unchanged.

A second fresh run added InstallAware's documented native logging switch and
finished in **232.65 seconds**, again returning Linux status **100**. Both primary
binaries have identical hashes to the first run. The native MSI log contains the
Kontakt `INSTALLSTART` record followed by `INSTALLEND` with field 3 equal to **1**.
In the pinned Wine `dlls/msi/action.c`, this field is `!rc`, where `rc` is the
result of `ACTION_ProcessExecSequence`: the **MSI execute sequence succeeded**.
The logged run also left no disposable processes and did not change the three
managed NI registry files from their pre-investigation hashes.

This establishes actual vendor deployment, successful MSI sequence execution,
and closure of the earlier stuck installation phase. It does not establish full
wrapper success: the native variable dump contains `COPYERROR=TRUE`, without a
failed source/destination or a demonstrated connection to outer status 100.
The two small vendor installation logs contain installed paths and registration,
not a definition of that exit code. Do not whitelist 100 or call this residual
harmless. Its exact vendor meaning remains a gap. Native Access recognition,
standalone launch, VST3 enumeration, authorization, audio, and managed-environment
migration were not exercised.

The exact tested candidate has the new correction in its **32-bit MSI module**,
SHA-256 `d9d9b3c0ec6361b7c89c6d9f68854e5c5af84e643104594f7af5a3e49ba59b8a`.
The 64-bit module remains the previous two-backport build, SHA-256
`96551956a18a4d14660ae9fda8b63acf6085281657f5c95dbd079db37ece161b`.
Both new modules compiled; the attempted 64-bit candidate-copy destination was
incorrect, so that copy failed. No running candidate was changed. The result is
explicitly evidence for the actual 32-bit-only replacement, not for a claimed
two-module deployment. Kontakt's installer process is 32-bit.

Aggregate receipts are in `evidence/kontakt-msi-root-cause/`. Raw vendor logs,
staged payload, proprietary MSI files, and disposable prefixes remain private.

Private reproduction assets on the Deck are under
`<HOME>/.cache/linux-vst-bridge/runner-builds/proton-11.0-2c-ni-msi/`:
`source/`, `candidate-strrefs/`, and `full-installer-strrefs-1/`.
The second logged run is `full-installer-strrefs-logged/`.
These are investigation assets, not an installed managed runner release.

To reproduce the source build, apply the two upstream commits in the listed
order, then this repository's patch, to the pinned Wine submodule. From the
already configured Proton source, use its existing build tool:

```sh
make enable_ccache=0 build_name=lvb-proton-11.0-2c-ni-msi module=msi module
```

The tested build used Valve SDK image
`registry.gitlab.steamos.cloud/proton/steamrt4/sdk/x86_64:4.0.20260331.220802-0`.
Copy only the resulting `build/msi/lib/wine/i386-windows/msi.dll` into the
separate runner candidate's `files/lib/wine/i386-windows/msi.dll` before launch.
Keep the Steam-installed runner and prior failing candidate intact.

Create a fresh disposable `STEAM_COMPAT_DATA_PATH`, carry the active graphical
session's display/Xauthority bindings, initialize via the exact runtime entry
point plus `proton getcompatpath /`, then invoke the same runtime and
`proton runinprefix` with the original setup's Windows `Z:` path and `/s`.
The follow-up adds the vendor-documented
`/l=C:\kontakt-install.private.log` to retain native outcome evidence.
Use an independent timeout and inspect the final prefix after process exit.
Neither `proton run` nor a bare Wine launch is an equivalent reproduction.

## Proton tooling and ownership

Proton has the appropriate module-build mechanism: its `module=msi module` target
builds the MSI implementation under the pinned Valve SDK. That existing mechanism
was reused for this correction. A compatibility-runtime defect belongs in the
runner; the manager selects the exact resulting runner and supervises the real
installer.

[Protontricks](https://github.com/Matoking/protontricks) wraps Winetricks for Proton
prefixes. [UMU](https://github.com/Open-Wine-Components/umu-launcher) provides the
non-Steam Proton launch route.
[Protonfixes](https://github.com/Open-Wine-Components/umu-protonfixes) applies
application-specific fixes and dependency verbs. These are useful integration
tools, but no verified existing Kontakt verb fixing this ambiguous table case
was identified. Installing VC++ again would not repair these MSI row widths.

## Lesson and next installation route

**Subsequent managed installation completed:** the existing NI prefix was moved
to the corrected pinned runner without changing its Windows MachineGuid. Native
Access 3.26.0 installed Kontakt 8 Player 8.13.1 and Factory Selection 1.4.2 through
its normal Install buttons, and a fresh launch recognized both as installed.
VST3 scanning and SDK inspection succeeded. Native proxy preparation then exposed
the separate 32-output-bus limitation in the bridge; Bitwig audio is not yet
tested. The exact continuation, rollback and results are in
[`evidence/kontakt-native-access-deck/README.md`](../evidence/kontakt-native-access-deck/README.md).
The direct-installer observations and unexplained wrapper status above remain
unchanged; they are not the basis of the later Native Access success claim.

1. Repair the demonstrated MSI decoding defect in the pinned runner and test the
   untouched vendor wrapper in a disposable prefix.
2. Require final payload, vendor registration, and installer completion together.
   Then check Native Access recognition after reopening, standalone launch, and
   VST3 discovery. Audio and DAW project recall remain later checks.
3. Only carry a successful exact runner correction into the managed NI environment
   with a preserved rollback snapshot and stable machine identity. Keep its
   working Native Access session architecture.
4. Store the package/runner match and measured outcome as compatibility data.
   Do not encode an unverified replacement installer as the compatibility policy.

Concretely, the next product change is a pinned runner revision containing the
two upstream patches plus this MSI-reader correction. Bind it through the existing
runner/environment owner, preserve a rollback snapshot of the existing NI prefix,
and run the original vendor package through the working Native Access/Proton
session there. Resolve the recorded wrapper outcome and verify Native Access
recognition after reopening before declaring managed installation complete.
Do not copy files out of these disposable prefixes into the live prefix, recreate
the licensed machine, or write Native Access's product database by hand.

K8I1 added 4,637 lines across 39 files relative to PR #127 without producing a
working installation. Those source tests exercised a proposed deployment engine;
they could not prove the vendor package installed. Preserve that draft as a failed
direction, not as the default continuation. The next evidence must concern the
real installer and the exact runner correction.

The repeatable rule is: API success is not semantic correctness; inspect the
complete relationships at the failing boundary. A proof that a table opens must
not become a claim that its contents are correct, and a timeout must not become
an assumed universal incompatibility that justifies replacing Windows Installer.
