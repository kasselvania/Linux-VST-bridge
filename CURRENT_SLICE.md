# RPI2 vendor processing quantum

Base eff40235bb29941476b9fda42122972716c76210. The user authorizes the next
improvement experiment; coordinator reviewed feasibility before any build.
Work only in the isolated rpi2-cpu-efficiency checkout, stacked branch/PR.

Compare the same private native/Windows candidate at quantum 256 / 512 / 256.
Default builds remain map v1/capacity 256. Experimental opt-in builds use map
v2/capacity 512 with a separate startup-latched processing quantum; IPC minor 12
stays unchanged. Wrong peer/layout rejects before processing. Keep sample rate
48000, JACK 512, reserve 2048, unchanged 24 AM Poly 4/master 0.35 and the existing
four-note 12-second hold in 20-second capture, headless, phase OFF.

Scope: native AP1 storage, queued splitting/setup/lifecycle, appliance selector
and readback, mirrored Windows storage and negotiated bounds, existing session
and comparison helpers, focused tests, sanitized evidence and RPI2 levers table.
Build once each side using cached Pi release and existing 5-minute host_only
MSVC workflow; no runtime rebuild or paid capacity purchase. Reuse each pair
for warm-up and measured hold in each condition. Record actual calls/frames,
audio gaps/zeros, process and separate companion CPU, temperatures/frequencies.
Keep failed warm-ups and distinguish higher throughput from backlog recovery.

Test map mismatch, negotiated bounds, zero/partial blocks, MIDI/parameter
boundary offsets and returned-event timing/failure behavior. No concurrent
process on one instance. Preserve originals, runtime/prefix/activation, installed
panel.rs difference, user state/master, preference bytes/metadata and JACK graph.
Do not change governor, scheduling/affinity/priority, FEX flags/cache, vendor
quality/polyphony, reserve, editor or authorization services. Stop at 75°C/current
power flags or lifecycle fault. No GUI/control of desktop and no credential logs.

Publish one draft stacked PR, commit/push but do not merge/default-install.
Retain limitations and all previously considered optimization levers in concise
RPI2 notes. Source/fake-peer tests are not actual Pigments timing proof.

## Completed disposition

Three sessions and six captures completed. Windows setup and completed frames
per call confirmed 256 / 512 / 256 with the same private binary pair. No reliable
CPU or audio improvement was demonstrated; repeated delivery losses were 4,352 /
80,896 / 618,752 frames and the two 256 conditions differed strongly. All runs
peaked at 54.0°C with zero thermal flags. A 1.9 GHz sample occurred within the
512 silence interval, while the last 256 failure sampled full clock. Preserve
that uncertainty. No change to the default is justified by this result.

The first pre-runtime launch refusal and its exact-basename private launcher
correction are retained. Original state/master, preference bytes/metadata, source
files, binaries, runtime settings and graph are restored; no owned unit remains.
Evidence and remaining optimization options are in docs/experiments/RPI2.md and
evidence/rpi2/pigments-vendor-quantum-comparison.json. Draft stacked PR #152 remains
unmerged. Next recommendation: bounded execution-versus-scheduler-wait comparison
at controlled frequency; no further live experiment in this slice.
