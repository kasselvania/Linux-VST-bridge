# SHIELD XL direct physical loopback

The operator connected SHIELD XL outputs directly to inputs and requested a
short Pi-side measurement alongside a direct MOTU loopback in Bitwig.
Base commit: `0d9bbd818a14cbd9ca113b848d4b96939d0bcf26`;
tree: `dc335498ae370a43a2ad474fb266bd6a4c418402`.

Primary claim: measure the native JACK/SHIELD XL physical stereo loop at
48 kHz / 128 frames, with Digitalis excluded from the signal path. Extend the
existing Rust qualification instrument with a three-second loopback mode;
emit a precomputed broadband burst below -40 dBFS and capture sent/returned
audio in preallocated memory. Reject connected physical ports before activation.
Analyze after retirement. No runtime dependencies or product DSP change.

The operator confirmed readiness. Temporarily remove the four existing
Digitalis physical-port routes, run the isolated probe, and restore those
exact routes. Preserve the running editor, plug-in levels, preset, timing and
runtime limits. Keep raw recordings private and retain sanitized results.

Acceptance: Pi build, known-delay/inverted/noisy/absent-signal analysis checks,
actual stereo capture with period/rate/xruns, connected-port refusal, and exact
graph restoration. The result is a physical hardware-loop observation, not
a new full Digitalis round-trip measurement, stress qualification or proof
that a particular Mac application/driver caused its baseline delay.

## Earlier session setup retained for context

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
