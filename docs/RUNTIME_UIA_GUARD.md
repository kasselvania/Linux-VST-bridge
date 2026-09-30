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
