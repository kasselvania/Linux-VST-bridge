# Wine UI Automation null-provider reference

The delivered Ubuntu FRAGMENTS trial reproduced access violation `0xc0000005`
at editor-removal stage 212, even after the Windows host's frame-detach correction.
An allow-listed header observation captured a null first argument and zero
access address. The same host process's mappings and Wine's own symbols place
the instruction in `uiautomationcore.dll`, RVA `0x7988`,
`create_uia_node_from_elprov + 0x18`. The vendor caller remains unproved.

The existing [AP11](AP11.md) and [AP12](AP12.md) records document this same
null-provider signature and the explicit per-process accessibility policy.
Managed preparation previously gave every new candidate Windows-default
accessibility. The new versioned preparation settings apply the existing
`DisabledForVendorProcess` capability only to the exact official FRAGMENTS
module/class and the measured application-owned r3 runtime. The entry scripts
and faulting DLL must match. Unknown builds retain the default. The proposed
profile stays `ReviewCandidate`, includes `WindowsAccessibilityUnavailable`,
and has a new identity. Its publication remains explicit. Prior publications,
prefixes, installers, environment owners and other products are unchanged.
This policy disables Windows UI Automation/screen-reader integration for that
vendor host. VST parameter automation is a separate interface.

Internal39's populated Deck update prepared new host/proxy pairs for Pure LoFi
1.0.0.6121 and FRAGMENTS 1.0.0.2925, but reverted their declared process policy
to Windows-default accessibility. The exact selected predecessors disable it.
The updated project restored both opaque states and produced captured output;
normal Pure LoFi editor close then failed at removal stage 212 with access
violation `c0000005`. The current fault header identifies that boundary, without
a new instruction-to-DLL mapping or vendor-caller attribution. Its processing
phase also retains missing frames, so the run is failed for musical acceptance.

Preparation now includes the existing bounded policy for those two exact Deck
module/class pairs and pinned runner, entry scripts and UI Automation DLL.
The [AP12 causal comparison](../evidence/AP12/delivery-cause-and-repair.json) and
[musical pair](../evidence/AP12/musical-pair-reboot-removal.json) are its retained
basis; live readback confirmed the same bytes. Unknown module or runtime builds
remain Windows-default. Proposed profiles remain review candidates with Windows
screen-reader integration unavailable. This source correction is absent from
internal39 and needs delivered close/reopen and retirement testing.

Internal29 prepared and explicitly replaced the publication with this policy,
but a separate retained-buffering verifier refused DAW admission before a
Windows DSP host existed. Internal30 delivered that bounded admission repair.
The retained official trial then completed three normal editor close/reopen
cycles on one instance and normal instance retirement. A fresh demo instance
completed another close/reopen cycle, captured altered stereo audio afterwards,
and retired normally. Both supervised terminal observations recorded editor
stage 217, failure/exception zero and confirmed host/transport cleanup. This is
a bounded installed commercial result for the exact declared policy; it does
not correct or redistribute the faulting DLL, qualify screen-reader use, or
establish vendor-family support, recall or dropout-free audio.

A separate runtime correction checks the null provider before dereferencing it
and returns `E_INVALIDARG`. Existing public null-output handling is retained.
It changes no vendor binary,
authorization logic, valid-provider handling or bridge exception containment.
The source is the Wine submodule at GE-Proton11-7's exact commit
`46b29104e3741fe23bf5e2547196a253aab88c89`; the SDK image is pinned by digest.
The [isolated builder](../tools/runtime-uia-guard/build.py),
[Wine patch](../tools/runtime-uia-guard/uia-null-provider.patch) and
[LGPL text](../tools/runtime-uia-guard/COPYING.LIB) retain the construction basis.
New Wine test code is LGPL-2.1-or-later. Wine code is not linked into the
proprietary manager or native proxy. No runtime binary is committed. The exact
correction and complete corresponding source are delivered separately from the
signed application; full GE/SLR distributions are acquired directly upstream.

