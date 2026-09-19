# Blackhole Immersive first discovery failure

2026-09-19; Steam Deck; Blackhole Immersive 1.4.4 (Windows VST3), bundled
PACE License Support Win64 5.10.2.4663. Installed manager/frontend source:
`ef9f36a29d6af70538bf3a2db6be6016faf530a3`; investigation base `c9758f5`.
Pinned runtime: Proton 11.0-2c build 25118279 with Steam Linux Runtime 4
4.0.20260805.254769, as recorded by this existing isolated installation.
No runner, prefix identity, licensing state, plug-in binary or publication changed.

## Observed stages

| Stage | Retained observation | Limit |
| --- | --- | --- |
| Installer | Outer exit 0; durable files/registration present; cleanup confirmed | Transaction explicitly says `installation_completeness: unproved` |
| Installed payload | Blackhole VST3, iLok License Manager and PACE service executable exist; service registered | Existence does not establish licensing readiness |
| User-triggered scan | `TimeoutError: Windows call deadline: load_library`; no factory record or classes | This is a loading timeout, not an attributed plug-in exception |
| Failure containment | Scanner process cleanup and transport retirement confirmed | Does not prove successful product startup |
| Library projection before repair | Generic `inventory_factory_absent_or_duplicate`; original error omitted from quarantined card | Reporting defect hides the earlier failing stage |
| Ordinary rescan | Unchanged quarantined module reuses its retained result | No fresh execution; user needs explicit retry after a relevant change |
| PACE startup observation | Named service registers its handler and reports state 4 (RUNNING), start result 0 | Service state alone is not proof of successful license operations |
| Instrumented module inspection | Same load timeout; 36.649 s total including startup/cleanup; cleanup and transport confirmed | No root cause or vendor authorization established |
| iLok application startup | Exact installed executable opened to Sign In; Luna observed no startup/error dialog | Does not prove authorization or successful Blackhole loading |

The actual VST3 module SHA-256 is
`b8a33c57b04eded38d0ca717bb5e3721330439f4931c401ea70e5246b516e90b`.
Installed iLok License Manager executable SHA-256 is
`61dca5ffe948c925b8e52b495a3211f3e27356094b69ee848773722677884834`.

## Bounded diagnosis

The original scan and installer records remain private on the Deck. The
investigation used the installed supervisor's environment construction and
module-inspection function with the same module, scanner, runner and HOME.
An outer dedicated systemd unit bounded each diagnostic and its children.
The idle bridge was paused and restored around the two diagnostic launches;
final readback confirmed active service, zero DSP/maintenance activity, no
pending transaction/stale transport and no unconfirmed cleanup.

1. A service probe attempted `sc.exe queryex PaceLicenseDServices`. This runner's
   utility did not return a query result (outer exit 103), so that exit is not
   used as PACE status. Its separately retained startup trace binds the named
   PACE service to `SetServiceStatus` state 4 and service start result 0.
2. One instrumented production-supervisor module inspection retained bounded
   loader/service/exception diagnostics. The existing 30-second Windows-call
   deadline was unchanged. It again stopped at `load_library`. PACE's named
   service reported RUNNING in this run too. Diagnostic observation totaled
   1,436,708 bytes; first 128 KiB and last 256 KiB were retained separately.
   The middle was not retained, which limits attribution. Observed handled
   exception traffic does not establish a fatal PACE or plug-in crash.

No GUI input, account inspection, activation, new installer, prefix recreation,
DAW session or timing campaign occurred. Private traces can contain process and
path details and are not part of this public evidence. The remaining live gate
is the installed iLok application's startup and user-owned authorization flow
in this same environment, followed by one deliberate fresh module inspection.

## Manager repair

Quarantined cards now retain and display the scanner's `inspection_error`
separately from the generic factory-parser quarantine. Their explicit
`Retry this exact module scan` action binds the environment, current scan,
module index, module digest and original report digest. The existing inactive
admission, pause/resume and supervised inspection path execute the retry.
Other module records are carried forward unchanged. Ordinary rescan still
reuses unchanged quarantines; this action makes a fresh attempt deliberate.

