# RPI2 — native ARM Wine/FEX on Pi 5

## Result

Actual Pigments now plays factory presets through the ARM Wine/FEX bridge. The
operator selected Zeus and reported that it played well. Altered Alter Boy was
saved, restored in a fresh process, and visually confirmed by the operator without
reselection; its saved master-volume value also matched. A further fresh process
restored that same state and produced stereo audio without opening the editor.

The short audio windows are mixed: Welcome and Zeus added no bridge gaps; Altered
Alter Boy added one 768-frame (16 ms) gap with its editor open at 71.6–74.35°C.
The same five-second note sequence in the fresh, editor-unopened process added no
gaps at 57.85–60.05°C. No current throttle/power flags or JACK xruns were reported.
This does not isolate temperature from editor workload or establish polyphonic
capacity. Sustained-load testing is deferred until the active heatsink arrives.
All three ordinary audio sessions closed cleanly. No new build was needed.

Previously, operator activation in ASC resolved the migrated fixture's observed
state-capture refusal: the unchanged probe returned 155,154 bytes in 150 ms before
editor attachment and 155,286 bytes in 168 ms afterward. Both calls had returned
`1` with zero bytes before activation. This strongly identifies authorization
readiness as the cause of that particular refusal, not every earlier RPI1 symptom.

GE-Proton11-7 AArch64 with UMU 1.4.4 now runs the reference Windows VST
through the existing native bridge on the Pi: architecture handshake,
MIDI, stereo processing, and clean shutdown passed. The native supervisor
release build took 17.18 seconds; the existing Windows host and VST binaries
were reused.

A subsequent matched Box64/Proton versus ARM Wine/FEX comparison completed
four 30-second runs with identical captured audio. FEX reduced mean time
inside this light synth, but did not reduce total runtime CPU use or establish
better deadline behavior. The changed topology is real; a general performance
win is not established.

Before activation, the first real Pigments attempt stopped before audio/editor startup:
Pigments initialized and exposed its 4,446 parameters, but `IComponent::getState`
returned `1` without writing any bytes. No preset selection or playback was
reached. This is a concrete state/initialization failure, not DSP or thermal
evidence. Details and the preserved failure are below.

The approved follow-up reached Pigments' editor through the existing vendor-access
host mode: editor-open, editor-close, component termination and module-unload
records were obtained. This separates the initial state refusal from the ability
to attach its editor. A subsequent single-call probe found that state capture
still returned `1` with zero bytes after 5.007 seconds and 401 message-pump turns.
Rendered appearance and responsiveness were unobserved at that stage. The earlier attempts,
including two launch-adapter mistakes, are preserved below.

A read-only follow-up found a vendor machine-identity error in the migrated
environment: ASC reports `Can't read file <REDACTED_PATH>: machine key changed.`
It appears 39 times across three retained ARM-environment agent logs and zero
times in the three original-environment agent logs. This is a concrete migration
fault to resolve before interpreting state refusal as a host defect. The later
activation intervention and state result are recorded below.

The preceding account-free process launch proof needed no source build. Its host
published its environment readiness token, pumped its Windows message loop,
accepted its stop file, and exited successfully. All 14 processes sampled
in the owned test cohort were absent after shutdown.

The running host mapped `libarm64ecfex.so` and `libarm64ecfex.dll`. Its Wine
preloader, Wine server, Python launchers, and pressure-vessel processes were
native AArch64 ELF executables (machine 183). No Box64 process participated
in that observed cohort. This is direct evidence of the changed execution
topology, not a performance prediction:

```text
Native ARM UMU / Linux runtime / Wine Unix infrastructure
    └── FEX ARM64EC translation
          └── existing x86-64 Windows host
                └── reference VST → existing bridge transport → ARM JACK
```

This experiment is separate from RPI1 and the ordinary `CURRENT_SLICE.md`.
It does not declare RPI1 a failed architecture. It establishes another
runtime candidate on which the transferable bridge has now run.

## Reference bridge result

One session processed 9,152 Windows blocks over about 49 seconds, including
a five-second MIDI/audio capture. At 48 kHz, JACK used 512-frame callbacks,
Windows processed 256-frame blocks, and the bridge retained its existing
2,048-frame reserve (42.67 ms).

- Native and Windows architecture handshake bytes matched.
- The note-on, note-off, and CC123 messages were sent without errors.
- Both captured channels were identical and finite, with 73,766 nonzero
  samples each and peak amplitude 0.04535433. Initial and final silence
  were preserved; the active note measured approximately 262 Hz, consistent
  with MIDI note 60 at the measurement's 2 Hz resolution.
- All 4,576 callbacks completed without process failure, deadline miss,
  missing/expired frames, or a gap. The capture reported zero JACK xruns.
- The host containing the reference VST also mapped FEX; all 14 sampled
  cohort executables were native AArch64 ELF images.
- Close completed successfully, the session directory was removed, all
  sampled cohort PIDs disappeared, and the original JACK graph returned.

The implementation changes runner selection and supervision in the RPI0
standalone. A pinned native launcher and its pinned native leader replace
the Box64-specific fields when `runner=native` is selected. Existing Box64
configurations and their executable checks remain supported. Native cohorts
use leader identity and cgroup ownership, rather than requiring every child
to be Box64; this is process ownership, not a sandbox-security claim. Cleanup
checks cgroup population including descendants. A startup identity failure
also stops the launched unit before returning an error.

[`launch-host.sh`](../../rpi2/launch-host.sh) is a fixture adapter copied beside
the staged runner. It requires the RPI2 private prefix, checks the selected
Proton and runtime entry hashes, preserves Windows command paths, and execs
UMU under the existing supervisor. The native cohort is bounded to 90 seconds,
3 GiB memory, and 512 tasks. This does not define a distributed product runner.

The existing JACK fixture now accepts `LVB_QUALIFICATION_CLIENT` (default
`lvb-arm-pigments`) and the neutral `notes` alias. The run selected
`lvb-arm-standalone`; its note schedule, meters, and processing stayed intact.

Validation: 21 standalone Rust tests passed, including unchanged legacy
configuration parsing, native selection, mixed/unknown runner rejection,
and changed-file hash rejection. The live run verified the changed launch
path. Exact tested source hashes, configuration, native/Windows logs, audio
summary, handshakes, and cleanup are in
[`reference-bridge.json`](../../evidence/rpi2/reference-bridge.json).

The retained timing window covers 1,643 of 9,152 requests: mean Windows
processing was 11.174 microseconds and mean admission-to-publication was
249.611 microseconds. These are a partial-window description of this light
reference workload, not a complete trace or a comparison with RPI1. No DSP
or transport algorithm was changed. Pigments performance and behavior remain
separate questions.

Reading the implementation establishes why that timing population is partial:
`Session::process` arms observations on the first note-on or nonzero input
(also unconditionally for protocol minor 6). The initial silent interval
was not timed. Audio-window retention is a separate limit.

## Matched runtime comparison

The comparison reused the exact native bridge, Windows host and reference
VST binaries. It needed no build. Each run sent the existing 47-message,
30-second sequence of 1, 2, 4, 6 and 8 voices, with note-offs and quiet tails.
The order was Box64, GE, GE, Box64. JACK remained at 48 kHz / 512 frames,
Windows blocks at 256 frames, and the bridge reserve at 2,048 frames.

The Box64 lane used RPI1's pinned Box64 0.4.4 and Proton 11.0-2c build
25118279 with Steam Linux Runtime 4.0.20260805.254769. It used a newly
prepared account-free prefix; the licensed Pigments prefix was untouched.
The GE lane used the staged ARM environment described below. This compares
the complete selected stacks, including their different Wine/runtime versions.
It cannot isolate the translation engines' contribution.

| Measurement | Box64 / Proton | ARM Wine / FEX |
| --- | ---: | ---: |
| Runtime cohort CPU, percent of one core | 10.21% | 10.69% |
| Native bridge CPU, percent of one core | 4.42% | 4.46% |
| Mean Windows process call | 19.47 µs | 15.65 µs |
| Mean admission-to-publication | 298.71 µs | 292.19 µs |
| Admission-to-publication p95 upper bound, each run | 448 µs | 448 µs |
| Maximum admission-to-publication observed | 1.84 ms | 4.05 ms |
| JACK xruns / bridge missing frames | 0 / 0 | 0 / 0 |

Means are the equal-weight average of two runs. CPU covers about 31 seconds
per run, excluding startup, and is measured separately for the native bridge
and hosted runtime cgroups. JACK, the MIDI/capture fixture, and sampling are
outside those CPU totals. Timing covers 5,713–5,731 requests per run from
first note-on onward. Quantiles are histogram upper bounds, not exact ranks.
Admission-to-publication measures internal block turnaround, not physical
key-to-speaker latency; the 42.67 ms bridge reserve remains unchanged.

All four complete stereo recordings were byte-identical: 1,440,000 frames
each, finite samples, matching channels, increasing held-note RMS across
the five voice steps, and a silent final second in every step. All 47 MIDI
messages were sent per run without errors. Every run closed cleanly and
removed its session. No matching experiment units remained and the original
JACK graph returned.

Sampled temperatures were 51.25–56.75°C, with current throttle/power flags
clear. Historical flags remained `0xe0000`. The unchanged `ondemand` governor
reported 1.6 or 2.4 GHz in one-second samples. Starting temperatures differed;
four short runs do not establish relative thermal efficiency or sustained
headroom. Neither stack reached a capacity limit here.

### What the implementation explains

The new measurements expose a large fixed cost outside this small synth:

- The native queue worker requests a 50 µs sleep when no work is queued
  (`native-vst3-proxy/backend/src/queued.rs`).
- The Windows delivery worker polls the shared mailbox with
  `NtDelayExecution` requesting 50 µs (`delivery_mailbox.h`). Existing startup
  measurements put the actual interval around 104–107 µs on both stacks.
- The owner services control/UI work between requested 50 µs sleeps
  (`offline_processing.cpp`). The existing calibration measured that sleep
  mechanism at about 1.06 ms.

During capture, the busiest Windows host thread accumulated roughly
9,570–9,941 scheduling slices per second, while the bridge issued only
187.5 process blocks per second. GE identifies that thread as `lvb-audio`;
Box64 retains the process name. Its measured CPU time was 2.30–2.32 seconds
on GE and 1.97–1.98 seconds on Box64 over each roughly 31-second window.
The two referenced Windows source files are unchanged from the reused
binary's source commit `791088bb31fcd75212b2df4ac2c7a6efb95182ba`.

Timer polling is therefore a concrete candidate for shared overhead. These
observations do not attribute every CPU cycle to waiting, nor explain
Pigments' failures: no CPU stack profile was collected. The current worker
also does protocol, buffer and validation work outside the plugin call.
Faster DSP alone leaves these costs in place. The GE run's 4.05 ms outlier
also prevents a claim that the new topology eliminated spikes.

The next transport change worth evaluating is notification-driven waiting
with bounded failure behavior, keeping all waits outside the JACK callback
and preserving Windows owner/message-loop service. Use this exact workload
to measure whether it lowers CPU without worsening timing. A demanding
plugin comparison is still needed to establish useful compute headroom.
Saving volume, restoring state, and mapping ShieldXL keys to preset changes
remain separate host/control questions; neither runtime comparison exercised
or repaired them.

### Reusing this comparison

[`comparison_runner.py`](../../rpi2/comparison_runner.py), copied privately as
`compare-ge.py` and `compare-box64.py`, keeps the same pinned native Python
leader on both lanes. Consequently both configurations say `runner=native`:
that field selects supervision, not the guest instruction set. The Box64
child still uses the translated x86 Linux runtime. Only its `--prepare` mode
creates the separate reference environment. Entry/adapter hashes pin the
existing packages; no runner is rebuilt or updated.

[`compare_bridge.py`](../../rpi2/compare_bridge.py) runs one named session
against those staged paths, under the existing supervisor and an exclusive
private lock. It checks for inactive RPI1 audio, starts below 56°C, refuses
current throttle/power flags or a sampled temperature of 75°C, and retains
failure logs. Reuse with a new label; directories are never overwritten.
These are fixture scripts, not a product runner API.

