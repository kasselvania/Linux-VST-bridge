# Pure LoFi / Bitwig first commercial baseline — 2026-10-04

The first actual commercial playback attempt on refresh5deck remains **unqualified
for dependable musical continuity**. The complete 811.365040484-second active
bracket was retained: its output contains sixteen exact stereo-zero spans of
512 frames each, and Bitwig's PipeWire node ERR count increased by eighteen.
Bridge delivery counters remained zero for missing/gap/expired output, and both
native and Windows audio workers were freshly promoted to RR priority5. These
facts do not identify the source of the zeros or establish a scheduling repair.
The [sanitized records](2026-10-04-pure-lofi-bitwig-baseline.json) retain the full
scan, graph changes, timing brackets, exact identities and private evidence hashes.

## Fixture and installed path

Arturia Pure LoFi1.0.0.6121, class `417274754156495350724C4650726F63`, uses the
maintainer's existing lawful installation and pinned
`proton-11.0-2c-25118279-slr4-4.0.20260805.254769` runner. Its exact module digest is
`b3e8ca7477487d0dc7fbd3f8815e2f9aec7c8b701f4cb65cf96433c14fa11636`.
Only this commercial class followed the offered rescan → inspection → preparation
→ replacement → SameCallback route. Selected revision
`a31b86347649682d6dd8dfd8520425cc` uses engine
`add49d7a5ed38c44d55395f58d471656d33a7ef8f097cdd43de2332ebe04fbd0`, from source
`d970eda91be4bc972d38a0d4b107cc5f4ededa54`, tree
`57a484beee1f28cb6d0df85f25977b458d64a1ff`. Actual loaded mappings independently
prove that engine and paired Windows host
`c8587d8c75ddce80c2317a10070735c65b30b6bc1649ff219821c44689bcf05f`.
Software remains `0.12.0refresh5deck`; no source or scheduling changes were made.

The original revision, its retained parent/artifacts, all preferences and the
original project stayed exact. There are ten published classes in an eleven-class
inventory: all **nine unselected published siblings** and ten unselected class
records remained unchanged. A separate working project was saved normally; opaque
vendor/project bytes remain private. Save and state operations are observed, but
this baseline did not reopen that project or claim persistence qualification.
The normal engine-update UX gap remains open: this manual reprepare is not evidence
that Update Bridge refreshed commercial publication engines automatically.

## Host, workload and capture

Physical Galileo Steam Deck runs SteamOS3.8.16 build20260716.1 and kernel
6.16.12-valve24.5-1-neptune-616-gb2f7cfe85e45. Bitwig6.1 is the system Flatpak
`com.bitwig.BitwigStudio/x86_64/stable`, commit
`8a048e733e74dda8b897339436153a2d5df952f29d362dfcaca9d8e0d6f6c231`, on
freedesktop runtime25.08.16, commit
`bd44a6230581917d04f89812a4c21090c304d390edb73995af1c2f9fd8abf4e8`.
Flatpak1.16.6 and PipeWire1.6.4 are retained version-specific facts.

GUI audio settings show PipeWire / System Out / Automatic sample rate / Auto512
samples (10.67ms). The actual graph locks 512 frames at48kHz. VST3 setup negotiates
maximum M512 and transport maximum512 at48kHz, D0, with vendor latency48 frames.
**Actual per-call VST3 N and the DAW/device deadline are unobserved.** These
separate facts must not be collapsed into an asserted512-frame callback deadline.
The recorder has autoconnect disabled and is explicitly linked only to Bitwig's
stereo output, retaining the speaker links. Negotiated links use F32P; recorded
output is F32LE48kHz, FL/FR stereo. No microphone source is recorded.

The working copy has one Pure LoFi instance and a looping one-bar4/4 clip at110BPM:
E3/G3/A3/G3 notes, each0.25beat, 100% condition, with0.75beat rests between notes.
Opaque vendor release/noise means MIDI rests do not define exact output silence.
The editor is visibly open at start/mid/end with meters; late display dimming and
wake are retained. These samples do not prove a constant graphics workload.
Automatic playback is **not user interaction**. Immediately before DAW launch
DSP/maintenance/keeper counts were0/0/0; earlier inspection/preparation may have
warmed the environment, so this is not a forced cold comparison.

