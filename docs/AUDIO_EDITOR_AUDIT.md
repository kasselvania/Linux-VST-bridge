# Audio and editor interplay: audit of 2026-10-06

> Audit record, not design authority. It states what the documents decide, what
> the code does, what the Steam Deck has recorded, and where those disagree. It
> proposes a pass rule and a test. It changes no behaviour.

Scope: how real-time audio is protected from a plug-in's editor, graphics and
input handling on the maintainer's Steam Deck (Bitwig 6.1 Flatpak, 48 kHz,
512-frame blocks, Buffered 512 delivery unless stated). One fixture; no claim
for other hosts.

## Headline

The bridge's loss counters have been zero since 2026-10-05, and they cannot see
the failure the operator hears. When the Windows side is late, the DAW's audio
callback waits for it, in 4 ms slices, for up to five seconds. A wait that ends
in a result is recorded as a successful callback with no missing frames. The
only trace is the callback-duration histogram, and it shows callbacks of up to
4.2 seconds in sessions that ended clean.

## What is decided

- Callback law: no unbounded wait and no dependence on the manager or editor;
  a late or dead host must not stall the DAW (`AGENTS.md`, Real-time laws).
- Decision D-030 (`docs/DECISION_REGISTER.md`, 2026-10-04) allows the callback
  to wait under the five-second host-health bound and accepts "the possible
  seconds-long DAW-thread stall at failure".
- `docs/PLATFORM_ARCHITECTURE_REVIEW.md` says never to turn that five-second
  containment into a callback wait.
- These three do not agree. D-030 is the latest and is what the code does.
- Editor and DSP share one Windows process. The editor runs on the owner
  thread, DSP on the `lvb-audio` render thread (`docs/ARCHITECTURE.md` 6.4,
  18.1).
- The render thread and the native transport worker request `SCHED_RR` 5
  through RealtimeKit. Every other thread of the owned Windows family is
  raised to nice 10. The DAW's own callback thread is left alone unless
  `LVB_CALLER_SCHEDULING=1`.
- Buffered delivery adds D frames of delay (512 frames, 10.67 ms, by default).
  Same-callback delivery adds none.
- Beta gate (`docs/INTEGRATED_BETA_DELIVERY.md`): a 30-minute interaction run
  and a two-hour soak with zero unexplained missing frames. It does not mention
  callback duration.

## What the code does

Verified in source for the two points the headline rests on; the rest is from a
read of the files named.

- The DAW callback waits for the result it needs in futex waits of at most 4 ms
  until a five-second deadline (`native-vst3-proxy/backend/src/performance.rs`
  `AUDIO_CONTAINMENT`, `INTERRUPT_INTERVAL`; `queued.rs` wait loop). On expiry
  the instance fails and outputs silence from then on. It never repeats or
  skips a block to stay on time.
- Frame counters are added only for callbacks that return success, and a late
  success adds nothing to `missing_frames`, `expired_frames` or the underrun
  totals. `native_callback_completion` records the duration of successful
  calls and the number of waits.
- While a state capture is in flight, the render thread can wait on the owner
  thread in 4 ms slices for up to ten seconds
  (`windows-factory-probe/source/mapped_processing.cpp`, `dispatch_capture`).
  The owner thread is the one that runs the editor's message loop.
- In Buffered mode the render thread is woken by the transport pump, which is
  one of the threads raised to nice 10.
- The owner thread polls about every 50 µs for as long as audio runs.
- Nothing in the code changes audio behaviour when an editor is open, and the
  audio path does not call into editor servicing.
- A lock inside the vendor's own code, shared by its editor and its DSP, is not
  something the bridge can see or mitigate. The editor thread holding it runs
  at nice 10.

## What the Deck has recorded

From 538 retained session logs, 450 with audio, about 100 hours.

| Period | Sessions | Hours | With lost or underrun frames |
|---|---|---|---|
| 2026-09-08 to 09-30 | 274 | 66.2 | 214 |
| 2026-10-01 to 10-04 | 131 | 8.4 | 38 |
| 2026-10-05 to 10-06 | 45 | 4.3 | 0 |

Callback duration, Buffered 512, 2026-10-05 16:00 onward. A block lasts
10.67 ms.

| Plug-in | Sessions | Callbacks | Over 25 ms | Longest | Waits in the worst session |
|---|---|---|---|---|---|
| Pure LoFi | 4 | 399,749 | 0 | 5.1 ms | 685 |
| Nibbi | 19 | 138,771 | 6 | 815 ms | 202 |
| BEAM | 2 | 33,847 | 4 | 4,248 ms | 1,063 |

- Both BEAM sessions had the editor open, ended without error and report
  `clean: true`. Each contains one stall of about four seconds: roughly a
  thousand consecutive 4 ms waits.
- Five of the six Nibbi sessions with a callback over 25 ms contain a state
  capture made while audio was running (save, delete or quit). In those, the
  longest callback was 26 ms, 39 ms, 97 ms and 105 ms where the capture
  returned, and 815 ms where it failed before the state-size correction. The
  sixth, 786 ms, is the 2026-10-05 session on the standard runner that ended in
  the editor-removal stall, with no capture. A longest callback in the same
  session as a capture is a coincidence in time only until the two are
  recorded against each other.
