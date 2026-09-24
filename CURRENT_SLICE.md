# RPI2 Digitalis effect onboarding

Base: `7cbf2490c601884ed84ef15ac79922c89ff22830` (draft PR #156).
Base tree: `c7f84e39cf7768afeaaecc79e99707964cc1a648`.
Branch: `codex/rpi2-digitalis-effect`, isolated from the canceled multicore/UI checkout.

Basis: `AGENTS.md` mission, identity, real-time, privacy and slice laws;
`GOVERNANCE.md` “Decisions”, “What evidence means”, and “Cost and safety”;
`docs/ARCHITECTURE.md` host/process and state boundaries; and the operator's
2026-09-24 device-agent handoff. The later operator direction supersedes the
old RPI2 CPU task on this new branch.

## Primary claim and fixture

On the Pi 5/ShieldXL fixture, onboard the owner's purchased Digitalis v1.1
Windows x64 VST3 as a headless, controllable stereo effect through the existing generic Windows
host and `lvb-arm-plugin-binding/v1`. The operator supplied
`Digitalis-v1.1-Windows.zip` (SHA-256
`c25e0f8682ddf57cb7bd7d3992dba45de223d58de1d0881b2287239638f78ff9`);
the enclosed setup executable has SHA-256
`5a596efce6a864e83e0436dc9edfe3f2a75950348841eb6a4dac6560ca900d95`.
Use the existing pinned GE-Proton11-7 ARM64EC Windows route, the repaired native
observer, 48 kHz, JACK 512 frames, vendor quantum 256 and 2,048-frame reserve.
Record the actual installed module/class, edition and authorization posture
after installation rather than inferring them from the archive name.

## Scope and acceptance

Privately stage the installer in a distinct environment, retaining the
working runner, Arturia environment and original artifacts. Use the vendor's
authorization UI only; the operator enters any serial there. Do not commit or
log the serial, installed module, state, audio, account data or installer.
Inspect the actual factory/class, bus, precision, parameter, latency and tail
metadata. Bind a few confirmed controls through the existing generic binding
and parameter/controller queue. Add only narrowly required shared host behavior
for output trim and a defined bypass policy; keep callback work bounded.

Run the existing source-owned stereo input capture and the physical input route
with the editor closed. Demonstrate finite processed audio and bypass behavior,
one actual audible control change, one private state slot saved and restored
after process restart, normal shutdown, and restored original JACK graph.
Report exact buffer settings, algorithmic latency, measured dry-path/analog
latency where available, missing delivery and longest interruption. Distinguish
intentional effect chopping from transport loss. Record product/module identity
and full-versus-demo status. If a full licensed build cannot be made ready,
state that gate explicitly and nominate the free Kilohearts Delay for the
first-effect slot.

## Non-goals and failure posture

No FEX optimization, Pigments performance campaign, multicore/UI change,
NTSync/kernel work, format rewrite, universal multi-I/O claim, vendor code
patch, preset sweep, licensing workaround or global runner install. A demo's
intentional dropout and save limits are not bridge faults. Stop on altered
authorization/readiness rather than repairing the vendor's licensing state.
Stop at 75 C or current thermal/power warnings. Preserve private logs and
artifacts; retain sanitized evidence only. Restore original graph and setup.
One physical owner operates the Pi. Deliver one focused draft PR, then leave
it open and unmerged for review.

## Observed disposition

The official installer stopped with `Error: Path not found`; successful
installer supervision is not claimed. The exact owner-supplied archive was
extracted into the isolated environment, and its module and factory content
were used. Aberrant DSP documents no activation step for purchased plug-ins;
the owner supplied the paid My Account download, and no serial was used. The
module did not expose an independent full/demo edition flag.

The generic stereo host, selected control binding, finite input/output,
current-input dry bypass, output trim, private sound-slot save, fresh-process
headless restore, and normal shutdown were exercised. The operator heard a
clear live-editor effect change and confirmed the same sound after restart
with the editor closed. All three successful physical sessions reported zero
JACK xruns, missing delivery, processing failures, callback failures, and
terminal bridge faults. The failed initial installer and host attempts remain
in private logs and are not counted as successes.

The reported vendor latency was 4,096 frames and bridge reserve 2,048 frames
at 48 kHz; analog loopback latency was not measured. The 128-ms combined
reported buffering is not a low-latency pedal claim. Private extraction is
not a completed managed installation path. Exact fixture identities, source
tests, physical readings, cleanup, and nonclaims are retained in
`evidence/rpi2/digitalis-effect-onboarding.md`.