The requested minimum was600seconds. Compaction/readback extended the first attempt
to811.365040484seconds; no restart or clean600second subsection was selected. Start
and end byte snapshots span38,935,552 frames, or811.157333333seconds. Recorder
buffering prevents exact sample-to-wall alignment. The entire895.445333333second
file contains42,981,376 stereo frames /85,962,752 sample values,343,851,008bytes,
SHA256 `54c736728ca206fa171d2df2427e6efd9ed2c9ab10b6307add9d27a1454b7c2f`.

An earlier detached **setup** recorder disappeared after SSH closure, before
playback started. Its termination cause is unobserved and its143,360byte partial
is retained. The actual baseline recorder stayed alive for all786 observer samples
(maximum observation gap1.378799298seconds). It was intentionally interrupted with
SIGINT before the900second sample limit, returning1 with empty stderr; pw-top and
the wrapper returned0. Upstream PipeWire1.6.4 initializes failure status and changes
it to success only on drain; SIGINT quits the loop without setting drained. This
is source-consistent signal termination, not an asserted recorder exit0 or an
unattributed audio failure. [PipeWire1.6.4 recorder source](https://raw.githubusercontent.com/PipeWire/pipewire/1.6.4/src/tools/pw-cat.c)

## Whole-session observations and cleanup

The whole-file float32 scan contains no nonfinite values. The byte-bracketed active
window has sixteen exact stereo-zero spans of512frames each:8,192frames total.
The1e-7 near-zero scan finds the same sixteen active spans. Full-file setup/stop
spans are retained separately. No adjacent sample step exceeds the declared0.05
scan threshold, but silence/step scans are not a universal click/glitch oracle.
All unexplained spans remain unresolved.

Bitwig node ERR rises10→28 in the active bracket; it reaches29 after that bracket.
The speaker sink remains ERR1 and the recorder remains ERR0. Thirteen zero spans
have **nearby associations** with Bitwig error increments using their byte-observer
brackets expanded by±1second; three do not. These are not direct time overlaps,
proven corresponding xruns, or bridge-origin attribution. Bridge missing/gap/expired
counters stay zero throughout all786 samples.

The authenticated native worker belongs to the Flatpak consumer; the supervised
Windows host is outside Flatpak. Both effective policies are RR|RESET_ON_FORK,
priority5, with RTTIME soft/hard200,000µs. Raw LVNS reply code1 means successful
`effective`, while code2 would mean `already_effective`. Retirement confirms native
OTHER0→RR5 through owned-supervisor RTKit and the successful Windows RR5 request.
This protocol code1 is unrelated to the earlier SDK `busctl` exit1 failure. The
historical SDK consumer was outside Flatpak; its precise RT refusal remains
unobserved. No portal or sandbox denial is inferred. Sampled AudioEngi audio/task
threads are ordinary; the exact callback-owner TID was not established.

Native completion telemetry covers the entire lifetime, not just this capture:
122,416 waits, none without a result, maximum15,187,161ns. It includes vendor DSP
and bridge waiting, excludes final C++ SDK sinks and is **not isolated bridge
overhead** or a device-deadline measurement. Native final counters are clean.
Normal GUI stop/save/quit retired the DSP session and transport; Windows raw exit
-15 was gated with confirmed cleanup, not a vendor exit0 claim. Final DSP and
maintenance counts are0, with one healthy managed keeper retained. Manager and
Moonlight stream closed normally; licensed state, software and siblings stayed exact.

## Retention and next discriminator

Private archive `refresh5-commercial-baseline.tgz` is308,316,952bytes, SHA256
`ee254cf83a3d5a47e55e00ffab948d2f2e04939e2010a1b00abf24ea30188993`; its94-file
manifest is verified. Supplement `baseline-supplement.tgz` is9,567,819bytes, SHA256
`4bfb58dd22ca8a493b6a88d84d5e4edae530cc8849b8ece39bf2a97e21a76fa9`.
These archives and complete output, working project, GUI/process/mapping/limit
records and independent analyses are durable outside Git. Only sanitized claims
and hashes are public.

The approved next discriminator is a separate continuous **native Test Tone**
control, with the same512/48k graph and capture conditions and no bridge DSP owner.
It uses a predeclared fixed-frequency model plus recurrence/zero-run checks for the
entire active window. Native GUI load differs from the vendor editor. Subsequent
user-authorized native ordinary-versus-RR diagnostics and kernel scheduler tracing
need exact owned-TID restoration and a reviewed bounded capture protocol. No
thirty-minute interaction, long soak, scheduling repair, general compatibility or
beta claim follows from this unqualified first commercial baseline.