The retry resolves both registered and onboarding-only environments, without
registering a failed product. Changed inventory, report, environment, scanner
bytes or scanner source are refused. Identical verified scanner bytes may move
to another immutable software-generation directory during a manager update;
a filesystem location is not the scanner's identity. Original reports and scan
history remain available. No new runtime, dependency or licensing workaround
is introduced.

Focused regression checks passed for cause presentation, exact retry selection,
ordinary quarantine reuse, onboarding-only routing, inactive admission, closed
action payloads and scanner relocation. Both crates passed all-target Clippy
with warnings denied; the Linux release build passed. Sandbox restrictions on
local Unix sockets and the cross-linker required rerunning those affected checks
with the permitted local access. No live retry was used as a reporting test.

Built code diff SHA-256 (three code files, base `c9758f5`):
`2e832a836388605c0c168f15059cb322d671645405b0472952f9d0c3c6d4c09f`.
Manager SHA-256:
`31e8c569cf783af84f5f55315e8a42088e52af4657bd1235025d93ea8364c7d0`.
Frontend SHA-256:
`026b6f5072e6b6a732321acc55e6468011e414c35a18e59bb3db454ec9b8abb6`.

Source commit: `fea71b4cc005b9fa6b3247af6daa3585138a9b7c`. After the operator
closed the manager, both binaries were installed through existing setup.
Readback verified executable hashes, command targets, running service identity,
the retained scanner error and the enabled exact-module retry bound to the
original scan/module/report. Six runtime artifact digests, 43 protected
registry/environment/onboarding/inventory/report files and nine native proxy
files were unchanged. The service was active with zero DSP/maintenance activity,
no pending transactions, no stale transports and confirmed cleanup. Private
manifest/launcher backups and the old immutable software remain available for
rollback. See [deployment receipt](deployment.sanitized.json).

Two staging attempts stopped on local file permissions before setup could run;
the old installation remained intact. Executable permissions were restored on
the verified new binaries and already copied immutable runtime files were
verified and reused. The next attempt completed. No live retry or GUI input
occurred; projection readback does not prove the new button was used or resolve
the underlying loading timeout.

## Authorized Moonlight presentation check

The operator explicitly authorized Luna 5.6 at Max for computer control. Luna
used the existing Moonlight stream, opened the manager through the normal
launcher, waited for installed-state readback, searched `Blackhole` and expanded
the card details. The parent performed no GUI input.

Luna reported these visible observations: `Blackhole Immersive.vst3`,
`Unresolved factory`, `Scan needs attention`, version unavailable,
`TimeoutError: Windows call deadline: load_library`, and
`inventory factory absent or duplicate`. `Retry this exact module scan` was
visible with active-button styling. Its backend availability was independently
verified during deployment. The button was not pressed; the manager was left
open with this card and its details visible. No iLok window or audio behavior
was inspected during this bounded presentation check.

Independent post-check process readback verified the running frontend uses the
deployed immutable executable. The original scan ID and report bytes were
unchanged. Bridge service remained active with zero DSP/maintenance activity,
no pending transactions/stale transports and no unconfirmed cleanup. This
establishes visible presentation of the retained failure and retry control,
not successful retry execution or resolution of the loading failure.

## iLok startup and user handoff

On the operator's continuation request, the exact installed iLok executable was
launched through the installed, verified launch adapter under the original
Blackhole environment, pinned runner and managed HOME. The idle bridge was
paused for the dedicated companion cgroup. The existing supervisor utilities
verified runner/image artifacts, locked the environment, bound the launch and
tracked the actual image mapping. Vendor output was discarded; no account or
license material was inspected.

