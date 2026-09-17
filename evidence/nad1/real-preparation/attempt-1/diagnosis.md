# Corrected diagnosis of the first real preparation

This is a read-only reinterpretation of operation `6345df5fd7ea38a5f07bfc6a2300e999`, not another installer attempt. The original private operation result, stage logs, ledger and immediate readback remain unchanged. The initial README/receipt describe the supervisor's misleading projection; this document corrects that interpretation.

## Established facts

The retained adapter stdout contains one exact installer-root frame and one matching installer-result frame. Both match the private operation/token; the root matches the admitted 35,769,456-byte installer SHA-256. The actual Windows child returned **100**. In the reviewed adapter source, 100 comes from the retained child process handle's exit code; it is not the adapter's wait-timeout sentinel (144). The source-owned adapter flushed that result into the captured log before the supervisor closed the capture at its deadline.

The Linux ledger separately observed the installer runner exit 100 about 203.687 seconds after its launch, after SCM retirement had begun. This does not equate Linux and Windows identities, or prove why the runner lingered. The Windows result has no retained per-record timestamp. The original claim that no installer completion record existed was wrong; the record existed but had not been interpreted.

The installed daemon (18,259,440 bytes) hashes exactly to the corresponding selected member of the exact bundled installer. A streaming comparison read only that member and wrote no extracted payload. The offline registry contains the fixed service under a numbered control set, with own-process type. The immediate readback checked only CurrentControlSet and incorrectly reported absence. Offline registration presence does not establish active SCM state or readiness.

A retained prerequisite log reports apply/exit zero. The daemon log contains service-mode, server-started and startup-ended messages, followed by stop messages; it also contains default-location/configuration warnings. These shared vendor logs are useful leads, not exact Windows-generation attribution or readiness proof. Authentication-related application behavior is not tested or diagnosed from incidental log messages.

## Two repaired project defects

1. `Nad1Owner.command()` waited for the runner before interpreting installer frames. A late runner therefore displaced an already-retained child result with `dependency_command_timeout`. The repaired owner validates and durably retains root/result frames as complete stdout records arrive, preserving the Windows child result separately from Linux runner retirement. Nonzero 100 reports `dependency_installer_nonzero`; child zero with runner timeout reports `dependency_runner_retirement_timeout`. Frontend output shows the installer code and runner state separately from cleanup. No exit value is relabeled as installation or readiness success. The legacy adapter's ambiguous 144 remains explicitly unavailable.
2. The offline registry reader recognized only CurrentControlSet. It now also recognizes exact numbered control sets, refuses conflicting multiple registrations, and makes no active-control-set or runtime-state claim.

## Remaining vendor diagnostic gap and concrete next experiment

The meaning of installer exit 100 remains unestablished. The captured stdout/stderr contains wrapper messages and adapter frames, not the installer's internal action log. The exact package carries InstallAware framework metadata. [InstallAware's own documentation](https://www.installaware.com/mhtml5/desktop/setupcommandlineparameters.htm) distinguishes `/s` (silent) from `/l=<logfile>` (internal installation logging); the executed contract included only `/s`. Its documented log includes internal variables and Windows Installer records, so it must remain private and bounded.

The next diagnostic, if authorized, should collect that native installation log to an exact operation-owned private location, retain the immediate child exit, identify the last completed action and first failed action, and correlate subsequent exact SCM state. It needs a declared size/time limit and retention before cleanup. It must also explicitly handle the now-existing exact daemon and registration; the ordinary preparation currently refuses an unadmitted existing generation. Do not simply click Prepare again, raise the timeout, run the daemon directly, or treat exit 100 as an allowed success code.

No real replay, software installation, Native Access launch, service transition or registry mutation occurred during this investigation. Rendering remains selected and unchanged. This repair fixes reporting and offline presence detection; it does not claim to repair the vendor's installer or qualify readiness.
