# WD0 — Managed FL Studio workspace: first usable stock project

Selected by the operator on 2026-09-24. This is the implementation work order,
not a completed qualification. Architecture:
[WINDOWS_DAW_WORKSPACES.md](WINDOWS_DAW_WORKSPACES.md). Work allocation:
[WORKSTREAMS.md](WORKSTREAMS.md). Tracking issue:
[#158](https://github.com/kasselvania/Linux-VST-bridge/issues/158).

## Outcome and fixed scope

From the normal manager/desktop entry, open the operator's exact Windows FL
Studio in a dedicated managed workspace, make a short stock-instrument pattern,
hear it through Linux audio, save and export it, close cleanly, and reopen the
same project after a fresh launch when licensed.

Deliver one canonical implementation PR with that connected result, not separate
framework, installer-proof, audio-proof and closure PRs. A typed CLI, ordinary
desktop launcher and useful status are sufficient for WD0. The engineer may
repair in-scope faults and continue without a new review at each step. Review is
before merge; existing tool, spending, user-data and security permissions still
apply.

First fixture: x86-64 Steam Deck Desktop Mode, mouse/trackpad, one FL workspace,
stock installed content. No third-party VST installation, Ableton, Max for Live,
ARM, Gaming Mode, new distro campaign, full manager redesign or touchscreen
qualification in this cut.

## Canonical and installed starting point

Start from canonical main at or after:

```text
commit a4e140a53cedb8b65b487a45d6fea0e44caea74d
tree   b3f3d264901267e2dcb723b9904c464baf6eefae
```

That main includes the accepted Serum candidate-D runtime support and
`x11_touch_routing_v2` policy. Reconcile later reviewed changes normally; never
reset another agent's branch or worktree.

The implementation branch starts from the normal architecture merge
`9f37338140a61e838e83d757628b6cb914b83bb5`, tree
`3bb22f1d9971c97f01dc65a8227f1a7d7f727f54`. The separate WD0 package
selects a manager/supervisor generation under the canonical managed root; it
does not replace the native-bridge service or its software catalogue. Source
validation is not a physical FL result. The installer, first launch, audio,
project, export, recall, and retirement gates remain open until observed.

### First physical result and selected correction

The operator granted the Deck handoff. The exact FL Studio 26.1.6 Windows
installer was privately fingerprinted and imported without changing Downloads.
The managed installer displayed completion, but its outer worker returned a
nonzero exit; the x64 FL application is present and independently verified.
The first managed application launch displayed FL's program-validity error and
retired cleanly. This is neither an installation-success claim nor an audible
DAW result.

The selected correction is the narrow authenticated-attribute verification
change from [Wine merge request !11824](https://gitlab.winehq.org/wine/wine/-/merge_requests/11824),
adapted to the pinned Wine source. It preserves the decoded attribute order for
verification; it does not disable signature checking or change signing. The
FL-only immutable successor changes the Proton version marker and both
`crypt32.dll` PE architectures; the standard runner and native-bridge runners
remain untouched. The isolated Wine crypt32 message group passed 1,032 tests
with zero failures, and the successor passed an unlicensed startup/cleanup
smoke. A managed FL launch and the stock-project workflow are still required
to establish a physical fix.

The existing managed workspace was subsequently uninstalled cleanly. Its
schema-1 record and exact install/uninstall receipts remain on the Deck. The
source baseline for the lifecycle correction includes merged UI0 at
`8bb191b28eedee8aabf058b06706860c9fc64933` through a normal merge into
PR #163. The live uninstalled workspace is the acceptance fixture: do not
delete it, recreate its prefix, edit its JSON or run another installer before
source rereview and a new Deck handoff.

### Repeatable installation lifecycle

**Workspace lifetime is not installation lifetime.** The FL workspace keeps
one stable ID, prefix, selected runner, projects, preferences, exports and
lawful account state across version changes. The selected installer is a
separate exact SHA-256/release pair. Each install and uninstall is a new
manager-owned operation with an append-only history record and retained
receipt. A clean uninstall removes the current installed application claim,
not the workspace or earlier evidence.

The ordinary controlled version-change path is: cleanly uninstall A, select
admitted installer B by exact SHA-256 and release, install B, finish its exact
readback, then launch B. Reinstalling the same A after clean uninstall is also
admitted. Selection itself never executes Windows code or touches the prefix.
An installed version must be uninstalled before another install; there is no
automatic in-place upgrade or fresh-prefix reset in this slice.

Schema-1 records migrate deterministically at readback. Old installation and
uninstall operation IDs, installer identity, environment and user roots, runner,
first failure and receipts remain exact. An incomplete or ambiguous old
operation becomes historical unknown and blocks new mutation until explicit
recovery; migration must not manufacture a clean outcome. Current status shows
the latest current failure, while older failures stay in history after a later
success. Unowned FL files in the prefix are not adopted as a managed install.

Read [AGENTS.md](../AGENTS.md), [ARCHITECTURE.md](ARCHITECTURE.md), this work
order, [WINDOWS_DAW_WORKSPACES.md](WINDOWS_DAW_WORKSPACES.md), the support
matrix and failure ledger. Reuse relevant source owners, especially manager
installer/import, onboarding, exact application/process generation, operator
model and runtime supervision. Inspect their contracts before reuse: ASC and
Native Access selectors are deliberately closed and must not become arbitrary
program launchers.

Before physical work, read the actual Deck state and coordinate with the current
custodian. Preserve:

- the working shared native-bridge service and installed generation;
- Serum candidate D, its selected publication and `x11_touch_routing_v2`;
- the other five selected publications;
- vendor prefixes, Bitwig projects and user audio settings;
- every required exact Windows host/source pair.

Workspace-only development must not rewrite the native bridge's `software.json`,
profiles, publications, environment markers or service selection. If a later
common package is required, it must explicitly carry all installed runner
policies and host/source authorities forward before replacing the service.

## Physical Deck custody

The operator may be using Bitwig while WD0 source work proceeds. Only one task
owns live Deck mutation at a time.

Without the physical-work handoff the WD0 owner may perform source changes,
tests, read-only machine inventory, installer fingerprinting and isolated
unlicensed fixtures. It may not:

- execute the FL installer;
- stop or replace the shared manager/service;
- change Steam launch configuration;
- change PipeWire/JACK/device settings;
- alter a managed runner, prefix or publication;
- start an acceptance session that competes with the operator or another agent.

## 1. Admit the installer now present on the Deck

The operator reports that the official Windows FL Studio installer is present in
the Steam Deck user's `Downloads` directory. This report does not admit a
filename, alias, version or digest.

Begin with a read-only inventory of plausible installer files in that directory.
Do not execute any candidate merely because its name contains `flstudio`.
Identify and retain privately:

- exact canonical path and original filename;
- file type and size;
- SHA-256;
- PE machine/version/product/company facts where available;
- Authenticode/publisher/signature result where available;
- download/source handoff confirmation from the operator.

Preserve the original download byte-for-byte. Import one exact private copy
through the existing installer-custody pattern. Public source and evidence may
record sanitized version/digest facts, never the installer bytes, local username,
credentials or license material.

If more than one plausible installer exists, refuse ambiguity and present the
read-only candidates to the operator. Do not fetch a substitute build from the
internet or guess from the mutable vendor website. The exact file already handed
off is the selected input unless the operator explicitly replaces it.

## 2. Create one fresh FL workspace

Add a small adjacent typed `DawWorkspace`/FL owner in the canonical manager. Do
not use `ApplicationId::Other`, an arbitrary executable path, a free-form argv or
a shell command.

The workspace must bind at least:

- stable workspace ID and revision;
- exact selected FL installer and release, current installation, active
  operation, and prior install/uninstall results;
- exact installed FL executable/resources and observed version;
- a fresh manager-owned prefix and admitted immutable runtime closure;
- explicit writable preference, project and export roots;
- audio-backend identity and effective settings when known;
- application/session state, first useful failure and retirement certainty.

Executable/runtime generations are immutable. The prefix, FL preferences,
projects and lawful authorization state are intentionally mutable under their
owners; do not hash ordinary preference changes as corruption.

Use the verified standard Proton 11.0-2c / Steam Runtime 4 lineage as the first
runtime candidate. Do not inherit Serum's policy just because Serum works, and
do not mutate a shared runner. If FL needs a runner correction, create one
coherent workspace-specific successor that preserves every requirement selected
for that workspace.

Do not clone any existing Arturia, Serum, NI or Blackhole prefix. WD0 uses stock
FL content only. A demonstrated missing official dependency can be installed
narrowly inside this workspace; no shotgun winetricks bundle or unrelated cloud
content.

## 3. Install and identify FL Studio through managed ownership

Run the real installer only after the physical-work handoff, through existing
exact launch/root ownership. Let the operator handle vendor consent, account and
license entry. Do not automate credentials or select paid/optional products
without explicit direction.

Preserve distinct facts for:

1. installer launched;
2. installer exited;
3. installed application identified and verified;
4. first application launch succeeded;
5. helpers/process descendants were owned and later retired.

Installer success alone is not a working DAW claim. If the installer spawns a
second stage or updater, bind its exact process generation rather than trusting
the bootstrapper's exit code or a process name.

## 4. Implement the minimum canonical DAW operations

Expose typed operations using existing CLI/operator conventions; final spelling
is the implementer's choice:

```text
workspace import/install
workspace select-installer INSTALLER_SHA256 RELEASE
workspace finish-install
workspace launch
workspace focus
workspace status
workspace stop --graceful
workspace uninstall
workspace finish-uninstall
```

Normal launch selects the known workspace and exact installed application. It
does not accept a caller-supplied path, arbitrary flags, PID or process name as
authority. Project selection is restricted to approved project roots through
normal file selection, not interpolated shell arguments.

Required application states include at least:

```text
not-installed
installing
ready
running
needs-user-action
failed
cleanup-unconfirmed
```

A second launch focuses the same exact session or refuses clearly. FL is not a
fake bridge DSP instance and startup must not require a VST keeper. The manager
may report FL and its exact owned helpers; without a real FL API it must not
pretend each internally hosted device is a bridge-owned session.

Add an ordinary desktop entry that uses the same canonical launch route. It must
work from a normal graphical session without a development shell, ad-hoc exports
or an SSH session remaining alive.

Graceful stop preserves the user's save opportunity. A timeout is incomplete,
not success. Forced termination requires explicit operator confirmation of
possible unsaved-work loss and targets only the exact workspace cohort. Never
use global `wineserver -k`, `killall wine` or process-name-only cleanup.

An operator-requested `workspace uninstall` is a separate, exact FL application
operation: it runs only the installed vendor `uninstall.exe` under the same
workspace process owner, never an arbitrary executable or another Wine prefix.
`workspace finish-uninstall` records removal only after that cohort retires
cleanly and the installed FL executable is absent. It does not wipe the mutable
prefix, preferences, projects, exports, licensing state, or imported installer.

## 5. Keep the runtime view coherent

FL and its scan/helper processes share one workspace filesystem, registry, Wine
server and runner. Use pressure-vessel/Steam Runtime where required by the
selected runner. Do not create an independent Bubblewrap/Proton stack for each
in-process plug-in.

Project only the actual required surfaces:

- selected runtime dependencies;
- current graphical session and exact Xauthority;
- audio-service socket and admitted driver libraries;
- selected input devices;
- workspace data plus approved project/export roots.

Keep unrelated prefixes, credentials and home data outside that view. Online
install/unlock access is scoped to the workspace and selected operation; do not
disable host security or create a host-wide exception.

A file visible to the host shell is not evidence it is visible inside FL. Verify
runtime paths from the launched process view. Keep durable projects/preferences
outside `/run` and temporary evidence roots.

## 6. Establish real audio, then evaluate managed ASIO

First obtain audible stock-project playback through an actually available
Windows audio route. Record the exact backend selected, the Linux endpoint and
the effective sample rate/block where observable. A moving FL meter alone is
not audio proof.

Then evaluate the selected managed route:

```text
FL Studio ASIO client
  -> exact 64-bit WineASIO driver
  -> PipeWire JACK client implementation visible inside the runtime
  -> existing PipeWire server and selected output
```

Bind the exact WineASIO PE/Unix artifacts, runtime ABI, prefix registration,
loaded JACK library and endpoint. Register only in the FL prefix, never
`~/.wine`. If driver placement requires runner-tree files, create a private
immutable workspace-specific runtime/driver closure rather than hot-copying
into a shared runner.

`pw-jack` outside Proton is not proof that the intended library loaded inside
FL. Verify the actual process/runtime crossing. Do not autostart a second JACK
server. Keep WineASIO server autostart and uncontrolled shared-graph buffer
changes disabled. Leave machine-wide PipeWire configuration and scheduling
untouched.

Start at 48 kHz and 512 samples where supported to reduce variables, but record
FL's effective driver block and PipeWire/JACK quantum separately. This is not the
native proxy's 512 added frames and is not a measured round-trip latency claim.

If WineASIO is concretely incompatible, retain that failure and select at most
one evidence-led maintained alternative in this slice. Preserve working built-in
audio as an honest partial result rather than losing it in a driver tournament.

Hardware MIDI is separate. A piano-roll pattern proves FL event playback, not
physical MIDI or input latency. Exercise an operator-selected device only if it
is already available and does not distract from WD0.

## 7. Complete one small musical workflow

Use a protected new project and a stock instrument included in the selected
edition. No third-party VST or internet sound-pack download is required.

1. Launch FL through the new normal desktop/manager entry.
2. Create a short note pattern, arrange it and hear it through the selected
   Linux output.
3. Change one stock instrument control and one mixer level; confirm the audible
   result follows.
4. Save the FLP in the approved stable project root.
5. Export a short WAV through FL's own export operation; verify it is finite and
   nonempty and retain it privately for the operator.
6. Quit normally and verify the exact FL/helper cohort retires, while the native
   bridge service and all six publications remain intact.
7. Relaunch normally. When licensed, reopen the same FLP and confirm notes,
   setting, routing and audible playback. Close normally again.

FL trial mode can save/export but cannot reopen saved projects. Record a trial
limit as `trial_limited`; do not debug it as Wine corruption, automate licensing
or purchase anything. If licensed recall is unavailable, retain the useful
playback/export/clean-relaunch result and report the exact remaining gate. Do not
claim complete WD0.

Touchscreen coverage is not required. Use mouse/trackpad. The merged Serum touch
repair is a shared Wine lesson, not proof that the selected FL runtime already
contains or needs the same patch.

## Validation and delivery

Add focused production-helper tests for:

- closed workspace/application identity;
- immutable installer/runtime binding;
- repeated launch/focus behavior;
- actual child lifetime rather than launcher exit;
- exact graceful stop/refusal and uncertain cleanup;
- approved project/export roots;
- absent or changed audio endpoint/driver;
- coexistence with current native publications and runner policies.

Run touched manager/CLI tests, strict Clippy where Rust changes, relevant
Python/shell checks and documentation/link checks. Reuse existing installer and
supervision regressions where changed. Do not replay AP17, Ubuntu, Raspberry Pi
or unrelated plug-in campaigns.

Return one implementation PR with:

- exact source head/tree;
- admitted installer/release facts;
- workspace, runtime and driver identities;
- manager/desktop operations and ownership boundary;
- physical stock-project workflow result;
- ASIO, built-in audio and MIDI claims kept separate;
- cleanup and rollback posture;
- sanitized first useful failures and remaining limits.

Private installer bytes, projects, audio exports, credentials, license files and
raw vendor streams remain private.

Update failure-class and support documents only for actual changed understanding
or physical coverage. Until the workflow passes, FL Studio is planned/in
progress, not supported. A schema-only or launch-only PR is not WD0 completion.

## After WD0

WD1 installs one exact third-party Windows VST3 through its lawful installer in
the same FL workspace, lets FL discover it normally, and exercises play, editor,
save/reopen and retirement. It creates no Linux publication or bridge transport.
Select one product after checking the workspace runtime and licensing; do not
import the whole current fleet at once.

Ableton follows the proven workspace owner in a separate prefix, initially
without Max for Live. The native-Linux bridge remains the first release-driving
product throughout.