Luna 5.6 Max observed the iLok License Manager window at its initial Sign In
screen, with no startup/error dialog. This establishes application startup only.
The first diagnostic had a five-minute runtime limit. The running systemd unit
rejected a request to change that property, so the limit was not silently
treated as removed. Luna confirmed the initial screen was still untouched;
the diagnostic was retired with confirmed cleanup and zero remaining owned
processes. A first handoff continuation stopped during the bridge's brief
startup-readiness interval; refreshed readback confirmed healthy idle state.

iLok was then reopened in the same environment as a supervised user session
with no runtime cutoff. Luna confirmed the same initial screen. The launch
binding, live image and unbounded normal application lifetime were read back;
memory/task limits remain. Closing the application triggers owned cleanup and
the configured bridge restart, which already completed after the diagnostic
retirement. The current user session remains open for the operator: its cleanup
and post-activation scan have not yet occurred. The operator was asked to sign
in, activate Blackhole if available, and close iLok. No automated sign-in,
activation, dependency update, prefix recreation or additional module scan ran.

The original scan remains unchanged. Next gate: the operator's authorization
result and application close, then one deliberate module retry. Successful iLok
startup does not explain the original timeout or establish iLok-wide support.
See [startup and handoff receipt](ilok-startup.sanitized.json).

## Private SSH credential handoff

The operator subsequently requested hidden SSH prompts so Luna can enter their
iLok credentials while the operator is away from the GUI. The session-specific
[helper](../../tools/ilok-login-handoff.py) was installed as `lvb-ilok-login`.
It uses private temporary runtime storage, a scheduled 15-minute expiration,
exact window/PID/start-time/cgroup binding and stdin-only keyboard delivery.
It never passes values through argv, environment, clipboard or command output.
The password record is removed before its single typing attempt; the helper
does not submit the login. [Operator instructions](../../docs/user/remote-ilok-sign-in.md).

Seven focused tests passed: hidden-input/non-TTY and echo-fallback refusal;
permission/symlink refusal; exact cgroup/target binding; normal sequence and
deletion before a failed password send; no replay; expiry-time recheck and
blocking expiry cleanup; replacement-nonce protection. The installed helper
matched SHA-256 `d6d8e64549c5c13451c0c61bd9ee5827f63655e6c8e0e500d4f52bb6eadc7460`.
Live `verify` returned `binding-valid`; runtime storage was verified as tmpfs,
directory mode 0700, with a scheduled expiration timer.

One real SSH prompt/GUI path check used explicitly fictional values. Neither
prompt echoed input. Luna selected each field and invoked each fill command
once: the User ID appeared as expected, and Password displayed masked bullets.
Luna cleared both fields and left User ID focused, Password empty and Remember
unchecked. Final helper status was `none`. No Sign In, activation or live module
scan was triggered. Actual operator credentials remain to be supplied; no real
credential, account identifier or credential file is included in this evidence.

## Vendor sign-in and activation

After the operator supplied the private SSH handoff and requested Luna to
continue, Luna invoked the two fixed field commands and submitted Sign In once.
The authenticated iLok view loaded without an error or additional-verification
prompt. Independent readback confirmed the temporary handoff file was absent.
Remember credentials remained unchecked; no credential value was printed or
included in the retained evidence.

The exact Blackhole Immersive license offered the current local computer as
its activation destination. Luna completed one vendor activation and observed
`Successful Activation`, then verified the current local activation and closed
iLok normally. No other license action, transfer, trial signup or purchase was
performed. The owned session reported zero remaining processes and confirmed
cleanup; its unit retired. Independent bridge readback was active and idle with
no pending transactions, stale transports or unconfirmed cleanup. A stale
manager status message during bridge startup did not persist in this readback.
See [activation receipt](ilok-activation.sanitized.json).

## Single post-activation scan

Luna submitted the exact-module retry once after manager refresh completed.
The backend recorded that action and a new inventory, preserving the original
failed report and module/scanner identities. The previous `load_library` timeout
did not recur in this run. Factory enumeration returned the audio-module,
controller and compatibility classes. Creating and initializing the component,
then querying its audio-processor interface, returned success.

