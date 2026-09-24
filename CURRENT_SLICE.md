# Digitalis two-hour owner session

The operator requested a bounded, informal two-hour Digitalis stress session
on the existing Pi 5 / SHIELD XL, with the editor open and JACK period,
processing quantum and bridge reserve all at 128 frames / 48 kHz.

Base: `28218b37ffba9ebf4983593ff6b8e8d067d1e2b9`.
The private native build reuses the previously identified timing-selector
source bundle, changing only `rpi1/standalone/src/supervisor.rs` to select a
finite Windows-cohort runtime at launch. The default remains 300 seconds;
an explicit `LVB_RPI1_RUNTIME_SECONDS` admits 30–7200 seconds.

The private session wrapper supplies its selected session duration. The
existing memory/task bounds, thermal/power stop, saved sound, Windows host,
runner, plug-in, and 128-frame processing path remain unchanged. Original
native and private helper files are retained for rollback. No live dry/bypass
or preset change is part of this request.

Acceptance for this setup is the focused runtime-selection test, Pi build,
read-back Windows limit of two hours, 128/128/128 readiness, editor presence,
stereo JACK routes, and a starting counter snapshot. Leave the supervised
session running for the returning operator. Starting the session does not
establish a completed two-hour run, gap-free audio, or analog latency.