On the Ubuntu fixture, the exact null-provider test crashed with the delivered
DLL (exit 5, access violation at the same Wine RVA). A separately sealed
candidate completed the same test with exit 0. Its fresh prefix's DLL digest
matched the built correction. This establishes the bounded invalid-argument
regression result, not FRAGMENTS editor retirement or customer delivery of that
runtime. The installed commercial journey and its failures remain in the
[Ubuntu receipt](../evidence/self-service-delivery/ubuntu-fragments-trial-2026-09-29.json).

The saved builder was exercised independently. Its binary digests differ from
the initial developer build; byte reproducibility is not claimed. The rebuilt
DLL/test were then run in another fresh reference prefix and completed with
exit 0, with the prefix DLL matching that rebuilt artifact. Both constructions
remain outside managed runtime selection and commercial environments.

The current construction narrows the patch to the missing provider check.
Matched original/corrected tests separately exercise `UiaDisconnectProvider(NULL)`
and `UiaReturnRawElementProvider(..., UiaRootObjectId, NULL)`, the entry points that
reach the faulty helper. The original DLL faults at `uiautomationcore + 0x7988`;
the corrected fresh prefix completes all 16 checks without failures or skips.
The original also passes the independent valid-provider and public null-output
checks. The valid path creates a node, reads its provider Name property, releases
the node and restores the provider reference count. Fresh-prefix DLL hashes
match the respective original or corrected image. These are x64 API checks;
the upstream x86 DLL is unchanged, and commercial editor acceptance is pending.

The [component packager](../tools/runtime-uia-guard/package.py) verifies the exact
GE archive and original x64 DLL, then seals the correction, matched test,
complete patched Wine source, recipe, patch, license and notices in `uia-guard`.
Compilation records and rechecks the consumed changed-source and recipe bytes;
packaging refuses later source or recipe edits. Its focused tests run with
`python3 -m unittest discover -s tools/runtime-uia-guard -p 'test_*.py'`.
The source archive includes the actual `build.sh` and SDK pin; direct rebuilding
requires the owned `/work` volume and `XDG_CACHE_HOME=/work/cache`, with two CPUs,
4 GiB memory, no extra swap, 256 PIDs and `make -j2`.

GE source `c191f35dcebbeccfacd3b4c6f6eea026e588c1c2` and Wine-staging
`6cc805ea57132eeaf44764e9213823c9b8d0d300` were checked against the generated
UIA dependency closure, including generated interfaces, UUID and CRT/import
libraries. Intersecting shared-header/import additions are unused by this
component; no effective UIA implementation changes are omitted. The included
isolated compiler recipe differs from GE release flags, so this is a rebuilt
component rather than byte-identical reproduction of that release. Its imports
resolve within the pinned GE x64 builtin DLL tree.

Delivery composes this component with unchanged pinned upstream GE/SLR only in
new private installation staging, checks the original DLL preimage, then seals
the whole immutable runner. The component manifest binds corresponding source
and notices alongside the binary. The reviewed
[r4 component release](https://github.com/kasselvania/Linux-VST-bridge/releases/tag/runtime-uia-r4)
delivers [uia-guard.tar.gz](https://github.com/kasselvania/Linux-VST-bridge/releases/download/runtime-uia-r4/uia-guard.tar.gz),
57,040,019 bytes with SHA-256
`988fb967608a8c116022c27822647b7a7eda50a283a9e53ee2f22fe299b60f6c`.
The corrected x64 DLL is
`9f7217f558986ae295c6abe931312e73c96d9007b4b1e93cf9dc6aaa023ff0ad`.
The managed r4 acquisition recommendation applies only to new environments;
retained r3 records, explicit choices and stored retries keep their exact runner
identity. No installed runner or licensed prefix is modified. Fresh commercial
editor and musical acceptance with Windows-default accessibility remains pending.
