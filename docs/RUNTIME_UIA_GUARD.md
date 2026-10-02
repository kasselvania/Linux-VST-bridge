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

A separate runtime correction checks null provider/output arguments before
dereferencing them and returns `E_INVALIDARG`. It changes no vendor binary,
authorization logic, valid-provider handling or bridge exception containment.
The source is the Wine submodule at GE-Proton11-7's exact commit
`46b29104e3741fe23bf5e2547196a253aab88c89`; the SDK image is pinned by digest.
The [isolated builder](../tools/runtime-uia-guard/build.py),
[Wine patch](../tools/runtime-uia-guard/uia-null-provider.patch) and
[LGPL text](../tools/runtime-uia-guard/COPYING.LIB) retain the construction basis.
New Wine test code is LGPL-2.1-or-later. Wine code is not linked into the
proprietary manager or native proxy. No runtime binary is committed or rehosted.
An eventual distributed correction needs its own immutable runtime identity,
complete corresponding source and notices, delivery and commercial retest.

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