Exact configurations, script hashes, full sanitized runtime logs, timing,
CPU samples, thread deltas, audio summaries and cleanup are retained in
[`runtime-comparison.json`](../../evidence/rpi2/runtime-comparison.json).
The one-time Box64 prefix preparation took 21.24 seconds and retained the
runtime's startup warnings; it was excluded from comparison CPU timing.

## Pigments: first ARM-runtime attempt

The operator selected the already-installed Pigments as the real workload.
The requested sequence was to launch it on ARM Wine/FEX, open its own editor,
select a different preset, and play it. The existing default-patch and button
results did not establish that changing presets was reliable.

The Pi executable is **our standalone VST3 host**. Pigments remains a VST3
inside the Windows host, rather than the vendor's standalone application.
Earlier DAW-hosted VST operation is a different host context. Runner topology
and standalone-host lifecycle/controller/message-loop behavior are separate
variables; neither can be declared the cause from this one attempt.

The Pigments-specific standalone now selects either its existing Box64/Proton
runner or a pinned native launcher/leader. Its class, complete bus contract,
MIDI policy, initialization wait, editor lifetime, state operations and
process-scoped retirement policy stay intact. Runtime configuration is distinct
for each runner. The native branch rejects Box64 diagnostic selectors and
waits for the pinned leader executable before accepting launch identity.
Startup failure stops the owned unit. Its cohort has a 300-second lifetime,
3 GiB memory limit and 512-task limit.

A private copy of the installed environment was made on the same Pi before
launch; copying took 303.58 seconds. GE-Proton migrated that copy. The original
installation and selected-state file were retained; the original selected
state still matches its recorded digest. Copying authorization files does not
prove that vendor authorization accepts a changed runtime/machine identity.
No authorization repair, new activation or license manipulation was attempted.

[`launch-pigments.sh`](../../rpi2/launch-pigments.sh) reuses the pinned ARM
runtime and launches the current installed Windows host,
`wf0-factory-probe-rpi1-e232.exe` (SHA256 `64d629e84a0fdf8833e97b9393cd41b9a5a214f82fe418d1792281622f0007f2`).
This is the host selected by the current private Pigments configuration, not
the older host used in the reference-synth comparison. Pigments is unchanged
at 7.0.1.6772, SHA256
`bdc91ebef8e5b486c8f998f1eef6a99626dd5a0b46d986263eeb1980b96a3c07`.
No Windows host or plugin was rebuilt. The native release build took 35.67
seconds; a final leader-readiness adjustment rebuilt in 10.61 seconds. All
14 library tests passed on macOS, including both runner launch/environment
paths; the resulting ARM executable ran on the Pi.

`pigments-ge-01` lasted 92.96 seconds, including first migration/startup:

- Native and Windows architecture handshakes matched.
- Module loading, component creation/initialization, audio-processor query,
  combined controller association and component-handler installation succeeded.
- The existing four buses and 4,446 parameters were reported.
- `getComponentState` returned SDK result `1`; its stream received zero writes,
  zero bytes and no unsupported interface queries or stream failure.
- The Windows host reported `capture_available=false`. The native startup
  observer correctly refused with `initial component state unavailable`.
- Temperatures ranged from 50.15 to 57.3°C, with current throttle/power bits
  clear. This did not exercise audio DSP.
- The owner exited with failure and retired the runtime cohort. No experiment
  units remained; the original JACK graph returned. Session files and logs
  remain private for failure analysis. This was failure cleanup, not successful
  plugin lifecycle retirement.

[`pigments_session.py`](../../rpi2/pigments_session.py) retains this single
session and its temperature/power samples. Sanitized configuration, exact
source/binary identities and lifecycle records are in
[`pigments-native-initial-state-failure.json`](../../evidence/rpi2/pigments-native-initial-state-failure.json).

The first failure was preserved rather than skipped to manufacture an audio
result. The next discriminating observation is Pigments' vendor editor:
an activation/content prompt and an initialization/host-semantics failure need
different responses. The existing `ap12-vendor-access` mode can open that
editor without claiming working state or DSP. Editor observation has not been
completed in this attempt. The operator has ruled out Screen Sharing through
computer control. Editor inspection, preset
selection and playback remain unperformed; do not infer them from successful
module initialization.

## Pigments editor investigation

The operator approved up to 25 minutes of active work and three live launches
from `9dbad8f`, with at most one targeted Windows-host build taking no more than
five minutes. Scope was editor access and state refusal, without audio,
Screen Sharing, computer control, credential changes, runtime installation or
prefix replacement. No Windows build was run.

The existing `ap12-vendor-access` mode permits ordinary state refusal, creates
the VST3 editor on the controller thread, attaches its Win32 parent and enters
the message-pump loop. It has no native audio peer or JACK session. The private
[`pigments_editor_session.py`](../../rpi2/pigments_editor_session.py) adapter
uses that existing mode, verifies the configured file hashes, holds both the
RPI2 run lock and original Pigments operation lock, and supervises a bounded
systemd cohort. The original environment remains unused.

Three launches were made:

| Attempt | Result | Elapsed |
| --- | --- | --- |
| editor-ge-01 | Adapter supplied named arguments in an order rejected by the host; no readiness or Pigments loading. It also incorrectly requested an audio architecture handshake without a peer file. | 6.03 s |
| editor-ge-02 | Audio-peer request removed; argument-order error remained. Again no readiness or Pigments loading. | 6.01 s |
| editor-ge-03 | Correct argument order reached Pigments initialization and editor attachment. | 62.60 s including cleanup |

The first two results are implementation mistakes in the experiment adapter,
not Pigments failures. The argument-order error precedes architecture negotiation;
removing the architecture request alone could not fix the first refusal.

On the third run, Pigments again returned `1` from initial `getState`, with zero
bytes/writes and no stream error. Nevertheless, the host reported
`ap12_vendor_access_open` about 25.23 seconds after launch. In the host source,
that event follows successful view creation, HWND attachment, sizing and the
window-show request. It does not establish correct rendering or responsiveness.
The existing message-pump loop was used; no independent pump-count measurement
was added.

The adapter issued the existing `vendor.stop` file 15 seconds after observing
the editor-open record. The host subsequently reported editor closure,
`terminateComponent` result 0, inspection exit 0, successful module exit/unload
and scanner completion. The runtime owner remained alive and was stopped by
the adapter's cleanup bound, returning 241. Successful plugin teardown and
the runtime wrapper's required cleanup are distinct observations.

The run logged `Failed to create DXGI factory.` before reporting editor open.
That is a graphics-initialization lead, not proof of blank rendering or lack of
a fallback. No explicit vendor activation/content verdict was obtained. The
host's `save_available=false` field and its window-title annotation describe
the **initial** state request, not a later vendor status observation.

Temperatures were 47.95–56.75°C; current throttle/power flags remained clear.
The runtime cohort reported 2.3 GiB peak memory. This included editor startup
and no DSP; it is not an audio-performance or sustained-thermal measurement.

**The after-editor state request remains unmeasured.** The existing access mode
does not issue that request. All three authorized launches were consumed,
including the two adapter mistakes. No fourth launch was made, and no untested
Windows-host change was substituted. The next discriminating operation is one
owner-thread state request after attachment and message pumping, while audio
remains inactive. It would distinguish a persistent refusal from one relieved
by normal editor initialization; it would not by itself identify licensing.

All experiment units were gone after cleanup, the JACK graph was unchanged,
and the original selected-state digest still matched. No original installation,
activation, password or credential change was made. All three runs, sanitized
host records, warnings and exact limitations are retained in
[`pigments-editor-open.json`](../../evidence/rpi2/pigments-editor-open.json).

## State capture after editor attachment

The operator then authorized the single post-editor state request. One focused
Windows-host build and one Pigments launch completed this operation. The
existing AP8 workflow gained an optional `host_only` dispatch selection with
a five-minute job timeout. It builds `wf0-factory-probe` and its source manifest;
the full workflow remains the default. The selected job passed in 99 seconds,
including setup and upload. No broader test matrix was run in that job.

The source change is confined to the existing inspection/access implementation.
`LVB_VENDOR_ACCESS_STATE_RECHECK=after-editor-pump` opts into one extra
`IComponent::getState` call after at least five seconds of successful pump
turns. The call stays on the existing controller thread. It records timing,
result and stream counters, without restoring state, changing parameters,
synchronizing a controller a second time or exporting the state payload.
Ordinary access sessions do not opt in.

The new host was staged beside the previous host in the same copied Pigments
environment. A separate configuration selects it, retaining the previous
host, launcher and configuration. Pigments and the runtime were unchanged.
The source difference from the prior host also includes preset-census code
which is guarded out of this editor-access mode.

`editor-ge-04` produced:

| Request | Result | Bytes / writes | Call duration |
| --- | --- | --- | --- |
| Before editor creation | `1` | `0 / 0` | 12,634 ms |
| After editor attachment and pumping | `1` | `0 / 0` | 41 ms |

The second call began 5,007 ms after editor opening, after 401 completed pump
turns. The owner-thread match was true; the stream remained quiescent, reported
no failure and received no unknown-interface queries. The host reported editor
closure, successful component termination, module unload and scanner completion.
The runtime wrapper subsequently required the existing bounded cleanup, as in
the preceding editor-only run. No experiment units remained, the JACK graph
returned unchanged, and both the previous host digest and original selected-state
digest still matched.

This rules out missing editor attachment/pumping as a sufficient explanation
for this particular refusal during the observed interval. It does not prove
that all vendor initialization has completed, or distinguish authorization,
content readiness, runtime behavior or other host-lifecycle requirements.
The 12.634-second initial call is a concrete startup delay inside `getState`;
wall-clock timing alone does not identify CPU work versus waiting.

`Failed to create DXGI factory.` remained in the log. Neither its rendering
impact nor a vendor activation/content verdict was observed. No audio processing,
GUI input, Screen Sharing, credential changes or state restoration occurred.
The next useful observation is the vendor's own activation/content/readiness
status in this environment, rather than another unchanged state request.

Build identity, both call records, pump count, temperature samples and cleanup
are retained in
[`pigments-post-editor-state.json`](../../evidence/rpi2/pigments-post-editor-state.json).

## Vendor readiness: machine-key error after migration

The follow-up inspected existing ASC diagnostic logs and resource metadata;
it performed no launch, build, GUI access or activation action. The three
native-environment agent logs contain 21, 15 and 3 instances of
`Can't read file <REDACTED_PATH>: machine key changed.` The original environment's
three retained agent logs contain none. These are different retained log windows,
not matched trials or a failure-rate comparison. The latest native log was
updated during the post-editor state experiment.

The referenced file exists in both environments, is a regular file of the same
size, and has mode `0600` with the current Unix user as owner. Its contents were
not read. Thus the apparent read failure includes an explicit machine-key reason;
it is not evidence that the file was omitted from the copy. The investigation
does not identify how the vendor derives that key or which migration change
caused the difference.

