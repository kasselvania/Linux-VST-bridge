# BR1 first Serum gate: bounded failure after one preset change

This is the first commercial follow-up to the source-approved BR1 head
`ce4c27aac25ddb2444b0ad2a13dc4534ec6ddd8f` (tree
`51ba3bf1d831f83e443cad8661253e467e80194f`). It is one operator
interaction, not a preset sweep or DSP-capacity measurement. Raw Windows and
native logs remain private under the retained `serum-br1-gate-24` run label;
this file contains only bounded observations.

## Candidate and setup

The native ARM64 candidate was built from that exact source head in a separate
private staging directory with the existing Release profile and
`jack-runtime,rpi2-quantum` features. Its SHA-256 was
`4b7259edc8ed5290aaba96b7c31c2c0a70d6432c399f235ea54cc5f1ffcf1403`.
The latter feature preserves the retained map-v2 capacity of 512 frames while
the processing quantum remains 256. The previous native candidate
(`bd531170df973bab7383dc82eb5d523615ed0a4003064aaa2f6b5e5f41d6bc26`)
was not replaced.

The Serum 2.1.5 module SHA-256 remained
`501e7bb3dd9cafe416b3412df3d4e084c01b7468201e5690b9009d7ecd4e5283`.
The retained WineD3D/software-OpenGL launcher, Windows host, UMU environment,
authorization state, JACK 48 kHz/512, bridge reserve 512, editor route and
ShieldXL/OMX-27 routing were used. The selected BR1 policy was
`LVB_RPI2_LIVE_RECOVERY=1024:1500:1`. OMX-27's
`system:midi_capture_4` was connected to Serum MIDI input; stereo Serum
outputs were connected to ShieldXL playback 1/2.

An initial staging attempt (`serum-br1-gate-23`) used
`jack-runtime` without `rpi2-quantum`. It produced a 256-capacity native map,
and the retained 512-capacity Windows host refused it with
`ap1_transport_error: backing file size` before audio readiness. That was a
native build-feature mistake, not a Serum result. It was stopped and its
private log retained. The corrected candidate above reported map version 2,
capacity 512 and quantum 256 before the commercial interaction.

## One operator interaction

The editor opened on the working instance. The owner played the initial sound
from the OMX-27 and heard it. Immediately before the change, native status
showed 77 accepted MIDI events, 790,199 nonzero output samples on each channel,
12,167 completed processing blocks, historical request high-water 6,
`fault=0`, no processing failures and no JACK xruns. There were already
2,304 missing frames across three gaps.

The owner released the keys, selected one factory preset, and reported that
the preset loaded. Its name was not retained. Selected parameter metadata
advanced to revision 3. When the owner played again, the visible plug-in
window closed and audio stopped. The first retained terminal status showed
`fault=7` (`recovery_exhausted`), 133 processing failures, 16,825 completed
blocks, historical request high-water 6 of 2,048, 124 accepted MIDI events and
no JACK xruns. A later status showed 250 processing failures with completed
blocks still at 16,825. Cumulative output had reached 1,077,908 nonzero
samples per channel, but those counters alone do not establish when audio
resumed. Final callback totals recorded 4,608 missing frames
across eight gaps and 1,280 re-priming frames.

The retained private performance trace resolves the recovery sequence more
precisely. It records an epoch-1 gap around frame 1,523,712, followed by
epoch-2 returned audio with nonzero samples. The owner later heard the initial
sound before changing the preset. Thus the one allowed same-instance recovery
had already been used before that preset change. The final slow request was
in epoch 2, sequence 16,825, at frame 2,782,976. Its measured full service
span was 39.854 ms, including a 38.801 ms Windows processing span. The
configured 1,024-frame lateness horizon is 21.33 ms at 48 kHz. The terminal
fault was recorded at frame 2,785,024, while the next processing request was
in flight. This is a second live-lateness incident, not a failure to execute
the first STOP/START. The trace cannot divide that lateness between the last
completed request and the in-flight one.
The trace does not identify why the earlier recovery was needed or how much
of the later Windows processing span was the plug-in's DSP itself.

The Windows-host lifecycle log recorded successful
`set_processing_false` and `set_processing_true` results, followed by the
new START acknowledgement. This establishes that the same-instance
STOP/START portion executed. Fault 7 records the later live-lateness
incident after the recovery budget was spent; the record does not establish
playable audio after the preset change. The retained log does not establish a
vendor exception or Windows-host crash before the owner stopped the service.
The owner's report that the plug-in “crashed” describes the visible outcome;
the initiating stall remains unknown.

## Containment and disposition

The low-rate session helper classified a terminal audio fault and sent
`quit`. Its 25-second wait expired, so its finalizer stopped the owned
service. The native process did not record `RPI1_CLEAN_SHUTDOWN`; the
session directory was retained for private inspection. Afterward the owner
service was inactive, no candidate or Windows-host process remained, JACK
returned to system-only ports, and the sample rate/period were still
48 kHz/512. Maximum sampled temperature was 54.0 C with no current power
warning. The original native candidate hash remained unchanged.

This is a **bounded commercial failure**, not a Serum usability success. BR1
prevented the prior 2,048-request stale backlog, recovered once before the
preset change, and exposed a specific exhausted-recovery classification during
a later processing delay. The preset change still
failed to return playable audio and did not retire normally. No second
preset, source change, BR2 work, FEX tuning or latency experiment followed.
