# RPI2 — native ARM Wine/FEX on Pi 5

## Result

After the operator activated Pigments in ASC under the ARM runtime, the unchanged
state probe succeeded in a fresh Pigments process. Initial state capture returned
155,154 bytes in 150 ms before editor attachment; the post-editor call returned
155,286 bytes in 168 ms. Both had returned `1` with zero bytes before activation.
This resolves the observed state-capture refusal in the migrated fixture and
strongly identifies authorization readiness as its cause. Audio, preset changes,
full save/reopen and sustained performance remain unqualified.

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

The subsequent real Pigments attempt stopped before audio/editor startup:
Pigments initialized and exposed its 4,446 parameters, but `IComponent::getState`
returned `1` without writing any bytes. No preset selection or playback was
reached. This is a concrete state/initialization failure, not DSP or thermal
evidence. Details and the preserved failure are below.

The approved follow-up reached Pigments' editor through the existing vendor-access
host mode: editor-open, editor-close, component termination and module-unload
records were obtained. This separates the initial state refusal from the ability
to attach its editor. A subsequent single-call probe found that state capture
still returned `1` with zero bytes after 5.007 seconds and 401 message-pump turns.
Rendered appearance and responsiveness remain unobserved. The earlier attempts,
including two launch-adapter mistakes, are preserved below.

A read-only follow-up found a vendor machine-identity error in the migrated
environment: ASC reports `Can't read file <REDACTED_PATH>: machine key changed.`
It appears 39 times across three retained ARM-environment agent logs and zero
times in the three original-environment agent logs. This is a concrete migration
fault to resolve before interpreting state refusal as a host defect; its causal
relationship to that refusal remains unproved.

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

Pigments reached initialization and editor attachment. After operator activation,
state capture succeeded before and after the measured editor-pumping interval.
Demanding-workload audio deadlines, sustained throughput, preset navigation,
state recall, ShieldXL controls, and sustained thermal behavior remain unqualified
on this runtime. Hangover remains an alternative; it was not installed or executed.
