# Serum 2 preset transition: native readback refusal

The owner reported using Serum 2.1.5 on the Pi after the Xfer browser flow,
then reported that the session closed after selecting a new preset. The
retained private
`serum-browser-19` session identifies the first host-side refusal as
`parameter readback identity/value`. Its initial selected-control snapshot
accepted IDs 0, 2000000, 2000003 and 7000000. The later revision-3 snapshot
accepted the first three and stopped before reporting ID 7000000. The old
readback check required each refreshed title and units string to equal the
initial binding census, and also treated an explicitly unavailable value as
fatal. The log does not retain the refused message, so it cannot distinguish
renamed metadata from unavailability at ID 7000000.

This is a different observed failure boundary from the first Steam Deck Serum
session in PR #107. That session reported a processing restart and a lifecycle
correlation failure after a preset change; LC1/LC2 corrected the processing
owner's activation-only restart handling. The Pi session had one processing
interval, 262,714 completed blocks, zero bridge faults, zero processing
failures, and an orderly Windows-host processing stop/join before the native
standalone exited with code 1. It did not show the Deck restart failure.

The candidate keeps exact module/class and selected parameter IDs, but treats
title and units as refreshable display metadata. Available normalized values
still must be finite and within [0, 1]. Protocol-invalid results and unknown
selected IDs remain errors. A protocol-declared unavailable value must have
canonical zero bits; it is recorded as missing and clears the panel's old
confirmed value rather than displaying zero or ending the session. Logs record
only the ID and revision when selected metadata changes or is unavailable; no
vendor text or preset data is printed.

Source-owned tests cover renamed metadata, unavailable values, malformed
values/results, wrong IDs and a panel that waits for fresh confirmation. All
28 native standalone library tests passed locally. The Pi Release build reused
the existing ARM build cache with `jack-runtime,rpi2-quantum` and staged the
new binary privately, SHA-256
`bd531170df973bab7383dc82eb5d523615ed0a4003064aaa2f6b5e5f41d6bc26`.
The earlier operator and original native candidates remain unchanged at
`4fd300e5cae799b74fd0d62b53d4afc51a8e288404b920c14ac679cc121edff9`
and `78452d81317aa2a4ac78bf1bc6b8a608250e32188d525e0ccb22f65a84f16895`.

The first candidate launch selected the helper's default graphics config by
mistake. It was stopped before readiness and retired; it is not a preset test.
The corrected `serum-preset-21` launch used the previously working private
WineD3D/software renderer config, 48 kHz, JACK 512, processing quantum 256
and reserve 512. Its editor visibly opened on the initial sound. One
next-preset click loaded a different factory sound, altered the visible
controls, and produced
`RPI1_PARAMETER_METADATA_CHANGED id=7000000 revision=3`. The editor stayed
open and processing continued for more than 15 seconds after that readback.
The candidate completed 67,520 processing blocks with zero bridge faults,
processing failures, callback failures or JACK xruns. It then recorded
`RPI1_CLEAN_SHUTDOWN`, exit code 0, no retained session and system-only JACK
ports. The maximum sampled temperature was 50.7 C with no power warning.

This was one next-preset selection from the initial sound; the owner has not
identified whether it was the same preset that caused the earlier refusal.
No MIDI was sent and this test produced no nonzero audio, so it proves only
that this refreshed-metadata transition no longer ends the session. It
recorded 1,792 missing frames across three gaps; the original session's
76,800 missing frames across 239 gaps remain separate. This source change
makes no audio-delivery, audible-playback or DSP-capacity claim.
