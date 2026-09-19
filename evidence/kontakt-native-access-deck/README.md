# Kontakt installation through Native Access on Steam Deck

Continuation of the MSI investigation in `docs/KONTAKT_MSI_FINDINGS.md`.

## Installed change

Source `dc36a3b` keeps vendor installation identity distinct from the current
runner revision. An explicit Rust maintenance transition updates the existing NI
environment and its current application/onboarding bindings, preserving the
original installation results and a private metadata rollback copy. Dependency
admission still verifies the exact retained origin and current vendor bytes;
only a later runner/environment revision can differ. The existing supervisor
uses the same rule. No replacement installer is introduced.

The separate installed runner is Proton 11.0-2c with the earlier upstream MSI
backports and the ambiguous-width correction in the 32-bit MSI module. Both MSI
module hashes are pinned in the runner record. Steam's runner is unchanged.

A private copy of the 3.5 GiB NI environment was retained before mutation. The
manager package was installed using its existing setup operation. With the
bridge service stopped, `native-access-runner RUNNER.json` selected revision 2.
Proton's `getcompatpath /` initialization then refreshed the existing built-in
DLL links; `runinprefix` alone skips that step. Both prefix MSI links now point
at the selected runner. The prefix inode and Windows MachineGuid were preserved.
Private rollback assets are under
`<HOME>/.cache/linux-vst-bridge/kontakt-native-access-20260919/` and the transition's
reported managed `private-rollback` directory. No vendor state is exported here.

## Actual Native Access operation

Native Access 3.26.0 was opened through the ordinary manager action with software
rendering. A normal Retry recovered its initial product-loading failure, and the
existing account library appeared. In the vendor UI, Kontakt 8 Player 8.13.1 was
selected using **Install**. The smaller Kontakt Factory Selection 1.4.2 was also
installed for the requested sound test. No product database was edited by hand.

Native Access changed Kontakt's button to **Open**, and Factory Selection to
**Installed**. Both Kontakt binaries in the managed NI prefix match the earlier
disposable correction runs exactly; see `installed-payload.sanitized.json`.
Normal window close completed operation `3c939e8f0f1a9f91ba0f8779b289a4f3` with no
session error, confirmed cleanup and graceful service retirement.

The wrapper's historical status 100 has not been globally whitelisted. The new
installation claim comes from the actual Native Access action, vendor UI status,
final payload and supervised session outcome.

## Verification

The Linux manager release built with Rust 1.95.0 and Zig. Eight Rust dependency
admission tests passed on the Deck; the 10 Native Access session and 18 dependency
recovery tests passed locally. They check the changed admission boundary only;
the vendor installation above is separate physical evidence.

Bitwig fixture: Flatpak 6.1, app commit
`8a048e733e74dda8b897339436153a2d5df952f29d362dfcaca9d8e0d6f6c231`,
runtime `org.freedesktop.Platform/x86_64/25.08`.

## Reopen, scan and the actual Bitwig blocker

A fresh Native Access session (`b77359d3a4fec6222143883154680338`) again
required the ordinary product-list **Retry**. Its **Installed** tab showed Kontakt
8 Player 8.13.1 with **Open** and Kontakt Factory Selection 1.4.2 as **Installed**.
Normal close completed with confirmed cleanup. This establishes recognition
across an application restart, not a machine reboot or audio authorization.

The ordinary **Scan installed plug-ins** action completed. Scan
`8f7b10fa384c698e97021fea901bd4ed` found the real Kontakt module, matching the
installed payload digest, and instrument class
`5653544E694B386B6F6E74616B742038`. There was no inspection error or quarantine.

The ordinary **Check compatibility** action also completed. It initialized the
component and its combined controller, enumerated 4,145 parameters, confirmed
float32 processing support and captured initial state. An editor interface was
created but not attached. These are SDK inspection results, not played audio or
project recall.

The next ordinary **Prepare a test candidate** action was attempted exactly once.
Operation `224a05555eb0da12d810720227e2fd4a` refused with
`Operator action: bus count/index bound`. Kontakt declares **32 stereo audio
output buses**, all main buses, plus one 16-channel event input and output.
The current generator allows at most eight buses in each group and only one
audio output. This is not merely a generator check: the Windows `BusLayout`,
native processor and audio transport also implement one stereo output.

No Kontakt proxy was published, no Bitwig project was modified, and no Bitwig
audio/editor/state test was run. The next engineering change must implement
and validate the real output-bus contract across the native proxy and Windows
host. Removing a check, discarding declared buses, or silently routing them all
to one pair is not a fix. See `scan-inspection.sanitized.json` for the exact
observed layout and refusal. The service is active after cleanup, with zero DSP
and maintenance leases, no pending transactions and no unconfirmed cleanup.

## Using the installed result

Open Native Access from the compatibility manager using its software-rendering
option. If its initial product list fails, press **Retry**. In **Library →
Installed**, Kontakt 8 Player now offers **Open**; Factory Selection is installed
for the eventual sound test. The vendor download/install flow remains the owner
of installation. Do not reinstall through the disabled K8I1 replacement path.

For a new environment or runner repair, preserve the prefix first, bind the
exact corrected runner, and perform Proton's normal prefix initialization before
opening Native Access. A `runinprefix` launch alone does not refresh old built-in
DLL links. Verify the actual linked MSI module hashes, then use Native Access's
**Install** action and check both final payload and recognition after reopening.
