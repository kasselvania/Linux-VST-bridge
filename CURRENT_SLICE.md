# RPI2 Serum 2 instrument onboarding

Base: `223e02ddc3c24ef47ef00da7e3af95adb93f4df1` (draft PR #159).
Base tree: `4656219dca3db9a693f9e3a6dfd96b0634b7cfab`.
Branch: `codex/rpi2-serum2-instrument`, isolated from Digitalis timing and the canceled multicore/UI checkout.

Basis: `AGENTS.md` identity, real-time, licensing and slice laws;
`GOVERNANCE.md` “What evidence means”; `docs/ARCHITECTURE.md` host,
controller, state and editor boundaries; `docs/AP8_RESULT.md`, `docs/SV1.md`,
`docs/LC2.md`, and `docs/AP11.md` for the separate Steam Deck sequence; the
operator's ARM appliance handoff and instruction to proceed with Serum 2.

## Primary claim and fixture

Establish one usable Serum 2 **instrument** on the Pi 5/ShieldXL through the
existing generic native host and pinned ARM64EC Windows route: exact class,
MIDI note audio, same-instance editor, selected controls, vendor state recall
after process restart, and normal shutdown. The owner-supplied Xfer installer
is version 2.1.5, SHA-256
`507b726d97bf78920157f3817aff003b9ee38ee961f4efd318cf43216370f695`;
the installed Windows x64 VST3 module is SHA-256
`501e7bb3dd9cafe416b3412df3d4e084c01b7468201e5690b9009d7ecd4e5283`.
The distinct Serum 2 FX class remains inventory only. Start at 48 kHz,
JACK 512, processing quantum 256 and bridge reserve 512; record
actual values at runtime.

## Scope and acceptance

Keep the Xfer installation in its separate private environment. Derive the
instrument binding from the actual 2.1.5 census, including canonical class
ID, stereo output, event buses and real parameter IDs. Reuse the existing
generic Windows host, native observer, parameter/controller queue and state
protocol. Preserve one exact host/source manifest pair. The operator handles
any vendor account or activation screen; never record credentials or license
material.

Qualify a selected, vendor-generated sound in one bounded session: note-on/off
with finite nonzero stereo output and release, actual editor manipulation,
four useful controls where the selected sound supports them, an opaque private
state save, fresh-process restore with editor closed, and user-confirmed
audible recall. Report the runtime identity, completed frames, timely delivery,
missing frames, longest loss, JACK xruns, processing/callback faults, temperature
and power flags separately. Verify exact retirement and JACK graph restoration.
The installed module and successful inspection alone do not satisfy this claim.

## Negative gates and nonclaims

Stop on changed authorization/readiness, wrong class, incorrect state, terminal
fault, current power warning or 75 C temperature. Retain original failures.
Do not call the Pi supported from Steam Deck evidence, an editor window,
process liveness or meter movement alone. Touchscreen, FX, multiple instances,
full MIDI controller support, low latency, stable 128/256 JACK operation, FEX
tuning, multicore, and universal vendor compatibility are outside this slice.
The present Pi MIDI adapter reports sustain CC64 unsupported; report that gap.

No proprietary installer, binary, state, preset, account data or private audio
is committed. Keep the working runner, Digitalis environment, other plug-ins
and canceled checkout intact. One owner operates the Pi. Sanitize retained
source and physical evidence, commit and push one focused draft PR, and leave
it open and unmerged for review. Any incomplete acceptance gate remains explicit.

## Observed disposition

The exact instrument class was inspected, bound and run through the generic
host. Source-owned MIDI notes produced finite stereo audio. Main Vol parameter
ID 0 changed the measured output level, and its opaque state restored with the
same parameter readback and output level in a fresh process. All four bounded
sessions shut down normally and restored the JACK graph. The editor opened and
reported successful lifecycle events, but its content remained completely
white in the original session, after close/reopen, and in one reversible
DirectComposition preference comparison. The preference was restored byte for
byte. A later private Serum-only WineD3D plus software OpenGL comparison made
the actual Xfer authorization page legible in two bounded sessions; this Pi
reported itself not yet authorized. No account or license material was entered.
Actual editor manipulation and licensed usability are therefore still
unverified. Each original session recorded at least one delivery gap, despite
zero JACK xruns, processing failures and terminal faults. The later long
graphics session exceeded the Windows host's 300-second supervision bound
and produced a real terminal fault and processing failures before cleanup.
The primary usable instrument claim is **not complete**; no Pi Serum
compatibility assertion or
deployment promotion follows. Exact identities, results and limits are in
`evidence/rpi2/serum2-instrument-onboarding.md`.

The later operator session again showed Xfer's authorization page, but OK did
not open a browser in this Pi environment. No browser handler was installed.
One operator-approved offline-authorization trial was canceled before opening
the editor; its temporary route change and prefix graphics selection were
restored and verified. The operator then requested the normal browser path.
Firefox ESR, the desktop portal and an explicit per-user HTTPS association now
let Serum's OK button open Xfer's sign-in page on the Pi. A private operator
candidate omits the Windows host's 300-second service lifetime only when
`LVB_RPI1_OPERATOR_SESSION=1`; the original candidate remains staged and
unchanged. The live owner session has no clock timeout and retains thermal,
power and terminal-fault stops. At that checkpoint, sign-in remained
owner-controlled and incomplete; the primary usable-instrument claim was not
established.

The owner later reported using the loaded Serum editor and encountering a
session close on a preset change. The private log identified a native selected
parameter readback refusal after refresh; unlike the first Deck preset failure,
this Pi session showed no processing restart or lifecycle correlation error.
The focused native repair accepts changed display metadata under the same
parameter ID and represents an explicitly unavailable value as missing. A
separately staged Pi candidate then loaded one new factory preset, reported
changed metadata for selected parameter ID 7000000, kept the editor open and
processing, and shut down normally. This is a preset-transition result only:
the original owner-selected sound was not identified, the candidate test sent
no MIDI, and three delivery gaps remain recorded. Details and exact identities
are in `evidence/rpi2/serum2-preset-readback.md`.

A later owner session connected the OMX-27 USB MIDI port to this Serum candidate
and routed its stereo output to ShieldXL. The initial sound was audible, but
after the owner selected another preset the Pi output fell silent while the
Serum editor meter still moved. Processing progress stopped, the request queue
filled, and the bridge reported terminal `fault=2` at `publish_request`.
The session was stopped, with no clean-shutdown claim. This is a separate
failed playable-preset gate; its cause is not yet established. The sanitized
result is in `evidence/rpi2/serum2-instrument-onboarding.md`.