- Same-callback trials on 2026-10-05 (Pure LoFi, 18 sessions, 846,256
  callbacks): 8 callbacks over 10 ms, longest 15.9 ms.
- The request to make the DAW's callback thread real-time was refused in 21 of
  27 recent sessions (`unsupported_supervisor`). The Bitwig Flatpak has no
  system-bus access to RealtimeKit.
- Deck settings at audit time: 8 CPU threads, governor `powersave` with
  `balance_performance`, user `rtprio` limit 0, PipeWire data loops at
  `SCHED_RR` 20.

## Resting cost

Measured the same day by counting each thread's wake-ups and CPU time over
five seconds. Percentages are of one of the Deck's eight CPU threads.

With no plug-in loaded and Bitwig closed, three Wine environments were still
alive on Test 14 (30 processes, about 190 threads) and used about 114%:

| Process | Wake-ups per second | CPU | Count |
|---|---|---|---|
| Wine device processes | 33,544 | 47.9% | 6 |
| Wine servers | 18,706 | 31.7% | 3 |
| Python supervisors | 89 | 23.5% | 5 |
| Xalia gamepad navigation helper | 5,981 | 9.6% | 1 |
| Environment owner hosts | 221 | 1.8% | 3 |

- In every environment one Wine device process had `winebus.sys`,
  `winehid.sys` and `winexinput.sys` loaded and the Deck's built-in controller
  and touchscreen open as raw HID devices. This is the runner's game-input
  stack. A plug-in host uses none of it.
- The supervisors wake rarely and cost a great deal per wake.
  `bridge-manager/runtime/session.py` is about 4,900 lines of Python in the
  runtime supervision path; `AGENTS.md` names Rust as the product language.
- No rule was found that retires an environment whose plug-ins have all been
  unloaded.
- With BEAM loaded, its editor open and in use, the whole system used about
  three CPU threads, and the audio server counted 4 to 14 errors per 30
  seconds for Bitwig. With the editor closed it counted none. BEAM's editor is
  an embedded Chromium (WebView2) with renderer, GPU and compositor processes.
  The host has the Deck's GPU device open and the hardware driver loaded, so
  this is not software rendering.
- Bitwig's `audio-N` threads run at ordinary priority. `lvb-audio` has its
  real-time priority.

Test 15 disables the HID bus driver and Xalia for plug-in host, keeper and
preflight launches only. Its effect has not been measured yet: the update
leaves no environment running.

## What is not established

- Whether each long callback was heard. No DAW-side or PipeWire-side
  measurement was taken in these sessions.
- What triggers BEAM's four-second stall. It is one event per session. State
  capture during processing, editor construction on the owner thread, and a
  vendor lock are all consistent with the records; none is shown.
- That editor interaction after opening (knobs, menus, touch) costs audio.
  The 2026-10-06 BEAM session has nine gestures and two callbacks over 25 ms;
  the records do not tie one to the other.
- Editor-open evidence without long callbacks exists for Pure LoFi only, and
  for 600 seconds, not 30 minutes. `docs/FAILURE_CLASSES.md` says both that the
  Pure LoFi symptom is gone and that it is blocked for dependable use.

## Ranked gaps

1. **Measured.** Lateness is invisible to the pass criterion. Zero missing
   frames is reachable while the DAW stalls for seconds.
2. **Measured as co-occurrence; mechanism read from code.** A state capture
   during playback appears to hold the audio callback for the time the plug-in
   takes to write its state.
3. **Measured, cause open.** BEAM stalls about four seconds once per session
   with its editor open.
4. **Read from code.** The render thread can wait on the editor's thread
   during capture, and its wake-up in Buffered mode comes from a niced thread.
5. **Measured.** The DAW's callback thread is usually not real-time, so the
   path into the bridge can be preempted by anything.
6. **Inferred.** A reactivated render thread may start at nice 10 until its
   real-time grant lands, and the grant and the nice exemption both depend on
   the thread name being set.
7. **Not measured.** CPU governor, graphics driver threads, compositor and
   wineserver contention while an editor draws.

## Proposed pass rule

A session passes only if all of these hold over its whole length:

- no missing, expired or underrun frames, and no rejected callbacks;
- no audio callback longer than half a block period (5.3 ms at 512 frames and
  48 kHz) in Buffered mode;
- no increase in the audio server's error count for the DAW;
- no instance failure and no editor failure.

One long callback fails the session. A clean stretch inside a failed session is
not a pass.

## Proposed test

Per plug-in, one instance, transport playing with sound for the whole run:

1. Ten minutes with the editor closed.
2. Ten minutes with the editor open and untouched.
3. Ten minutes working the editor by mouse or trackpad: knobs, menus, resize.
4. Ten minutes working it by touchscreen.
5. Save the project twice during playback, then close and reopen the editor
   twice.

Each stage is its own recorded interval, so a stall is attributed to the action
that was happening. Read back the frame counters, the callback histogram and
wait count, the audio server's error count and the scheduling of the audio
threads. Run Pure LoFi first as the known-good reference, then BEAM, then
Nibbi. The 30-minute and two-hour runs follow only after every stage passes.

## Before fixing anything

The first change this audit asks for is measurement, not a repair: record the
time and length of each callback over the threshold, and the action in
progress, so the next BEAM session names its own cause.

Last reviewed: 2026-10-06.
