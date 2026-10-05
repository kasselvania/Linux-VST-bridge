# Native Test Tone control — 2026-10-04

The matched native-only control retained **600.000626008 seconds** of its declared
active bracket (exactly600 seconds /28,800,000 stereo frames between file-byte
snapshots). There are no active ≥48-frame stereo-zero spans, no nominal recurrence
errors above the predeclared1e-5 threshold and no new Bitwig/recorder/sink graph ERRs
in that bracket. Its original fixed1000Hz model **remains failed**. A separately
labeled single global frequency/phase/amplitude fit characterizes the retained
signal without a rerun, threshold change, per-window rephasing or omitted samples.
See the [sanitized complete records](2026-10-04-native-tone-control.json).

This is a separate project containing only Bitwig's native Test Tone as an active
source: sine, visible1.00kHz after explicit1000Hz entry, gain-24.0dB, mix100%, track
-10dB and master0dB, with no clips or MIDI gating. It uses the same Bitwig6.1
Flatpak, frozen refresh5 software, PipeWire/System Out GUI settings and512/48k
Bitwig graph as the [commercial baseline](2026-10-04-pure-lofi-bitwig-baseline.md).
Output links negotiate stereo F32P into the explicitly non-autoconnecting F32LE
48kHz recorder (recorder quantum1024). No microphone is linked. DSP/maintenance
counts are0 before/after; sampled processes have no mapped bridge DSP sessions.
One healthy managed keeper remains. Native GUI/DSP load differs from the vendor
editor and does not establish the same contention conditions.

The entire900-second recording is retained:43,200,000 stereo frames /86,400,000
finite sample values,345,600,000bytes, SHA256
`ef60d862b08f13a60433f8e5781a9f603a1faad962a9e8eb4e8d35e535b224a2`.
Source-on pre-roll lasted about242seconds before the declared marker. Whole-file
≥48-frame zero spans occur only at initial setup and after normal source disable;
no clean subsection was selected from the declared bracket.803 observer samples
have maximum interval1.347778458seconds. Bitwig/recorder ERR stays0 throughout the
whole capture. The sink has one0→1 error increment before the declared bracket,
then remains1; this setup witness is retained. Display dimming and late title-bar
wake are observed, so a constant graphics workload is not claimed. Concurrent
activity consists of read-only trace-capability checks on Deck and evidence drafting
and analysis off machine; native priority, source and global settings stay unchanged.

The fixed1000Hz oracle has max residual0.0399106028 and28,773,700 values per channel
above1e-5, so its `passed=false` is preserved. Independent nominal-frequency
recurrence max is2.69462682e-8 with zero threshold exceedances. The executor's
single all-sample recurrence-frequency estimate is999.998569495341Hz. One global
sine/cosine/DC fit evaluated every active sample/channel at that frequency: max
residual1.27493156e-6, RMS5.00524622e-7 and zero values above the unchanged1e-5.
Both channels are identical. This is supplemental characterization, not replacement
of the declared oracle. The discrepancy is consistent with a small nominal-frequency
mismatch; oscillator, resampler and clock causes are unassigned. Zero absence alone
is not the continuity argument, and no universal glitch-free claim is made.

The recorder reaches its exact900-second finite sample extent and has already
exited1 at the normal-stop observer, before the wrapper could send SIGINT; stderr
is empty. PipeWire1.6.4's finite-record limit quits without setting drained; main's
initial failure status changes to success only on drain. This is source-consistent
finite-limit completion, **not proven SIGINT termination**. pw-top/wrapper exit0
and the wrapper retains no capture failure. [PipeWire1.6.4 recorder source](https://raw.githubusercontent.com/PipeWire/pipewire/1.6.4/src/tools/pw-cat.c)

Normal transport stop, source disable, save/quit and Moonlight disconnect complete.
No test consumer/recorder remains; registry/software/Pure LoFi preferences stay
exact. The initial read-only tracefs-stat permission exception and a post-retirement
pw-dump failure from missing XDG_RUNTIME_DIR are retained helper failures, corrected
without altering or repeating playback. Trace tools exist, but tracefs controls are
root-only and sudo-n is unavailable. No privilege change or trace has occurred.

Private archive `refresh5-native-control.tgz` is144,386,059bytes, SHA256
`a298845002fc7a0124341297c61b7a40f7494fc3e99ad45284ecf24b113de771`, with50 verified
manifest files. Supplement `native-control-supplement.tgz` is1,722,209bytes, SHA256
`5d5ab6bfcccdd1a149dc06e80f24e80b027bc4cb944d9f88de44e8cab2383700`, with31 manifest
files. Complete audio/project/GUI/process records and every analysis are durable
outside Git; only sanitized claims and hashes are public.

This control supplies a positive native continuity witness under its declared
conditions. It does not eliminate graph/capture/scheduling effects under commercial
load or establish a bridge/vendor cause for the earlier zeros. The next bounded
user-authorized diagnostic is a matched paced Bitwig ordinary-versus-RR worker
comparison and one private kernel trace, after exact protocol/access review. Actual
per-call callback timing, DAW/device deadlines, thirty-minute interaction, longer
soak and product scheduling repair remain unperformed.