[Arturia documents that demo mode disables save and load](https://support.arturia.com/hc/en-us/articles/5671785160732-Demo-versions-What-should-I-know).
That makes the machine-identity error a plausible explanation for refused state
capture, but current Pigments demo mode and causality have not been established.
No fresh ASC GUI status is available: all three GUI logs match those in the
original environment. The driver warnings and licence-server `ret:2` also occur
in original agent logs; neither is classified as a new ARM failure, and the
meaning of that return code remains unknown.

The selected content inventories also match by relative filename and size:
8,359 files across Pigments resources/binaries, samples, presets, and shared
images/binaries. This checks copy completeness within those six subtrees; it
does not prove byte identity or successful Windows-side resource loading. The
new Windows-profile alias resolves to the existing copied profile.

The next useful operation is the vendor's normal activation check in this
environment, followed by the already-built state probe if activation succeeds.
No licensing or machine-identity files should be edited to force that result.
This finding concerns the migrated environment; it does not explain RPI1's
earlier audio, preset, control or thermal failures. No experiment units were
active at the end, and the original audio service remained inactive.

The sanitized comparison, scope and limitations are retained in
[`pigments-vendor-readiness.json`](../../evidence/rpi2/pigments-vendor-readiness.json).

## ASC startup in the migrated environment

At the operator's request, `asc-ge-01` launched the installed Arturia Software
Center 2.12.0.3157 in the same copied environment through GE-Proton11-7 AArch64.
No build or installation was needed. The pinned launcher now accepts the exact
ASC executable path as well as the two existing host paths; its staged ASC copy
is separate from the existing audio and editor launchers.

The service started at 03:38:42 UTC on September 23, 2026, on the existing X11
display `:1`. At 03:39:26 UTC both ASC and its agent were present under native
AArch64 `wine-preloader`, and ASC had written a fresh GUI log. Temperature was
49.05°C. No DXGI-factory failure, unhandled Wine exception or thermal-stop marker
appeared in the captured runtime log at that observation. Agent logs continued
to report the machine-key error.

The session was left running for operator access under the existing five-minute,
3 GiB and 512-task bounds. It holds both operation locks, refuses an active
original audio service, checks the ASC executable digest, and stops at 75°C or
current throttle/power flags. Systemd owns cleanup of its process group. The
maximum lifetime ends around 03:43:43 UTC; successful final cleanup is not claimed
from this startup observation.

No rendered window, responsiveness, sign-in or activation has been observed.
The operator was asked whether the window is visible. Shell syntax checks passed;
the actual launch, sanitized log metadata, process classification and limits are
retained in [`asc-native-startup.json`](../../evidence/rpi2/asc-native-startup.json).

### Operator access recovery

The operator then requested repair of the SSH video connection and lost VNC
password. The Pi's existing TigerVNC desktop was still active and restricted to
loopback, but the Mac had no video tunnel listener. The existing SSH master now
forwards Mac `127.0.0.1:5901` to the Pi's loopback VNC listener. A replacement
desktop password was installed atomically, with the old file privately backed up,
and saved in the Mac Keychain as **Pi desktop over SSH**. No credentials were
printed or added to this repository.

An authentication-only RFB handshake through the restored tunnel succeeded;
it requested no framebuffer and sent no input. The desktop server retained its
process identity. The ordinary Mac Screen Sharing client was opened for the
operator, with the password on their clipboard and a Keychain-backed copy
shortcut retained locally. The operator confirmed: "Yes, the desktop is visible."

ASC was relaunched as `asc-ge-02` at 03:49:48 UTC on September 23 with the same
runtime, copied environment, verified launcher/guard and five-minute limit.
Its first session had already ended. Activation remains unconfirmed. The access
repair is recorded in
[`desktop-access-recovery.json`](../../evidence/rpi2/desktop-access-recovery.json).

## State capture after operator activation

The operator completed activation in ASC session `asc-ge-03`, which used the
requested longer, 30-minute allowance. After the operator reported "Activated",
the ASC cohort was stopped and the existing `editor-ge-05` probe was run once
with `appliance-state-recheck.conf`. The Windows host, Pigments module, probe
script and configuration were unchanged from `editor-ge-04`; no build was needed.

| State request | Before activation | After activation |
| --- | --- | --- |
| Before editor attachment | Result 1, zero bytes, 12,634 ms | Result 0, 155,154 bytes, 150 ms |
| After editor pumping | Result 1, zero bytes, 41 ms | Result 0, 155,286 bytes, 168 ms |

The second call followed 5,006 ms and 418 completed pump turns. Both successful
captures used one stream write without stream failure or unknown interface
queries; the second call retained its owner thread and quiescent stream. The
initial success precedes editor attachment: this state-capture operation does
not depend on an attached editor in the activated fixture. The state payloads
were neither exported nor compared, and no restoration or save/reopen was tested.

The new agent log generation contained zero `machine key changed` messages.
Its previous generation contained 552; the file shrank on restart, so these are
separate generations, not a delta over an append-only log. Together with the
operator's activation and unchanged-probe result, this strongly attributes the
earlier migrated-environment state refusal to authorization readiness. Normal
ASC activation may also update preferences; the experiment does not isolate
every such change or explain unrelated RPI1 failures.

Editor attachment still took 33.29 seconds overall; the state-call improvement
does not explain the remaining startup time. Temperature ranged from 49.6 to
56.2°C, with current throttle/power flags clear and a reported 2.3 GiB memory peak.
The DXGI-factory warning persisted. Editor closure, component termination,
module unload and scanner completion succeeded, while the runtime wrapper still
needed bounded cleanup (exit 241). No experiment units remained and the JACK
graph matched the previous probe. No audio was activated.

The state and timing comparison, identities and cleanup are retained in
[`pigments-post-activation-state.json`](../../evidence/rpi2/pigments-post-activation-state.json).

## Factory presets, audio and process-restart recall

Three ordinary audio sessions reused the installed native standalone and existing
Windows host. Pigments is still the VST3 loaded by our host executable; this does
not exercise Arturia's standalone application. The ordinary host is distinct
from the focused post-editor diagnostic host above. No binaries were rebuilt.
JACK stayed at 48 kHz / 512 frames, Windows blocks at 256 frames, and the bridge
reserve at 2,048 frames. With Pigments' 48-frame latency, reported latency is
2,096 frames (43.67 ms), not a measured physical key-to-speaker delay.

The operator confirmed Welcome and selected Zeus and Altered Alter Boy using the
vendor editor. The initial suspicion that the editor had frozen was withdrawn
by the operator. No screen capture or computer control was used.

| Preset / condition | Captured sequence | New bridge gaps | JACK xruns |
| --- | --- | ---: | ---: |
| Welcome, before opening editor | One note, 5 s | 0 | 0 |
| Zeus, editor open | One note, 5 s | 0 | 0 |
| Zeus, editor open | Increasing held MIDI keys, 30 s | 0 | 0 |
| Altered Alter Boy, restored, editor open | One note, 5 s | 1 / 768 frames / 16 ms | 0 |
| Altered Alter Boy, restored, editor never opened | One note, 5 s | 0 | 0 |

All captures contained finite stereo audio and no MIDI delivery errors. The
five-second fixture sends note 60 at velocity 96, its note-off, and CC123. The
operator identified Zeus as monophonic and reported that it played well. The
30-second sequence had already been queued; its 1/2/4/6/8 held keys therefore
do **not** establish polyphonic capacity. Master volume was set to 0.45 for each
capture. Sample equality was not an acceptance criterion.

After Altered Alter Boy was selected in `pigments-ge-02`, the host saved 365,534
bytes privately and shut down cleanly. Session `pigments-ge-03` restored those
bytes before opening its editor, read back the saved master value
`0.59233081340789795`, and the operator confirmed: "Yes, Altered Alter Boy was
restored." This establishes one preset/master recall across a full process
restart. It does not establish DAW project recall, machine-reboot persistence,
or ShieldXL preset-button mappings. The state payload is not exported.

The editor-open Altered test added a 16 ms gap at 71.6–74.35°C. Its counter
interval includes fixture connection and startup as well as the note; it does
not prove DSP caused the gap. The larger chord test was skipped. Closing the
editor produced lifecycle 8 and temperature fell from 73.25 to 66.65°C over 24
seconds. The existing policy retains the view until instance retirement, so
this close was not view destruction. The session then reached its time bound
and closed cleanly; no editor-hidden note repeat occurred within it.

Instead, `pigments-ge-04` restored the same state in another fresh process and
never opened the editor. Its five-second capture added no gaps at 57.85–60.05°C.
The runtime cohort's recorded memory peak was about 1.13 GiB, versus 2.66 GiB
in the editor-open Altered session. Process lifetimes, starting temperatures
and UI histories differed. This establishes useful editor-unopened behavior,
without isolating graphics cost, proving a memory leak, or attributing the
earlier gap to heat. Full readiness took 19.04–27.24 seconds across the three
sessions; this is not isolated preset-load time.

Every session recorded an initial 1,536 missing frames. Existing phase records
place the additional pre-playback gaps in the two editor sessions between the
editor-open request and completion of attachment. The editor-unopened session
retained only its initial gap. None reported callback deadline misses, process
failures, or active throttle/power flags; the register stayed at its historical
`0xe0000`. A bridge gap is observable even when JACK reports no xrun. Initial
queue filling and editor startup remain specific work to examine, without a
claim about the underlying cause.

All sessions exited successfully, removed their session directories, and
returned the original JACK graph. No experiment units remained. Existing phase
capture stayed enabled, producing private files of approximately 24–93 MB per
session; its control-thread work is part of this configuration. No new tracing
was added. Sanitized counters, audio summaries, phase markers, identities and
cleanup are in
[`pigments-audio-presets.json`](../../evidence/rpi2/pigments-audio-presets.json).

The planned active-cooling follow-up was to reuse Altered Alter Boy, volume and note
sequence, first without opening the editor and then with it open, at comparable
starting temperatures. Only then increase held notes to characterize polyphonic
capacity. Heat, editor startup, save/recall, preset selection and ShieldXL control
ownership remain distinct questions. A general runtime performance win has not
been established.

### Active cooling: first battery-powered chord sequence

After fitting a fan, the operator found that cooler clearance prevented proper
GPIO seating. That boot had no ShieldXL audio card: CS4270 reported four I2C
lost-arbitration messages and probe error -11, and JACK could not start. No
Pigments test ran. The operator subsequently removed U2, reporting damaged pads,
to gain clearance. Upstream identifies U2 as the HCPL-0631 MIDI-input optocoupler;
the CS4270 audio codec is U1. Physical TRS MIDI input is excluded from this fixture.

On the next boot, ShieldXL audio and JACK were available, all six control devices
were registered, and the operator saw the display. Idle temperature was 24.85°C
with no throttle flags. The operator reported battery-only PiSugar power and a
running fan. Control-device registration is not a post-modification physical
knob/button test, and the successful audio check does not establish that every
damaged trace is healthy.

Session `pigments-cooled-01` reused the installed Pigments native and Windows
hosts without rebuilding. Only the private X-session fingerprint was refreshed
after verifying the current user-owned session; the previous config was retained.
The host restored the same 365,534-byte Altered Alter Boy state, confirmed master
0.45, and retained 48 kHz / 512-frame JACK / 256-frame Windows blocks / 2,048-frame
reserve. The vendor editor was never opened.

| Check | Audio duration | Temperature during fixture | Added gaps / JACK xruns |
| --- | --- | --- | --- |
| Single-note repeat | 5 s | 29.8–31.45°C | 0 / 0 |
| 1, 2, 4, 6, 8 held MIDI keys | 30 s | 29.25–35.85°C | 0 / 0 |

The chord fixture holds each chord for three seconds within a six-second window,
then sends note-offs and CC123. All 47 chord-sequence MIDI messages arrived without
errors, both channels produced finite audio, and the operator reported it was
clean throughout, including the larger chords. Both intervals added zero missing
or paused frames, process failures or callback deadline misses. The whole session
peaked at 35.85°C with no throttle flags. Sampled Pi input voltage during the chord
fixture was 4.97274–5.0384 V; that is not a battery-current or endurance measurement.

One 1,536-frame startup gap preceded the intervals and remains unresolved.
Readiness took 43.23 seconds; the session closed cleanly after 130.47 seconds,
left no new session directories or experiment units, and restored the JACK graph.
The Pi remained powered on. Audio and state stayed private; sanitized observations
are in `evidence/rpi2/pigments-active-cooling.json`.

This establishes a short, fan-cooled battery workload with larger chords and
operator-confirmed sound. It does not establish sustained eight-voice capacity,
maximum polyphony, editor-open behavior, or an isolated causal comparison of the
cooler. The editor-open repeat from the original follow-up plan remains undone.

## What ran

The fixture was Raspberry Pi 5, Debian 13.7, kernel
`6.18.50+rpt-rpi-v8`, AArch64, 4 KiB pages. The existing X11 session was
used. RPI1's audio service was inactive before and after the work; its
licensed environment was not used or changed.

All new packages, dependencies, logs, and the account-free prefix were
staged in a separate private directory. The Debian launcher packages were
extracted there; nothing was installed system-wide. Runtime updates were
disabled after UMU's first resolved installation.

| Component | Exact selection |
| --- | --- |
| GE-Proton | [GE-Proton11-7 AArch64 release](https://github.com/GloriousEggroll/proton-ge-custom/releases/tag/GE-Proton11-7), published September 16, 2026 |
| UMU | [1.4.4 Debian 13 ARM64 package](https://github.com/Open-Wine-Components/umu-launcher/releases/tag/1.4.4) |
| Steam Linux Runtime | `steamrt4-arm64`, `4.0.20260914.260627` |
| pressure-vessel | `0.20260914.0` |
| Windows probe | Existing `rpi0-windows-probe.exe`, SHA256 `86161880adb171519bfa1b263ea136c6f40dd595097891fb5436d256cf727866` |
| Windows host | Existing `wf0-factory-probe.exe`, SHA256 `6f00cf377458037e4ae64c2425578b7c408a0a5ed6f027a0249e0d979f55fc2c` |

The GE archive and launcher package matched their GitHub release SHA256
digests. UMU reported a successful runtime archive SHA256 check and runtime
mtree verification. Package hashes, sanitized logs, live process topology,
launch settings, and results are retained in
[`native-arm-runtime-observations.json`](../../evidence/rpi2/native-arm-runtime-observations.json).

## Attempts and remaining startup faults

1. `ge-hello-01`: operator-agent launch error: the required `--self-test`
   argument was omitted. Exit 64, no pass marker. The 128.690-second service
   duration included downloading the runtime and creating the prefix.
   Optional Xalia game-accessibility startup also failed with “No displays
   available.” This attempt is preserved, not counted as a pass.
2. `ge-hello-02`: correct argument, Xalia disabled. Exit 0 in 7.592 seconds,
   but no marker in captured output. The Unix executable path selected
   GE-Proton's `umu.exe` launch route. Exit success alone was insufficient.
3. `ge-hello-03`: same probe copied inside the fresh prefix and launched as
   `C:\bridge\probes\rpi0-windows-probe.exe --self-test`. Exit 0 in 7.068
   seconds and exact marker
   `RPI0_X86_64_WINDOWS_PREFLIGHT_PASS 5250493057494e36` captured.
4. `ge-host-01`: the existing host's `--environment-owner` mode, with its
   actual executable hash, wrote the matching `environment.ready` token.
   A live process snapshot confirmed native ARM execution and FEX mappings.
   After writing `environment.stop`, the cohort exited 0 and retired.
   Its 21.376-second duration includes deliberate observation time; it is
   not a startup benchmark.

Successful launches still report locale generation, a missing destination
for optional `igdext64.dll` copying, and an empty game-drive-parent error.
UMU also warns that a Windows path is not a local Unix executable before
passing it to Proton successfully. These messages remain in the logs. No
claim of a fault-free runtime is made. `PROTON_USE_XALIA=0` disables the
unneeded game-accessibility helper; it does not disable the host's Windows
message loop.

Spot temperatures during setup reached 61.5°C; the final reading was 54.3°C.
The final throttle register was `0xe0000`: historical flags remained, with
current bits clear. This short launch work is not a thermal or DSP test.

## Continuing from the staged environment

The private directory retains a small `run-ge.sh` wrapper, downloaded
packages, extracted launcher/dependencies, pinned runner/runtime, and fresh
prefix. The wrapper uses an exclusive lock, a new log/unit name, a 90-second
systemd lifetime, a 3 GiB memory bound, and cleanup confined to its cgroup.
It is a fixture helper, not a product runner implementation.

For another account-free launch, use a new label and the existing Windows
path. For example, from that private directory:

```sh
./run-ge.sh ge-hello-new 'C:\bridge\probes\rpi0-windows-probe.exe' --self-test
```

That initial launch-only boundary has been extended by the reference bridge
run above. Its configuration replaces the legacy runner fields with
`runner=native`, `launcher_path` / `launcher_sha256`, and `leader_path` /
`leader_sha256`. The other RPI0 configuration fields remain unchanged. The
staged launcher is distinct from the earlier `run-ge.sh` process-only helper;
the Rust supervisor owns its systemd unit. Use a new evidence destination for
each subsequent session and hold the private `run.lock` exclusively.

Pigments now has operator-confirmed factory preset playback and one successful
preset/master recall across process restart. Short editor-unopened playback of
the restored preset also succeeded. The editor-open gap, startup gaps, sustained
throughput, broader recall and ShieldXL controls remain unresolved. Resume load
testing with the active heatsink using the comparison described above. Hangover
remains an alternative; it was not installed or executed.

While waiting for active cooling, the operator selected installation groundwork
and short functional checks for Efx FRAGMENTS, then Analog Lab Pro. REAPER is
deferred for a later DAW demo. The checked packages, concrete standalone-host
adaptations and execution order are in
[Arturia expansion groundwork](RPI2_ARTURIA_EXPANSION.md). This preparation does
not add an installation or audio result for either new product.

## Reassembly and PiSugar power baseline

On September 23 the operator confirmed a passive aluminium CPU heatsink,
reconnected ShieldXL, and a PiSugar 3 Plus. The last reported supply condition
was charger-connected; that condition was not independently verified during
the first idle readings. After a fresh boot, idle readings were 46.1–48.5°C, all four cores
were online, and the throttle register remained `0x0` (including historical
bits for this boot). Two PMIC input-voltage readings were 5.04108 and 5.04242 V.
PiSugar monitoring software was absent; battery charge, total power, battery-only
runtime and supply behavior under audio load were not measured.

The planned short Pigments note test stopped before launch: ALSA listed only
the two HDMI devices, and JACK was absent. The ShieldXL boot configuration was
still included and its `snd_soc_cs4270` driver loaded, but the kernel reported
four I²C "lost arbitration" messages, `cs4270 1-0048: failed to read i2c at
addr 48`, and a deferred sound-card probe. The JACK service repeatedly failed
its mixer setup because the card was unavailable. SSH subsequently became
unreachable during read-only checks. This is an unresolved hardware/bus bring-up
failure; neither PiSugar causation nor a spontaneous power loss is established.
No plug-in ran and no driver, power policy or boot configuration was changed.

After the operator resolved connection issues and rebooted, ShieldXL and JACK
were available again without an agent-side driver or boot configuration change.
Idle temperature was 31.8°C, input voltage 5.04376 V, and throttle flags `0x0`.
The first launch (`pigments-power-01`) refused the changed X-authority fingerprint
before loading Pigments. The current, user-owned desktop session file was verified
and only its pinned fingerprint in the private appliance configuration refreshed;
the previous configuration was retained privately. No binary was rebuilt.

The retry (`pigments-power-02`) restored the previously saved 365,534-byte
Altered Alter Boy state, set master to 0.45 (readback 0.44999998807907104), and
captured five seconds of finite, nonzero stereo audio with the editor unopened.
Three MIDI events were delivered with no errors. Before/after counters remained
at one gap / 3,584 missing frames, so the note-fixture interval added no gaps;
the pre-existing startup gap remains. JACK xruns, process failures and callback
deadline misses stayed at zero. The observer's 27 samples ranged from
35.85–41.35°C and 5.01562–5.05046 V, with throttle flags always `0x0`.

**The operator subsequently confirmed this successful test was strictly battery
powered, with the charger disconnected.** Supply-mode provenance is the operator's
physical observation; voltage and throttle readings alone do not distinguish
battery from charger power. The disconnect time was not logged. This establishes
one short real-plug-in battery-powered run, not battery endurance, a measured
power budget, sustained capacity, or charger switchover behavior.

The session reached readiness after 40.16 seconds and exited cleanly after
62.30 seconds, following an intentional quit after the capture. The Pi remained
reachable, its audio server remained running, and no experiment units or session
directories remained. Private `power-note.json`, `run.json`, `note.log` and the
audio capture retain the underlying observations. The earlier boot's I²C fault
has not been causally explained by this successful retry.

## Efx FRAGMENTS stereo processing

The same ARM Wine/FEX environment now runs Efx FRAGMENTS 1.3.1.6566 through the
existing Windows VST3 host, with native stereo audio input added to the Pi test
application. Digital input/output comparison, one setting restored across a
process restart, and user-confirmed physical stereo processing are recorded in
[the Arturia expansion result](RPI2_ARTURIA_EXPANSION.md). The existing Pigments
executable and its earlier observations remain separate.

## Clear Skies stress and manual USB MIDI setup

The operator selected Clear Skies (reported Poly 8) and lowered its master to
0.36433076858520508. A fresh headless process restored that state without changing
the volume. A 30-second chord ladder was followed by a 120-second generator of
repeated eight-key chords, using the unchanged Pigments and bridge binaries on
the fan-cooled, battery-powered Pi. The generator's six local tests passed and
its separate native build took 4.662 seconds.

**The operator heard fuzz and stopped the demo. Sound quality is unresolved.**
The final observer status request timed out; the last complete stress snapshot
was at 108.134 seconds and showed no added gaps relative to its startup baseline.
The generator completed its recording, which does not prove uninterrupted host
processing. Peak sampled temperature was 49.05°C with no throttle flags. Neither
these counters nor sample peaks below full scale explain the audible fuzz.
Sanitized observations are in
`evidence/rpi2/pigments-clear-skies-stress.json`; audio and vendor state stay private.
No 16-note demo was prepared or run.

For subsequent hands-on work, USB enumerated the controller as `omx-27` and its
JACK MIDI capture port was connected to Pigments. The editor opened, MIDI events
were accepted, and both playback connections were explicitly recreated. The
operator reported no sound; audible operation was not confirmed before a power
cycle. On the next boot, the ShieldXL card enumerated but JACK reported repeated
ALSA polling timeouts and could not accept clients. Its service restart required
local administrator authentication. This is a separate unresolved audio-service
failure, not evidence that MIDI routing or vendor authorization failed.

A candidate session-helper change attempted an explicit longer manual lifetime.
It passed syntax validation but failed live: this systemd build rejects changing
`RuntimeMaxUSec` on an active service. The helper consequently retired the first
reopening attempt before editor attachment. A short isolated service check
reproduced the rejection; a runtime drop-in/reload also failed to extend an
already armed timer. Both temporary check services were removed. The candidate
was reverted to the previously working helper and its 280-second session limit.
A longer manual session remains unimplemented; the failed attempt is retained.

On a later connection, ShieldXL and JACK again accepted clients before Pigments
was started. This recovery does not establish what caused the preceding DMA
stall. No driver, overlay or audio-buffer configuration was changed by the agent.

## Aioliane two-note editor comparison

The operator subsequently reported that Aioliane rose from about 21% to 100% on
Pigments' CPU meter and stopped sounding with two keys held down. Its private
state was captured. A fresh process compared the same saved state, master
0.58833080530166626, and C4/G4 at velocity 96 in three 20-second recordings:
editor never opened, editor open, then editor closed again. Notes were held for
12 seconds per pass. No external controller was connected to the plugin during
these automated intervals, and plugin/host binaries were unchanged.

| Condition | Average total Pi CPU | Added missing frames | Temperature during capture |
| --- | ---: | ---: | ---: |
| Editor never opened | 16.66% | 512 | 47.95–50.70°C |
| Editor open | 26.37% | 1,536 | 49.60–52.35°C |
| Editor closed again | 12.45% | 0 | 49.60–51.80°C |

These CPU figures are averages across all four cores over each trial, including
setup/release/retirement, not Pigments' meter or a peak audio-thread budget.
The corresponding Windows-cohort averages were 0.596, 0.839 and 0.546 occupied
CPU-core equivalents. None of the captures contained exact stereo silence
between seconds 3 and 14, during the sustained-note portion. Small transport
gaps therefore do not reproduce the operator's prolonged two-note cutoff.
The full session peaked at 54°C with no throttle flags and shut down cleanly,
restoring the audio graph. State restore and editor transitions occurred outside
the measured note intervals; their cumulative counters remain retained.

The open editor increased measured load in this ordered comparison, and closing
it reduced load. This does not establish the cause of the earlier degraded
session. Original manual pitches/velocities were not recorded, and preset/menu
history was not replayed. The failure remains open. The new two-note fixture
passed seven tests and built on the Pi in 3.422 seconds. Full sanitized results
are in `evidence/rpi2/pigments-aioliane-editor-comparison.json`; recordings and
vendor state remain private.

A subsequent fresh headless Aioliane pass added E4/B4 to the preceding C4/G4
stimulus, retaining velocity 96, the saved master level, 12-second hold and
20-second capture. All nine MIDI events were accepted. It added zero gaps,
missing frames, processing failures or JACK xruns, and the captured sustained
portion contained no exact stereo silence. Average total-Pi CPU was 22.36%;
temperature peaked at 51.8°C without throttle flags. Shutdown was clean and
restored the graph. The extended fixture passed eight tests and built in
0.895 seconds. This one short hold does not establish sustained capacity or
explain the earlier manual failure. See
`evidence/rpi2/pigments-aioliane-four-notes.json` for measurements and listening status.

The operator subsequently confirmed that this four-note headless Aioliane pass
sounded exceptionally clean. This closes listening confirmation for that exact
12-second hold, without extending the claim to other presets or longer runs.

### Pigments Previous/Next preset controls

The operator confirmed that Pigments' MIDI panel assigns CC28 to Previous
Preset and CC29 to Next Preset, with Learn disabled. The exact retained
`IMidiMapping` results translate these to writable helpers 2394/2395; the host
does not write the read-only preset-action parameters. Ordinary JACK CC input
remains unsupported. The test uses the existing processor/controller parameter
path and a separate private binding for the same Pigments module and class.

In `preset-learn-01`, one scripted Next pulse changed the visible preset to
the operator-reported “electric swings.” After an acknowledged editor close,
a Previous pulse changed state; reopening showed “electric mich.” The
session closed cleanly and peaked at 53.45 C without throttle flags. These
observations establish working vendor navigation in this ARM runtime; they
do not establish that every pulse advances exactly one adjacent preset.

The ShieldXL panel now supports optional `mode: "momentary"` buttons; omitted
mode retains the existing toggle behavior used for Fragments Freeze. The
control-thread model retains press/release edges in a bounded queue, spaces
updates, waits for controller readback and reports timeout/overflow. Button-only
surfaces are permitted. Existing save and toggle tests remain green; all 25
library tests passed. The native candidate built on the Pi in 14.33 seconds.
The previous generic binary was preserved and restored; the navigation candidate
and its binding remain separately named in private staging.

In the fresh `preset-buttons-01` session, a scripted Next pulse changed saved
state before the editor was opened. The operator then tested button 2
(Previous) and button 3 (Next), and reported needing two presses for a change.
The operator accepted that behavior for now. **One-press navigation remains
unqualified**; a sweep must not count button messages as confirmed preset changes.
The panel's OLED connection reported unavailable in this session; successful
display behavior is not claimed. No note-hold or preset sweep was run here.
All 13 observed physical presses produced helper value 1 and all 13 releases
produced 0 (three Previous clicks, ten Next clicks). The two-press behavior
therefore cannot be attributed simply to this host alternating 1/0 on successive
presses. The private MIDI assignment records have no toggle/momentary field;
Pigments' action interpretation and delivery timing remain unverified causes.
The final state was captured privately; shutdown was clean. This session peaked
at 54.0 C with no throttle flags.
State and MIDI configuration remain private. Sanitized results are retained in
`evidence/rpi2/pigments-preset-navigation.json`.

Deferred separately: compare Pigments' own CPU meter with whole-Pi and per-thread
measurements, and establish the graphics-rendering path and its load. The
navigation result does not resolve the earlier performance degradation.

### Five verified selections, headless four-note sweep

`preset-sweep-01` completed five successive observed Next selections, with no
editor opened and no physical MIDI connected. Each trial used C4/E4/G4/B4,
velocity 96, held from seconds 2–14 in a 20-second capture. Master parameter 0
was set to 0.35 and read back after each preset load. The note generator and
generic native host binaries were reused. This is four held keys; the preset's
own voice limit, sequencing, unison and envelopes were not changed.

| Selected preset | Whole-Pi CPU, trial average | Missing frames added | Gaps added | Peak temperature |
| --- | ---: | ---: | ---: | ---: |
| 24 AM | 35.79% | 481,536 (10.032 s) | 91 | 52.90 C |
| 3030 Bassline | 8.21% | 0 | 0 | 50.15 C |
| 3k | 9.54% | 0 | 0 | 50.15 C |
| 5th Sweep | 23.85% | 0 | 0 | 52.35 C |
| 7AM | 22.13% | 0 | 0 | 52.35 C |

All five received the nine planned MIDI events. Processing-failure and JACK
xrun counters added zero even during 24 AM's substantial bridge loss. No
recording had nonfinite samples or samples reaching full scale; the largest
peak was 0.376548. CPU figures average all four cores across the whole trial,
not exclusively the hold and not Pigments' own meter.

24 AM's first nonzero recorded sample arrived at 11.008 seconds, roughly nine
seconds after note-on. This result does not distinguish expensive first-note
initialization from sustained processing overload. A warm repeat in the same
headless session is a useful next comparison before attributing this to graphics
or steady-state DSP capacity. 3k produced audio from approximately 2.055–5.851
seconds and then silence, with no added transport loss. Its decreasing recorded
level does not establish why it stops sounding; silence is not automatically a
bridge failure. The operator said the sounds heard sounded great, while being
uncertain whether 24 AM and 3k were audible. Do not describe all five as audibly
confirmed passes.

Preset names and banks were read from the component and controller portions of
private saved state using a strictly bounded parser for this exact Pigments
serialization. The parser validates the bridge envelope identity, sizes and
checksum and requires matching component/controller labels. It matched the
previous operator-confirmed Electric Swings/Electric Mich states, refused six
damaged or mismatched real-state variants, and passed five synthetic unit tests.
It is experiment instrumentation, not a stable Arturia preset API or a durable
content identity. Raw state and audio remain private.

Restoring the prior 8-Bit Crystals state did not restore the browser's Next
position: the first observed Next selected 24 AM. That selection established
the sweep's starting point. Every subsequent pulse was checked after both
edges and again after settling, with unchanged/duplicate selections refused.
This observes successive selections; it is not an independent complete preset
index or proof that no unobserved library entry was skipped. Initialization
navigation added 8,704 missing frames in two gaps; the four later navigation
intervals added none. These are separate from the note-trial numbers above.

The complete session lasted 210.52 seconds, peaked at 53.45 C and reported no
throttle flags. The starting state and level were restored and the session
closed cleanly with the original JACK graph. Power-source mode was not
reverified. Detailed results are in
`evidence/rpi2/pigments-five-preset-sweep.json`; the bounded observer and identity
reader are in `rpi2/pigments_preset_sweep.py` and
`rpi2/pigments_preset_identity.py`.

### 24 AM first hold versus same-process repeat

A fresh headless session restored the exact 24 AM state saved before the sweep's
first trial, including master 0.35. It played the same C4/E4/G4/B4 chord twice
(velocity 96, 12-second hold in each 20-second recording), with five seconds
between trials and no preset reload or editor between them.

| Trial | Missing frames added | Gaps added | Average whole-Pi CPU | Peak temperature |
| --- | ---: | ---: | ---: | ---: |
| First hold | 645,376 (13.445 s) | 1 | 39.29% | 52.35 C |
| Same-process repeat | 0 | 0 | 28.64% | 52.90 C |

Both accepted all nine MIDI messages and added zero processing failures or JACK
xruns. Neither reported throttling or recorded nonfinite/full-scale samples.
The first hold was exactly silent throughout the sampled seconds 3–14 and
produced its first tiny nonzero output at 15.499 seconds, after note-off. The
repeat contained no exact stereo silence in that same held window and peaked at
0.0477053. Residual pre-note audio means the repeat's first-nonzero timestamp is
not a response-latency measurement.

Only after both trials did the host open the editor. The operator confirmed
**Poly 4** and heard the sound, describing it as very quiet and characteristic
of the preset. The test's deliberately reduced master also affects loudness.
Do not classify its sound character as failure, or turn that listening judgment
into a denial of the separately measured first-trial transport loss.

The saved state after both trials still identified 24 AM in both component and
controller metadata. This pair supports first-use/transient work as a lead and
shows the same configured four-key workload can run without added transport
loss after the first play. It does not identify JIT translation, sample loading,
scheduling, or a particular plug-in subsystem as the cause. This was a fresh
process, not a cold boot or flushed cache; it restored state rather than using
the original sweep's browser navigation. The operator's prior starting state
was restored before shutdown. See
`evidence/rpi2/pigments-24am-first-repeat.json` for retained measurements.

### 24 AM warm-up and editor closed/open/closed comparison

`24am-editor-01` restored the same Poly 4 state once at master 0.35, ran a
four-key warm-up, then repeated the same 12-second hold with the editor closed,
open, and closed again. There was no intervening preset restore. The previous
clean warm repeat did **not** reproduce: every recorded seconds 3–14 held window
was exactly silent, including both headless passes. Warming up is not an
established remedy, and this sequence did not provide a clean audio baseline.

| Condition | Average whole-Pi CPU | Audio worker, one-core CPU | Missing frames added | Peak temperature |
| --- | ---: | ---: | ---: | ---: |
| Warm-up | 39.37% | 85.26% | 697,088 | 53.45 C |
| Closed before | 39.84% | 83.47% | 622,848 | 54.00 C |
| Editor open | 55.90% | 94.82% | 1,027,840 | 56.75 C |
| Closed after | 36.16% | 84.93% | 790,016 | 56.20 C |

Each pass accepted nine MIDI events and reported zero processing failures and
JACK xruns, despite substantial missing output. The host remained alive and its
processed-block counter advanced. All recordings were finite and below full
scale. Status intervals include observer setup/teardown around each 20-second
capture, so their missing-frame totals are not exact silence durations inside
the recordings. Closing the editor does not reset accumulated work; the final
pass can carry backlog from the open pass. Request-queue high water rose from
117 to 1,163 during the open trial. These are queue entries, not audio frames.

The operator saw approximately 65% on Pigments' meter at rest and over 100%
with cutoff during the **automated four-note chord**, clarifying that this was
not a separate one-note test. The meter's denominator remains unestablished.
The `lvb-audio` thread is the Windows worker that calls the VST3 processor, but
its CPU time also includes bridge work; these observations do not isolate DSP
from translation, polling, copying, or synchronization costs.

Read-only process inspection found OpenGL/GLX Mesa libraries and a V3D render
device in the Windows host after opening the editor. Its GPU render counter
advanced by 8.414 seconds around the open trial and 0.371 seconds around the
final closed trial. This establishes actual host GPU activity, beyond the desktop's
accelerated V3D capability. It does not establish that every part of the editor
is GPU-rendered. Xtigervnc separately consumed 23.85% of one core during the
open trial, versus 0% before and 0.63% after. The open condition includes both
editor and remote-desktop costs.

The 175.39-second session peaked at 57.3 C with no throttle flags, restored
8-Bit Crystals and the exact prior master value, and shut down cleanly with the
baseline JACK graph. No binary changed. Power source was not reverified. The
next useful investigation is time spent inside processing versus the worker's
other work and queue recovery, rather than attributing this run to heat or
graphics alone. See `evidence/rpi2/pigments-24am-editor-comparison.json`.

### Governor comparison: preflight dependency

The separate CPU-efficiency slice selected a warmed headless
ondemand/performance/ondemand comparison of unchanged 24 AM. Live preflight
found ondemand with a 1.5–2.4 GHz range, 45.5 C and no throttle flags. The native
host and four-note fixture hashes matched the retained campaign. No owned audio
experiment or plugin JACK ports/connections were present.

The governor and scheduler-statistics controls are root-owned and not writable
by the SSH account; non-interactive sudo requested a password. Scheduler
statistics are disabled, so waiting time is unavailable, not zero. The
experiment stopped before launch without changing system settings, plugin state,
or binaries. It has **no governor-performance result**. Continuing requires an
operator-authenticated temporary way to set and restore the governor; scheduler
statistics are optional if enabling them is unavailable. See
`evidence/rpi2/pigments-governor-preflight.json`.

The reviewed `rpi2/governor_guard.py` now provides the narrowly scoped temporary
mechanism, pending operator authentication. It runs in the foreground, permits
only the fixed governor sequence through a transient operator-owned FIFO, and
does not execute benchmark code or requested commands as root. It captures and
restores both original settings on completion, handled signals, errors or a
15-minute timeout. SIGKILL and power loss are outside that guarantee. Final
status distinguishes successful restoration from completed phase transitions;
interruption or invalid requests return nonzero even after successful cleanup.
Four focused tests passed. The guard's staged SHA256 is
`83f2d5d77608540e7a51ff468006dd81a550661cdc0b3c2c4b7438759b9c09fd`.
No governor comparison has run yet.

Attempt 01 subsequently authenticated the guard and completed one ondemand
warm-up, then stopped before measurement when the live phase-tail reader
misparsed a partially appended record. The complete retained stream has 91,775
valid records in 33,422,937 bytes; no durable corruption was found. The exact
transient failing fragment was not retained. A focused reproduction shows how
the former iterator could skip a partial prefix and parse its later suffix.
The repaired reader freezes the byte extent, discards incomplete edge records,
and continues to reject malformed complete records. Its retained-tail check
reported 21.21 ms maximum queue wait and therefore still failed the unchanged
20 ms drain threshold.

The warm-up accepted nine MIDI events but added 643,072 missing frames, with
exact silence throughout recorded seconds 3–14 despite fixture exit zero.
This is not an audio pass or a governor comparison. The 69.37-second session
peaked at 51.25 C with no throttle flags and restored the original preset,
exact master and JACK graph. The guard restored ondemand/schedstats 0 and
returned exit 2, distinguishing aborted work from successful cleanup. Original
failed evidence remains in `evidence/rpi2/pigments-governor-attempt-01.json`.

The retry adapter now uses validated unique attempt labels, requires recent
worker observations as well as low queue wait, and requests the same explicit
phase flushes before/after each capture and every five seconds during it. The
offline reducer joins callback sequence/bridge position and verifies request
IDs, separates conservative queued note-active/idle/release cohorts, reports
nominal versus retained/completed block coverage and uses completed frames as
the CPU-work denominator. Missing coverage cannot be counted as an improvement.
CPU samples span each cohort's actual execution interval, including runtime
work outside vendor calls; detailed caller CPU remains unavailable. The
recorder's formatting and disk synchronization are observer costs. Thirteen
focused tests passed, including nine reader/reducer tests on the Pi. Retry is
pending coordinator review and renewed operator authentication.

### Governor attempt 02: recorder delivery prevented admission

The reviewed retry completed one ondemand warm-up, then refused all six drain
checks because retained worker observations were 3.55–4.06 seconds old. Every
check nevertheless had 188 matched blocks, zero unmatched requests and only
3.59–3.67 ms maximum queue wait. This is not evidence of persistent live
backlog. No measured hold, performance condition or second ondemand condition
ran. The 99.64-second session restored 8-Bit Crystals, master
0.48033079504966736 and the baseline JACK graph. The guard restored ondemand
and scheduler statistics 0, removed its request FIFO and returned cancellation
exit 2. Peak temperature was 52.35 C with no throttle flags.

Offline inspection of attempt 02 locates the freshness failure inside a
partially emitted flush. For example, at byte 32,290,630 the selected worker
record was 2.928 seconds older than its preceding marker; the same flush later
contains a worker record only 12.85 ms after that marker. Other checks likewise
sampled older rows partway through a flush containing current records.
`worker_request_observed` uses raw monotonic time, equal to `recorded_ns`; it
does not use converted worker-call timestamps. Converted process timestamps
remain correctly written by the phase writer. Mark acknowledgment means queued,
not flushed. Callback and worker rings drain in batches, and history is emitted
in insertion order. No flush-completion timestamp exists in this record.

The recorder formats JSON directly into unbuffered `File` with `writeln!`, then
syncs after the batch. Multiple formatting writes and serialized batch emission
are source-visible mechanisms; actual syscall count and time spent writing,
waiting for storage or being descheduled were not measured. The retained data
proves incomplete delivery at the reader, not the exclusive cause of its cost.
Existing live status already exposes callback count, paused frames and completed
blocks. With this fixed 512/256 quantum and zero failures,
`callbacks*512 - paused_frames - bridge_processed*256` approximates outstanding
frames, subject to separately sampled counters. It was 512 before warm-up and
256 afterward. The atomic native `ap12.status` lane also exposes current epoch,
request position/stage/time and sampled submitted-frame totals. Neither the
printed queue high-water mark nor the single delivery-mailbox slot is current
native queue depth.

Useful warm-up measurements survive this observer failure. All 3,750 published
blocks in the 20-second stimulus matched completions with no reported phase
drops. Conservative queued seconds 3–13 contained 1,874 contiguous completed
blocks (479,744 frames). Vendor wall mean/p95 were 5.382/6.527 ms; total service
mean/p95 were 5.544/6.721 ms against the 5.333 ms quantum. Respectively 849 and
951 blocks exceeded that quantum. Across the cohort's actual 10.415-second
execution span, the audio worker used 10.167 CPU seconds and accumulated 0.0486
seconds of scheduler wait. Windows-cohort CPU was 19.011 seconds, approximately
1.902 CPU seconds per rendered second. Nineteen active-span frequency samples
were approximately 2.400 GHz. These interpolated CPU spans include runtime work
outside vendor calls; they are not pure DSP or energy measurements. The
recording was finite but exactly silent throughout held seconds 3–14, despite
fixture exit zero and nine accepted MIDI events.

The CPU-efficiency objective remains unachieved. Recommended next scope is a
small control-plane recorder buffering change with explicit flush before sync
and an overhead check, while using existing live status for drain. This would
address the observer confound before another governor comparison. No recorder
or product binary was changed, and no third authentication was requested.
See `evidence/rpi2/pigments-governor-attempt-02.json` for retained findings.

### Buffered recorder prerequisite: one same-ondemand original/candidate pair

The coordinator authorized buffering on the ordinary recorder thread, keeping
schema, serialization, incident/marker/finish boundaries and error behavior.
`phase_capture.rs` now uses a 64 KiB `BufWriter` and explicitly flushes before
the underlying `sync_data`. Nothing changes in the callback, DSP, routing or
latency. A counting-writer test produced the same 300 records/95,824 bytes with
31,800 direct underlying writes versus two buffered writes. Separate tests
exercise write failure, flush failure preventing sync, and propagated sync
failure. Focused Pi release-test compilation took 23.80 seconds; the native
candidate release build took 21.66 seconds. No Windows/runtime build occurred.

The original binary remains the default. Candidate SHA256
`6c3cc393837dc81168a2c7bdb7fc33e4f469ba80659512f4f91d9cfec392e876`
is staged separately from original
`9d0a611d7175b05e398f484d9217fb29b7004f3e0eaac76acd6ad00a541bd1da`.
Only the recorder source was changed in the installed source tree. Its existing
older `panel.rs` differs from this PR's base and was deliberately preserved.
A 58-file source/manifest/lock hash inventory is retained. Exact original
binary-to-source correspondence was not proved by a reproducible rebuild;
this limits attribution to a sole binary difference. The final seconds of
candidate building overlapped initial baseline launch, ending before readiness,
warm-up and measurement. This startup sequencing limitation is retained.

Both fresh sessions restored the same 24 AM state and ran the same warm-up and
measured four-note hold, editor closed, ondemand unchanged and scheduler
statistics disabled. Mark requests before/after capture and every five seconds
were identical. Three stable small live outstanding-frame estimates guided
admission; those separately read counters are not an atomic queue bound.
Offline pre-stimulus traces independently matched all 188 expected blocks,
with maximum queue waits 7.398 ms original / 6.501 ms buffered. Both measured
conservative note-active windows matched 1,876 contiguous blocks / 480,256
frames, with no phase drops or position holes.

| Active-cohort measurement | Original recorder | Buffered recorder |
| --- | ---: | ---: |
| Recorder CPU seconds | 1.8794 | 0.1104 |
| Recorder CPU, one core | 18.79% | 1.11% |
| Native-process write calls | 3,128,974 | 168 |
| Native-process bytes written (`wchar`) | 10,791,131 | 10,784,737 |
| Windows-cohort CPU seconds | 14.4632 | 14.4516 |
| Windows CPU seconds per rendered second | 1.4456 | 1.4444 |
| Vendor mean / p95 wall ms | 4.909 / 5.616 | 4.897 / 5.609 |
| Service mean / p95 wall ms | 5.065 / 5.791 | 5.051 / 5.770 |
| Whole-Pi CPU | 43.15% | 39.92% |

Recorder CPU fell approximately 94.1% in this matched pair. Process write counts
include all native writes, but nearly all disappear while byte volume remains
similar. At about 0.5 seconds after capture-time marks, original complete-record
ages were approximately 1.5–2.9 seconds versus approximately 0.49–0.52 seconds
with buffering. Overall age still grows between deliberate five-second marks;
the recorder is not a synchronous live-status interface. Windows processing
time and work barely changed, so this is a native observer-efficiency result,
not an improvement in Pigments DSP capacity or measured energy.

**Audio regression possibility remains open.** The original measured hold
added zero missing frames/gaps. The candidate added 6,144 frames in 24 gaps,
each 256 frames. All occurred 2.556–2.887 seconds after graph-ready, during the
held-note attack, not outside the capture. Exact-zero stereo frames in the
full seconds 2–14 window were zero original versus 6,144 candidate. The later
seconds 3–14 subwindow had no exact silence in either, but excludes these gaps
and cannot be described as a full-hold pass. Both recordings were finite and
below full scale (peaks 0.04818/0.05180). Both accepted nine MIDI events with
zero processing failures/JACK xruns. No operator audible judgment or rerun was
requested. The single ordered pair cannot attribute the extra gaps to buffering
versus run variability; it provides no audio-reliability win.

Both sessions shut down cleanly and restored 8-Bit Crystals, exact master
0.48033079504966736 and baseline JACK graph. Governor remained ondemand,
scheduler statistics remained 0, no owned experiment stayed running, and the
default original binary hash was verified. Temperatures peaked at 54.0 C in
session samples; denser trial samples reached 55.1 C in the candidate, with no
throttle flags. Current power source and energy were not measured.

Governor A/B/A remains explicitly deferred, not achieved. Review the recorder
repair as an opt-in prerequisite, retaining the active-attack gap caveat, then
target Windows processing work whose cost remained almost unchanged. Detailed
results and provenance: `evidence/rpi2/pigments-recorder-buffering.json`.

### Same-binary native phase trace OFF/ON/OFF

The follow-on candidate reads `LVB_RPI1_PHASE_TRACE` once during native startup.
Absent or `off` disables native phase tracing, `on` enables it, and invalid values
fail startup. OFF skips phase event clocks/production, worker clock conversion,
the phase-only JACK cycle/gettid block, recorder thread and phase file. Startup
reports mode/recorder/file; `mark` explicitly reports unavailable when disabled.
ON retains the buffered recorder and existing record format/error behavior.

This is **not a fully uninstrumented runtime**. Callback deadline clocks and
scalar output/nonfinite/missing-frame counters remain, as do request/transport
and fault timestamps, the commercial `ap7-observer`, `ap3-transport`, and Windows
host diagnostics. Neither DSP settings nor the synchronization backend changed.
Six focused native tests passed, including disabled no clock/event/file/thread,
actual enabled event/marker output, byte identity and write/flush/sync errors.
Test compilation took 12.54 and 23.19 seconds; incremental native build took
59.33 seconds and ended before the first session. The separately staged candidate
SHA256 is `85780f4856f2bac9f605628ca7decd80568cca0ad36fa92db70960bf74dd1a62`.
Preserved generic binary SHA256 `9d0a611d7175b05e398f484d9217fb29b7004f3e0eaac76acd6ad00a541bd1da`
was restored before launch. The 58-file source inventory matches this change
except the intentionally preserved installed `panel.rs` difference.

The reviewed experiment uses one identical candidate for OFF1, ON and OFF2,
with the same preset, warm-up, near-drained admission and measured four-note hold.
Common low-rate CPU/frequency/temperature/native I/O sampling and before/after
status remain identical. ON alone has the recorder and before/after/five-second
marks with durable sync. No phase-file freshness test is used. OFF has no phase
attribution; CPU comparisons use common wall windows, not an invented completed
block denominator. Counter intervals are wider than the hold. Graph-ready stdout
observation approximates wall alignment; capture frame 96,000 schedules note-ons
and 672,000 note-offs, and output onset/zeros are separately measured. Initial
zeros, internal zero runs and reported missing frames are not conflated.

| Measured observation | OFF1 | ON | OFF2 |
| --- | ---: | ---: | ---: |
| Whole held 2–14 s Windows CPU, one core | 145.57% | 146.83% | 146.26% |
| Stable 3–14 s Windows CPU, one core | 145.15% | 146.36% | 145.22% |
| Phase recorder CPU during whole hold, one core | absent | 0.94% | absent |
| Whole-Pi CPU during whole hold | 38.92% | 37.73% | 37.46% |
| Status missing-frame / gap delta | 0 / 0 | 0 / 0 | 1,024 / 4 |
| Exact-zero output frames, 2–14 s | 0 | 0 | 1,024 |
| Exact-zero output frames, 3–14 s | 0 | 0 | 0 |

OFF2's four 256-frame zero runs begin at capture seconds 2.640, 2.6613, 2.672
and 2.704, during the attack. OFF repeats therefore expose audio variation even
without native phase tracing. This is not evidence that tracing improves audio,
or that disabling it cures earlier gaps. The three full recordings were finite,
with peaks 0.04413 / 0.05051 / 0.04465. Residual tails were already nonzero before
the measured stimulus, so first-nonzero-at-second-2 is not new-note onset.
All measured passes accepted nine MIDI events with no process faults, callback
deadline misses or JACK xruns. The ON phase record covered all 2,250 held-window
blocks and 2,062 stable-window blocks without position holes or phase drops.
ON full-hold vendor/service means were 4.977/5.130 ms; stable means 4.954/5.107 ms.

Warm-ups remain severe failures. OFF1 and ON captured exact silence throughout
held seconds 2–14. Their wider status intervals added 644,096 and 639,488 missing
frames. OFF2 added 313,856 missing frames; first held-window nonzero output was
at capture second 8.5547, preceded by 314,624 zero frames and followed by seven
more 256-frame internal zero runs. These are observations, not ordinary onset
latency. Disabling native phase tracing does not remove this startup problem.

The Windows audio-calling thread consumed approximately 94% of a core and a
second thread named `Processing Thre` approximately 44% during the measured
hold. These thread names/CPU observations do not isolate DSP from translation
or synchronization, but contradict a blanket claim that the workload uses only
one thread. Native transport used about 3.7% and the retained commercial observer
about 1% of a core in all modes. The ON native process made 170 writes during the
whole hold; OFF modes made none in that interval. The small recorder cost is
measurable, but whole-Pi variation prevents a firm total-system savings claim.

Temperature reached at most 54.0 C in trial samples, with no throttle flags.
Most active frequency samples were approximately 2.4 GHz. OFF2 included one
1.8 GHz sample at graph-observation +3.553 seconds, later than its captured gaps;
that sparse and approximate alignment does not establish causality. No energy
or battery-life conclusion follows. All three sessions restored 8-Bit Crystals,
master 0.48033079504966736 and baseline JACK graph, then shut down cleanly.
No owned service remained; ondemand/schedstats=0 and the preserved generic
`lvb-arm-plugin-standalone` hash were verified. The separate older
`lvb-arm-pigments-standalone` executable was not modified.

This baseline supports an exploratory **large-effect warmed multicore setting
comparison**, after verifying the actual plug-in setting. It does not support
small-effect audio reliability claims, cold-start usability or generic core
scaling promises. Retain the warm-up failures and attack interval in that next
comparison. No extra runs, governor change, GUI action or default replacement
were performed here. Results: `evidence/rpi2/pigments-phase-trace-comparison.json`.

Path clarification: the preserved generic build output is
`source-bridge/rpi0/standalone/target/release/lvb-arm-plugin-standalone` (9d0a611...).
The session helper's actual CLI default is the separate Pigments-specific binary
in that same release directory (24e6ab174071a46c715d03c6e9d6718652097e965ffb3911eb2aa8298a2ef617).
Comparison helpers select their executable explicitly with `--binary`; these
three launches all selected `staging/phase-trace-01/trace-host`. Earlier references
to preserving the original "default" describe the generic build output, not a
change to the session helper's default argument. Both original binaries remain.

### Multicore preference works at startup but full state overrides it

The next authorized slice checked a real vendor control before attempting a
multicore comparison. Arturia's [FAQ](https://support.arturia.com/hc/en-us/articles/24036230171804-Pigments-General-Questions)
identifies the global Multicore option; the [7.0.1 manual](https://dl.arturia.net/products/pigments/manual/pigments_Manual_7_0_1_EN.pdf),
section 3.7.1.1, describes instrument-wide settings that persist across preset
changes. This does not settle what a full host-state restore does.

Installed vendor resources define `Multicore2`, displayed as Multicore and
transmitted to processor parameter `Multicore`: Off/On item order, default On,
`savedinpreset=0`, `savedinstate=1`, `savedinpreffile=1`. The preference file and
both the original operator and 24 AM component/controller states contained On.
Thirty retained Windows logs yielded 124,488 rows including repeated complete
4,446-parameter catalogs; no actual VST Multicore control was found. Resource
positions were not converted into invented VST parameter IDs.

One explicitly reviewed no-note check privately backed up the exact preference
and changed only `Multicore2` from 1 to 0 while all owned plug-in processes were
closed. It used the same 85780f... candidate, tracing OFF and unchanged config.
Actual `getState` results were:

| Observation | Component | Controller |
| --- | ---: | ---: |
| Startup, Welcome | Off | Off |
| After restoring unchanged 24 AM state | On | On |
| After restoring original 8-Bit Crystals state | On | On |

The preference is therefore read and effective at startup, but the full-state
restore resets this control. A preference-only ON/OFF experiment restoring the
same 24 AM state would test On in both conditions. No performance comparison,
notes, GUI interaction, invented parameter, opaque state rewrite or rebuild was
performed. This is a confirmed control dependency, not a multicore performance
verdict. Two focused preference-edit/refusal tests passed.

The session took 43.25 seconds, peaked at 49.6 C with no throttle flags, and shut
down cleanly. Original preset/master 0.48033079504966736 and multicore On were
captured before quit. Preference bytes and mode/owner/mtime were restored exactly
after exit; original generic and helper-default binary hashes, baseline JACK
graph, ondemand, schedstats=0 and no owned services were verified. The one optional
FEX-stat presence sample preceded a loaded Pigments mapping, so establishes no
availability result. No second attempt or prolonged session followed.

Stop this headless control route under the current constraints. A useful next
lever is a separate bounded check of the pinned FEX runtime's existing shared
statistics during an owned workload, to distinguish translation/cache/fallback
work from other processing costs. No FEX cause is inferred here. Sanitized result:
`evidence/rpi2/pigments-multicore-mechanism.json`.

### Existing FEX counters constrain the first-note compilation hypothesis

One unchanged headless session used the existing 85780f... candidate with native
phase tracing OFF. A bounded reader found a 4,096-byte version-2 ARM64EC shared
statistics file for the exact loaded Pigments process, reporting
`FEX-2609-41-g82510eb`. It was accessible in the host mount namespace and did not
require a PID-namespace translation. No feature flag, runtime or binary changed.
The source-defined header is 64 bytes and each linked slot 112 bytes. File names
use Linux PID; slot IDs are Windows thread IDs. No Linux/Windows TID guess was
used. Paired reads checked topology and counter resets; one pre-stimulus interval
lost three slots and was excluded from stable-interval attribution.

Timing fields use CNTVCT reference ticks. Current boot reports the ARM timer at
54.00 MHz and `arch_sys_counter`; raw ticks remain primary and derived seconds
use that reported frequency. JIT-path elapsed time includes lookup, locking and
compilation, not just compile CPU. JIT count measures compile attempts; L1 cache
misses can hit later caches and do not mean retranslation. Lock timers measure
acquisition, and overlapping timers are not summed into exclusive CPU budgets.

The first and repeated 20-second captures both failed audibly relevant continuity
checks, without an operator listening claim:

| Observation | First hold | Repeated hold |
| --- | ---: | ---: |
| Wider status missing frames / gaps | 363,520 / 16 | 630,528 / 3 |
| Exact-zero frames in held 2–14 s | 366,080 | 557,312 |
| Exact-zero frames in stable 3–14 s | 318,080 | 528,000 |
| Windows cohort CPU in held 2–14 s, one core | 149.11% | 173.33% |
| Observed Pigments-process CPU, surviving threads | 147.15% | 153.15% |
| Separate Arturia-named process CPU, surviving threads | 0.12% | 17.46% |
| Compile attempts in stable intervals touching held window | 367 | 1,056 |
| Aggregate JIT-path elapsed in those intervals | 0.1424 s | 0.1658 s |

Touching intervals include activity beyond the requested window edges; fully
contained intervals and the entire sparse timeline are retained separately.
These are process-level counters, not per-audio-thread attribution. First-hold
output begins at capture second 9.5467, then has fifteen short zero runs. The
repeat has one short zero run at 2.384 s and a long exact-zero run from 2.3947
through 15.520 s, followed by another short run. Both recordings were finite;
peaks were 0.03359 and 0.04861. All nine MIDI events per capture were accepted;
processing faults, callback deadline misses and JACK xruns remained zero.

There were **zero compile attempts in every sparse interval overlapping the
repeat's 2–3 s attack**, despite the cutoff in that interval. Its major later
burst was 856 attempts and 122.54 ms of aggregate JIT-path elapsed in graph-relative
seconds 5.277–5.801. A first-note compilation explanation therefore does not fit
the immediate repeated cutoff. Graph-ready stdout arrival only approximates the
capture clock, and counter increments cannot be assigned exact callback times;
the several-second separation is retained without asserting a sole cause.

The measured pass was admitted after outstanding-frame estimates 0, 512 and 256,
with about 100 ms status response times. Active frequency samples remained near
2.4 GHz; maximum temperature was 51.8 C and throttle flags zero. This is another
failure after the same near-drained rule; a warmed baseline is not reliably
established. The observer consumed 16.55 ms of CPU across 42 first-hold snapshots
and 21.46 ms across 53 repeat/tail snapshots; maximum read wall times were
0.571/0.857 ms. No claim that observer overhead is absolutely zero follows.

Retained prior OFF1/ON/OFF2 measured data put the process containing `lvb-audio`
at 142.75/143.93/142.72% of a core, and the separate Arturia-named process at
0.503/0.504/0.496%. Thus extra companion activity explains much of the current
cohort CPU increase, but Pigments-only work also rose. The companion was nearly
idle during the failed first hold, so it does not alone explain both failures.
Names and captured PID grouping establish separate processes; no account or
licensing behavior was inspected or inferred.

The 78.54-second session closed cleanly. Exact original preference bytes,
8-Bit Crystals/master 0.48033079504966736, baseline JACK graph, ondemand/schedstats0
and both preserved binaries were verified; no owned service remained. Five
focused decoder/reducer tests cover corrupt schema/links, reset/churn, empty
samples and boundary attribution. No further runtime test or tuning followed.
The recommended next efficiency experiment is a source/build feasibility review
for 512-frame vendor batching at unchanged48k/JACK512/reserve2048, followed by a
bounded comparison if feasible. This does not justify FEX-cache tuning or stopping
the companion. Execution/wait attribution remains necessary if throughput does
not improve. No batching change or additional run was made in this slice.
Results: `evidence/rpi2/pigments-fex-stats.json`.

## Optimization options retained after the FEX check

The completed comparison changed vendor processing quantum 256 to 512, keeping 48 kHz,
JACK 512 and reserve 2048. Larger calls may reduce fixed overhead per rendered
second; they do not establish adequate throughput or repair backlog recovery.
The private candidate uses map v2/capacity 512, with quantum selected separately.
Default builds and the installed original binaries retain their previous layout.

| Option | Evidence and next useful decision | Why other changes wait |
| --- | --- | --- |
| Processing block size | Compared the same candidate 256 / 512 / 256 with actual audio and completed calls/frames. | Completed below without a demonstrated efficiency win; keep as a selectable experiment. |
| Bridge work per call | Native phase tracing OFF removed its recorder but did not materially reduce sustained vendor CPU. Examine copies, validation, wakeups, allocation, logging and release-build work if larger blocks help. | Measure which side consumes time before optimizing it. |
| Scheduling and CPU placement | The controlled-frequency profile found 8.976 s caller CPU with 0.011 s runnable wait, plus 4.210 s worker CPU with 0.015 s wait in the aligned post-attack interval. | Runnable wait is too small to explain sustained cost in this capture; blind affinity can take cores away from plug-in workers. |
| Governor, cooling and power | Current peaks were 51.25 / 51.25 / 54.0°C with no flags. A 1.9 GHz sample overlapped the 512 gap; the final 256 failure sampled full clock. Earlier governor comparison was not completed. | Thermal failure is not established here; sustained energy/thermal work remains separate. |
| Pigments multicore | Preference OFF was readable at startup; unchanged full state restored ON. A plugin processing worker is active. | Need valid control after state restore before a same-state comparison. |
| FEX translation and cache | Existing counters showed no compile attempts at the earlier cutoff. The new profile identifies counted scalar floating-point loops in Pigments' translated code. A future targeted generated-ARM/codegen inspection could test whether translation adds avoidable cost there. | Map overlap prevents exact guest-instruction attribution; no blind flags, ISA changes or weakened memory ordering. |
| Wine/Proton synchronization | Native Wine/FEX and host labels occupy a small share of sampled user cycles in the measured repeat; the inspected caller hot loop is arithmetic, with little runnable wait. The worker used 1.65 s system CPU in the aligned interval without kernel stacks. | No current evidence supports fsync/ntsync or priority tuning for the caller; worker kernel work remains unclassified. |
| Backlog recovery | Long silence can outlast a transient slowdown; near-drained admission did not guarantee the repeated hold. | Epoch/resynchronization must preserve notes/state and cannot create compute capacity. |
| Editor, graphics and companion processes | GPU rendering has been observed; a separate Arturia-named process used more CPU in the failed repeat. Account for processes separately. | No GUI campaign or stopping/patching authorization services in this test. |
| Controller, preset and state lifecycle | Full state includes settings that override startup preferences. Headless navigation and recall need their own correct lifecycle. | Visible editor or audible startup does not prove these semantics. |
| Memory and asset loading | Page faults, swap, working set, sample I/O and cache locality remain possible resource leads. | Investigate if measurements point there; no blind DSP/denormal changes. |
| Later product tradeoffs | Measure roundtrip latency, reserve/JACK size, sample rate, quality, unison, polyphony, effects and instance count explicitly. | These change latency, workload or sound; portable power and sustained tests remain distinct. |

## Vendor quantum 256 / 512 / 256: no demonstrated efficiency win

The same private native/Windows pair completed three headless sessions using
24 AM, Poly 4, master 0.35, four held notes, 48 kHz, JACK 512 and reserve 2048.
The editor stayed closed and native phase tracing stayed off. Windows setup
reported maxima 256 / 512 / 256; completed-frame/call ratios independently
matched those values exactly. Vendor-reported latency remained 48 samples.

| Repeated capture | First 256 | 512 | Last 256 |
| --- | ---: | ---: | ---: |
| Missing delivery frames, whole pass | 4,352 | 80,896 | 618,752 |
| Reported gaps, whole pass | 17 | 1 | 11 |
| Exact-zero frames during held 2–14 s | 4,352 | 80,896 | 560,640 |
| Pigments host CPU, % of one core during hold | 143.78% | 144.72% | 150.57% |
| Separate Arturia-named process CPU, same window | 0.49% | 0.49% | 0.51% |
| Native transport CPU, same window | 3.80% | 3.57% | 3.97% |
| Windows audio-calling thread CPU, same window | 94.54% | 94.50% | 97.62% |
| Pigments processing worker CPU, same window | 44.60% | 45.57% | 48.03% |
| Peak temperature across all samples | 51.25°C | 51.25°C | 54.0°C |

The first repeated 256 capture had seventeen 256-frame zero runs between
2.533 and 2.709 seconds. The 512 capture had one continuous zero run from
3.797 to 5.483 seconds. The last 256 capture had ten short runs followed by
continuous silence from 2.373 to 15.211 seconds, extending beyond note release.
The same 256 condition therefore varied substantially across sessions.

Warm-ups were also failures: both 256 captures were silent throughout the
12-second hold; the 512 warm-up contained 94,976 exact-zero held frames.
Warm-up delivery losses were 643,584 / 92,672 / 678,912 frames. The separate
Arturia-named process used 19.66% / 7.66% / 23.56% of one core during those
warm-up holds, unlike its low usage in all three repeated holds. Its activity
cannot explain the last repeated failure by itself.

No run reported a thermal/power flag, callback deadline miss, process failure
or JACK xrun. Those counters plainly did not establish continuous output.
The 512 repeated capture included a 1.9 GHz sample at graph-observation +4.176 s,
inside its silence interval, and another at +14.225 s. The other repeated held
windows sampled approximately 2.4 GHz. Frequency variation remains a possible
contributor; it does not explain the last 256 failure at sampled full clock.
Sampling does not prove clock constancy between observations.

Larger processing calls worked correctly, but this comparison does not show a
reliable CPU or audio improvement. Do not promote 512 as a fix. CPU comparisons
use common wall windows, not a falsely matched cohort of executed DSP blocks.
The next useful experiment is a small execution-versus-scheduler-wait comparison
at controlled frequency, preserving this patch and runtime. The audio-calling
thread remained at roughly 95–98% of one core, so that comparison should also
distinguish DSP/translation from active synchronization or host work. A busy
thread does not establish that priority is the fix. Backlog recovery remains
a separate usability problem. No further live run was made in this slice.

The native candidate built in 20.61 seconds; the existing Windows host-only job
completed in 99 seconds. Source commit `9a0ac94` identifies both builds; the
native build manifest records the deliberately preserved installed `panel.rs`
difference. Default builds keep map v1/capacity 256; only the private candidates
use map v2/capacity 512, with a separate processing-quantum selector. Focused
native and portable C++ tests covered layout mismatch, negotiated bounds,
partial/zero blocks, note/parameter boundaries, delayed output, returned-note
expiry and terminal refusal. These tests are distinct from the six captures.

The first launch attempt was refused before runtime startup because the original
launcher permitted only its original executable basename. A separately pinned
launcher accepting exactly the candidate basename corrected this; no original
launcher, runtime, prefix or activation change was needed, and no rebuild was
performed. That failed attempt is retained.

All three completed sessions restored the original 8-Bit Crystals state and
master level. Preference bytes and metadata, original native/Windows binaries,
source files, launcher, governor, scheduler-stat setting and JACK graph were
restored or remained unchanged. No owned runtime was left running; final idle
readback was 47.2°C with flags zero. This is not a battery-energy measurement.

Sanitized results, source/build identities, exact clock samples, audio hashes,
CPU windows and cleanup are retained in
[`pigments-vendor-quantum-comparison.json`](../../evidence/rpi2/pigments-vendor-quantum-comparison.json).
Proprietary audio recordings and state remain private. Draft PR #152 is stacked
on #150; neither a merge nor default installation is claimed.

## Critical-path profile of the 256-frame Pigments candidate

One completed headless profile reused the private native/Windows pair with
GE-Proton11-7-aarch64, UMU 1.4.4, steamrt4-arm64 and FEX
2609-41-g82510eb on Pi 5 kernel 6.18.50+rpt-rpi-v8. Pigments 7.0.1.6772
used 24 AM Poly 4/master 0.35. The stimulus was an **automated four-note chord**
(60/64/67/71, velocity 96) held for 12 seconds within a 20-second stereo
capture. It was not a separately played single note. JACK stayed at 48 kHz / 512
frames, reserve 2048, vendor quantum 256 and map v2/capacity 512. Native phase
tracing and the editor were off. The temporary performance governor and
scheduler statistics returned to ondemand/0; state, master, preferences and JACK
graph were restored, and the session shut down cleanly. No new audio or runtime
build was used.

The first map preflight produced an incomplete `perf` file when an attached
recorder failed to exit on its own; it sent no notes and its session was retired.
The corrected SIGINT stop yielded one warm-up and one measured capture. The
warm-up profiler itself used **18.66 CPU seconds** and woke 5.14 million times
in about 20.8 seconds. That failed warm-up added 755,456 missing frames and all
576,000 held capture frames were exactly zero; it cannot serve as an
unperturbed warm-up baseline. In the measured repeat, `perf` used about **0.02
CPU seconds** and woke once. That repeat still added 53,248 missing frames in
26 groups during the early attack, with 53,248 exact-zero held capture frames.
Its 4–14 second capture interval had zero exact-zero frames; that does not erase
the attack failure or qualify 256-frame playback. Process faults, JACK xruns and
current thermal/power flags were zero. Active ARM clock samples stayed near
2.4 GHz; peak sampled temperature across the session was 54.55°C (52.35°C in
the measured capture).

The measured repeat's post-attack snapshot bracket runs from graph-observation
+4.251 to +13.610 seconds, entirely before scheduled note-off. It completed
**449,792 frames / 1,757 calls** in 9.359 seconds, delivered 449,024 frames and
added no missing frames or gaps. The audio-calling thread consumed **8.976 s CPU,
or 5.109 ms per completed 256-frame call**, with 0.011 s runnable wait. The
Pigments processing worker concurrently consumed **4.210 s CPU, or 2.396 ms per
call**, with 0.015 s runnable wait. Thread ticks divide this into 8.94 s user /
0.04 s system for the caller and 2.56 s user / **1.65 s system** for the worker.
`cycles:u` did not sample that kernel work, and runnable wait does not include
blocked time; the worker's kernel mechanism remains unclassified. These thread
CPU costs are concurrent, not additive latency. Status counters were read about
100 ms after each CPU snapshot;
the exact read brackets and completed-frame denominator are retained. The
previous phase trace's 4.977 ms mean inside `processor.process()` came from a
different run, so its difference from this CPU ratio is not an exclusive host
cost. One 256-frame period is 5.333 ms: the caller is heavily occupied, while
runnable waiting is a small observed fraction. The measured loss counter rose
by 4,096 frames at graph-observation +2.380 s, had risen 34,304 at +3.004 s,
and reached its final +53,248 by +4.251 s. Estimated *current* outstanding work
peaked at an observed 2,816 frames and returned to 256 by +5.499 s;
`request_high` is only a historical maximum.

At graph-observation +2–14 s, 1,628 user-cycle samples were collected. The
perf-selected labels put 90.588% of weighted cycles in
`pigmentsprocessor.dll`, 2.701% in `ucrtbase.dll`, 1.876% in native
`libarm64ecfex.dll`, 1.545% in unresolved JIT space, and 0.264% in our Windows
host. Within the aligned post-attack bracket, the perf-selected processor share
was 90.495%. These are *sampled user execution labels*, not exclusive DSP
time or a breakdown of each translated instruction. The final FEX map had
287,310 valid lines, 11 malformed lines and 4,587 duplicate starts. Pinned FEX
source applies `CompiledCode.Size` to each subblock's map entry, creating
overlapping ranges: 74.376% of held sample weight matched multiple guest RVAs.
Only 0.795% matched multiple modules. Excluding those cross-module conflicts,
**90.075%** of all held user-cycle weight mapped unambiguously to the processor
module. Final-map coverage cannot rule out temporal JIT reuse because no
time-stamped map history was retained.

The perf-selected `pigmentsprocessor.dll` RVA `0x735530` accounts for 25.209%
of all held user-cycle weight, but overlapping labels prohibit treating that as
one exact block's cost. The bounded RVA family `0x7354e0`, `0x735530`,
`0x7356e0` accounts for **29.243%** of all held weight where *every* overlapping
candidate is in that family; on the audio caller alone it is **37.842%**.
Read-only inspection of the exact privately retained module at those RVAs found
counted scalar SSE floating-point buffer/state arithmetic, recurrent state
stores and a subsequent packed-float mix. The inspected central loop contained
no pause, lock, atomic, polling loop or call. This supports **translated
arithmetic/data work in a Pigments loop family** as the dominant identified
critical-path mechanism, rather than active synchronization in that region.
The module is stripped; an adjacent export name is not its internal function
identity. The samples and FEX map do not identify exact guest instructions or
measure whether generated ARM code is inefficient, and they do not prove every
processor-module cycle is useful DSP.

The next justified **caller optimization inquiry** is the generated ARM code
for this scalar SSE loop family: establish whether the pinned FEX lowering adds
avoidable instructions or memory traffic before changing codegen. Separately,
kernel stacks would be needed to attribute the worker's 1.65 s system CPU
before considering any synchronization or kernel-path optimization. Pigments'
algorithm and worker split are vendor-owned costs. This profile gives no basis
to change the bridge's processing quantum, fsync/ntsync, priority, affinity,
runner, quality or preset now. No further live comparison was made.
Sanitized attribution and exact observation brackets are in
[`pigments-critical-profile.json`](../../evidence/rpi2/pigments-critical-profile.json);
licensed binary, disassembly, raw maps, perf data, audio and state stay private.
