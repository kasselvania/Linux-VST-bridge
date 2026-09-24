# RPI2 vendor processing quantum

Base eff40235bb29941476b9fda42122972716c76210. The user authorizes the next
improvement experiment; coordinator reviewed feasibility before any build.
Work only in the isolated rpi2-cpu-efficiency checkout, stacked branch/PR.

Compare the same private native/Windows candidate at quantum256 /512 /256.
Default builds remain map v1/capacity256. Experimental opt-in builds use map
v2/capacity512 with a separate startup-latched processing quantum; IPC minor12
stays unchanged. Wrong peer/layout rejects before processing. Keep sample rate
48000, JACK512, reserve2048, unchanged24AM Poly4/master0.35 and the existing
four-note12-second hold in20-second capture, headless, phase OFF.

Scope: native AP1 storage, queued splitting/setup/lifecycle, appliance selector
and readback, mirrored Windows storage and negotiated bounds, existing session
and comparison helpers, focused tests, sanitized evidence and RPI2 levers table.
Build once each side using cached Pi release and existing5-minute host_only
MSVC workflow; no runtime rebuild or paid capacity purchase. Reuse each pair
for warm-up and measured hold in each condition. Record actual calls/frames,
audio gaps/zeros, process and separate companion CPU, temperatures/frequencies.
Keep failed warm-ups and distinguish higher throughput from backlog recovery.

Test map mismatch, negotiated bounds, zero/partial blocks, MIDI/parameter
boundary offsets and returned-event timing/failure behavior. No concurrent
process on one instance. Preserve originals, runtime/prefix/activation, installed
panel.rs difference, user state/master, preference bytes/metadata and JACK graph.
Do not change governor, scheduling/affinity/priority, FEX flags/cache, vendor
quality/polyphony, reserve, editor or authorization services. Stop at75C/current
power flags or lifecycle fault. No GUI/control of desktop and no credential logs.

Publish one draft stacked PR, commit/push but do not merge/default-install.
Retain limitations and all previously considered optimization levers in concise
RPI2 notes. Source/fake-peer tests are not actual Pigments timing proof.