Inspection subsequently reported `ap8_failure: controller query tuple`, followed
by SDK host exit 90. The rejection is at the component's `IEditController` query
in `windows-factory-probe/source/inspect_module.cpp`: this branch accepts either
a successful non-null combined controller or `kNoInterface` with a null pointer
before creating a separate controller. The actual returned code and pointer
presence were not recorded, so this evidence does not distinguish an overly
strict host check from an inconsistent plug-in response. Repeating the same
check cannot supply those missing values. No runtime change or second retry
was performed.

Luna observed the Eventide card as **Installed - not published**, **Effect ·
1.4.4**, with `Windows SDK Host failed: 90` and **Check compatibility**. No vendor
dialog appeared. Scanner cleanup and transport retirement were confirmed; the
bridge is active with three keepers, zero DSP/maintenance/pending transactions,
zero stale transports and no unconfirmed cleanup. The launcher's raw -15 exit
is retained separately from the host's structured code 90. The temporary
credential handoff remains absent. See the [scan receipt](post-activation-scan.sanitized.json).

This establishes vendor activation and progress beyond the original loader
failure on the exact fixture. It does not prove why that earlier timeout
occurred, complete compatibility inspection, publication, Bitwig operation or
audio. The remaining specific issue is controller-interface handling.

## Controller-interface response

A separately identified diagnostic host from source `5807e6b` recorded the
missing tuple in the existing activated environment. Blackhole returned
`kResultFalse` (1) with a null `IEditController` pointer. The old inspector only
allowed `kNoInterface`/null to reach `getControllerClassId`, so it rejected this
response before trying the separately declared controller. This pinpoints the
post-activation failure in the bridge's acquisition policy; it does not explain
the earlier pre-activation loader timeout.

The [FUnknown contract](https://steinbergmedia.github.io/vst3_doc/base/classSteinberg_1_1FUnknown.html)
documents `kNoInterface` for an absent interface. Blackhole's observed response
differs from that contract. However, the pinned official
[PlugProvider](https://github.com/steinbergmedia/vst3_public_sdk/blob/586dc5e6c8012c3e4b01c79389375cbe96bdb1da/source/vst/hosting/plugprovider.cpp#L147)
tries the component's declared separate-controller ID after a failed combined
query. The compatibility repair admits only the additional observed
`kResultFalse`/null tuple, preserving exact controller identity and refusing
inconsistent success/null, failure/non-null and unrelated error responses.

The diagnostic completed in 7.819 seconds with cleanup and transport retirement
confirmed. All 54 protected canonical/proxy files and the prefix identity were
preserved, with no installed-software change. The bridge returned active and
idle. See [diagnostic and build receipt](controller-diagnostic.sanitized.json).
The first diagnostic CI run was cancelled after review caught an incorrect
synthetic expectation for the Windows SDK's signed `E_NOINTERFACE`; the corrected
build passed before its artifact was used. No real plug-in run used that stale
test candidate.

## Controller repair and installation

Source `39c8da7` implements that narrow allowance and records the original query
tuple. Failed-query outputs remain separate from owned controller pointers, so
malformed non-null failures cannot be dereferenced or released during cleanup.
The existing Windows fixture checks normal no-interface/null, false/null,
success/null and error/non-null responses. The Windows build and tests passed.

The real candidate then completed inspection in 8.968 seconds: exact separate
controller `5653454248496D626C61636B686F6C65`, 79 parameters, a 7,917-byte state
capture, float32 processing support and an editor interface supporting HWND.
No editor was attached and no audio was processed. Its default audio layout is
one 16-channel input bus and one 16-channel output bus. Cleanup and transport
retirement passed; the same 54 protected files and prefix identity were retained.
See [repaired-host receipt](controller-repair.sanitized.json).

Normal setup installed that verified scanner, preserving 58 product records,
reports and proxy files plus the manager, frontend and other runtime helpers.
The first installed scan completed with three classes, no inspection error and
no quarantine. See [scanner deployment](controller-deployment.sanitized.json).

Luna's first **Check compatibility** click was refused with `operation already
running`. The worker's recorded outer lock acquisitions succeeded; the manager
uses a separate preparation-kit host and an inner fail-fast registry acquisition
after releasing its serialization lock. A concurrently refreshing UI can hold
that registry lock. This is a manager admission problem, not a Blackhole crash.

The preparation kit still contained its earlier host, so its host and source
manifest were updated to the same verified repair. All 73 other archive entries,
including the native builder and compiled native library, remained byte-identical;
their repository inputs are unchanged between the old recipe source and
`39c8da7`. Setup preserved 59 canonical records/reports/proxy files. With the GUI
closed, the offered **Check compatibility** action completed and retained the
exact controller association. Source `42c3d93` replaces the inner fail-fast lock
with the existing bounded, operation-attributed registry acquisition. It
rechecks inactivity while holding the verified guard, then drops that guard
before launching the inspector. The focused contention test, Rust checks,
Clippy with warnings denied and Linux release build passed. Normal setup
installed this manager, preserving the frontend, runtime helpers, preparation
kit and all 59 protected records/reports/proxy files. Previous software and
private launcher/manifest backups remain available for rollback. See the
[manager deployment](controller-manager-deployment.sanitized.json). The failed
GUI action remains retained separately from subsequent outcomes.

Luna then opened the updated manager normally and clicked **Refresh preliminary
inspection** once. Operation `4b449f7bfa0790ecb4d581984be44510` completed with
the exact separate-controller association. The new inner acquisition actually
encountered contention: four attempts, 30.522 ms total, then acquired. A transient
busy/status-unavailable presentation cleared without another action. Luna
subsequently observed **This operation completed**, the Eventide Blackhole
Immersive 1.4.4 card still **Installed - not published**, and **Prepare a test
candidate for this inspection and recipe**. The parent performed no GUI input.

Independent readback confirmed the completed receipt, installed manager hash,
all 59 protected files unchanged, active bridge with three keepers and zero
DSP/maintenance/pending transactions/stale transports, and no unconfirmed
cleanup. See [GUI operation and readback](controller-gui-readback.sanitized.json).
No preparation or publication was attempted. Inspection is complete; audio,
editor attachment, Bitwig loading and project recall remain unproved. Blackhole
declares a single 16-channel bus in each direction; the current descriptor/proxy
supports stereo buses. The next audio decision is whether to investigate real
stereo negotiation first or implement the full immersive layout.

## CI fixture corrections

The later CI run on `42c3d93` exposed two fixture defects, retained as failures:
the [manager run](https://github.com/kasselvania/Linux-VST-bridge/actions/runs/35464117061)
matched the text `123` inside an allowed observation timestamp, and the
[Windows run](https://github.com/kasselvania/Linux-VST-bridge/actions/runs/35464117029)
exited 82 immediately after readiness, before the controller query. The Windows
test wrote directly to the gate path, allowing the host to read incomplete
bytes. These failures do not invalidate the separately retained successful
commercial inspections, and they are not reported as passes.

Fixture-only source `2e4e600` checks forbidden private/PID field names
structurally and validates the complete readiness binding before publishing
a flushed gate file by atomic rename. Production host/gate behavior and all
four controller acceptance assertions remain unchanged. The 12 dependency
reporting tests, Python compilation and focused handshake/publication check
passed locally. Hosted platform results are tracked on the PR; these local
checks alone do not establish a passing Windows run for the corrected harness.

## Deferred user request

The operator asked to remember, not implement here, a classification of tooling
and reporting by scope: shared installer/plug-in infrastructure, cross-product
dependencies such as iLok, and individual plug-in/profile behavior. The operator
also reported Kontakt plug-in crashes without the reports seen for Arturia.
This is a reporting-coverage observation, not an attributed common crash cause.
The existing ordinary capture action excludes experimental Kontakt/Serum; its
relationship to those particular incidents remains to be established.
