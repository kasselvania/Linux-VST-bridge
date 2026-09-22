# RPI1 — private Pigments ARM appliance transfer

## Status

- RPI0 functional architecture: **PASSED; PERFORMANCE UNQUALIFIED**
- Steam Deck ASC/Pigments transfer baseline: **RESOLVED**
- exact runtime transfer: **PASSED**
- Proton-on-Box64 core Pi process checks:
  **FUNCTIONAL; SETUP FAULTS PRESERVED**
- account-free UI Automation normal/override checks: **PASSED**
- Proton-on-Box64 Phase A preflight:
  **PASSED WITH PRESERVED SETUP FAULTS**
- exact ASC 2.12.0.3157 installer admission: **PASSED; NOT EXECUTED**
- private ASC installation and sign-in: **PASSED IN PRIOR OPERATOR HANDOFF;
  NOT REPEATED IN THE CENSUS DIAGNOSIS**
- private Pigments installation: **EXACT MODULE REVALIDATED ON PI**
- Pigments factory/class, bus, parameter and state-capture census:
  **PASSED ON PI; LAUNCH-PATH DEFECT DIAGNOSED**
- prior private Pigments standalone: **SUSTAINED PROCESSING REPORTED;
  AUDIO AND COMPLETE RETIREMENT UNPROVED**
- source-owned ShieldXL stereo tone: **MEASURED AND HEARD**
- deterministic editor-closed Pigments audio after bounded CC123 repair:
  **MEASURED AND HEARD; NORMAL RETIREMENT PASSED**
- final MacBook-supply comparison after reboot: **STARTUP CONFIGURE TIMEOUT;
  NO AUDIO STARTED; NO RECORDED UNDERVOLTAGE**
- subsequent initialization-wait repair: **MEASURED AND AUDIBLE STEREO AND CLEAN
  RETIREMENT PASSED; NO RECORDED UNDERVOLTAGE**
- sustained power margin and performance: **UNQUALIFIED; NEW TRANSIENT
  UNDERVOLTAGE AND STARTUP GAPS PRESERVED**
- overall RPI1: **PENDING; REPEATED COLD START AND LATER GATES REMAIN**

RPI1 is not a greenfield Wine experiment. It transfers the accepted Steam Deck
Arturia/Pigments runtime contract onto the accepted RPI0 hardware path and tests
only the new ARM/Box64 and appliance boundaries. A separate Ubuntu attempt that
did not reproduce the complete Steam Deck contract is retained as environment-
drift evidence, not selected as the RPI1 baseline.

This experiment is isolated from the ordinary repository slice in
`CURRENT_SLICE.md`. Do not edit that file, install this experiment into an
ordinary product environment, or merge this experiment automatically.

The current census diagnosis and next implementation boundary are recorded in
[`RPI1_CENSUS_RESULT.md`](RPI1_CENSUS_RESULT.md) and
`evidence/rpi1/pigments-census-diagnosis.json`. Earlier transfer/admission records
remain historical observations; their original `NOT_RUN` fields are not rewritten.

The September 22 audio preflight, narrow observation additions, power findings,
and exact next boundary are recorded in
[`RPI1_AUDIO_PREFLIGHT.md`](RPI1_AUDIO_PREFLIGHT.md) and
`evidence/rpi1/audio-preflight-power.json`. The later operator power swap is an
operator-triggered restart, not a newly reproduced spontaneous crash.

The operator subsequently authorized testing the already-connected replacement
supply. One editor-closed Pigments session produced measured and audible stereo,
then exposed an inherited reference-fixture CC123 parameter mapping defect.
See [`RPI1_AUDIO_RESULT.md`](RPI1_AUDIO_RESULT.md) and
`evidence/rpi1/pigments-deterministic-audio.json`. No new voltage event or machine
restart occurred during that short session. It does not qualify sustained power
margin or erase the earlier boot undervoltage observations.

The operator then authorized the narrow MIDI repair and one repeat of the same
gate on the same supply. The repaired run produced measured and audible stereo,
accepted all three MIDI messages without a terminal fault, and retired normally.
See [`RPI1_CC123_REPAIR.md`](RPI1_CC123_REPAIR.md) and
`evidence/rpi1/pigments-cc123-repair.json`. A new brief kernel undervoltage event
recovered during this successful run. Startup gaps remain; neither sustained
power nor performance is qualified. No later physical gate was started.

The operator subsequently requested one final short comparison using a MacBook
power supply. The same binary, after the requested shutdown and supply swap,
failed its initial Configure reply deadline before audio readiness. Pigments
component initialization and initial state capture completed, but consumed the
ordinary control-response budget. No recorded undervoltage or restart occurred
during this attempt. See [`RPI1_POWER_COMPARISON.md`](RPI1_POWER_COMPARISON.md)
and `evidence/rpi1/power-comparison-startup-timeout.json`. No retry was made;
the next repair is bounded startup readiness, before the later appliance gates.

That subsequently authorized repair now waits for the exact host's completed
initial inspection before sending Configure. One fresh-process session passed
measured and audible stereo and normal retirement on the same MacBook supply without any
recorded undervoltage or throttle flags. This attempt did not include another
machine reboot; repeat cold-start reliability remains open. See
[`RPI1_STARTUP_READINESS.md`](RPI1_STARTUP_READINESS.md) and
`evidence/rpi1/startup-readiness-audio.json`.

## Exact starting custody

- branch: `experiment/rpi1-pigments-arm64-appliance`
- starting head: `4a88802914d33633adb1b8b8c76507f0883e63d3`
- starting tree: `d1f7ebc4e0fd4d6e3e2ca342f6e72103dd043325`
- RPI0 repaired executable source:
  `791088bb31fcd75212b2df4ac2c7a6efb95182ba`
- RPI0 repaired executable tree:
  `10f102fe04ce52a5a9436481331007146b85ebba`
- accepted AP18 ancestry:
  `3fa410fe95b790b9174417f803c56a32b2b3d2d2`
- accepted ordinary Pigments revision-18 ancestry:
  `b1f1aed7b7f8857da9f6860252f11a44996a28bf`

Both Arturia branches are ancestors of the RPI0 starting head. RPI1 must reuse
that integrated source rather than cherry-picking an older or partial Arturia
implementation.

## Primary claim

On the exact Raspberry Pi 5 and ShieldXL fixture, a private user-owned Pigments
installation can be acquired and authorized through the real Arturia Software
Center, then run as an x86-64 Windows VST3 through the native AArch64 standalone
host with physical MIDI, stereo audio, the real mouse-operable editor, state
restore, exact cleanup, and a fresh restart.

No DAW is required or launched.

## Selected inherited baseline

RPI1 inherits, but does not automatically re-claim, the following exact Steam
Deck results:

- Pigments profile `arturia-pigments`, revision 18,
  `verified_exact_fixture`;
- class `41727475415649534B61743150726F63`, Pigments 7.0.1.6772;
- Deck module SHA-256
  `bdc91ebef8e5b486c8f998f1eef6a99626dd5a0b46d986263eeb1980b96a3c07`;
- Proton 11.0-2c build 25118279 with Steam Linux Runtime 4
  4.0.20260805.254769;
- process-scoped vendor retirement;
- retained editor view until instance retirement;
- reported-zero event-output compatibility policy;
- Windows accessibility disabled for the vendor process;
- detached direct vendor editor lifecycle;
- concurrent read-only state capture v12;
- float32 processing;
- 512 bridge frames recommended on the Deck, with 256 unqualified there.

The transfer baseline is machine-checked by `rpi1/validate_transfer.py` against
`rpi1/transfer-baseline.json`. The Deck module digest is an identity reference,
not permission to copy that commercial binary into Git or another public store.

## Runtime transfer decision

The primary RPI1 runner is the exact accepted Deck Proton closure:

```text
proton-11.0-2c-25118279-slr4-4.0.20260805.254769
```

Its exact runner and entry-point digests are retained in the transfer manifest.
Acquire the bytes only from the user's retained installation or the exact
official Steam depot identity, verify the complete declared closure, and keep it
private. Do not substitute an ambient `latest`, GE-Proton, the incomplete Ubuntu
configuration, or a similarly named Wine build.

RPI0's pinned Box64 remains the outer x86-64-on-AArch64 translation layer:

- Box64 commit `2f130fab1d6e1a4ee8a71dc60cfdfcc839ad192a`;
- executable SHA-256
  `79cdd30e5480f5dfb8cd717af99d0e5a89eb55b31701de6fde7896a6a0e79ab6`.

RPI0's source-built Wine 11 remains a control and recovery tool. It is not the
selected replacement for the Deck Proton contract. If the full Proton/SLR launch
shape cannot run on ARM, retain the exact failed boundary before selecting any
narrow Proton-Wine-under-Box64 adaptation. Do not silently call that adaptation
the original Proton environment.

## Pi environment

Use a new private Pi-owned Arturia environment. Keep its machine identity,
prefix, runtime identity and ownership stable across the campaign. Do not copy
Steam Deck account state, cookies, tokens, activation payloads or a runnable
licensed prefix. Configuration and publicly recordable runtime contracts may be
transferred; account and authorization interaction remains operator-owned.

The first fixture remains:

- Raspberry Pi 5 Model B Rev 1.1, 8 GB;
- Raspberry Pi OS Trixie / Debian 13.7, AArch64;
- 4 KiB `6.18.50+rpt-rpi-v8` kernel;
- ShieldXL CS4270/JACK at 48 kHz;
- no active cooling and no overclock;
- X11 through the established local-only VNC/SSH route;
- physical Monolit MIDI unless the operator selects another controller.

The RPI0 `0x50000` historical undervoltage/throttling register remains a
performance blocker. Record temperature, current/sticky throttle bits and clock
state throughout RPI1.

## Phase A — exact runner transfer and account-free preflight

### Runner acquisition checkpoint

Valve's signed macOS SteamCMD was pinned, inspected and run with anonymous
login against the exact recorded Proton app/depot/manifest. Public connection
and anonymous login passed, but Steam refused the depot with missing-license /
no-subscription. No Proton bytes were downloaded and no credentials were
requested or inspected. The exact runner must therefore come from the user's
entitled retained Steam Deck installation. This is an acquisition boundary, not
a Proton, Box64 or Pi execution failure. See
`evidence/rpi1/runner-acquisition.json`.

### Runtime transfer and Pi preflight checkpoint

The user's retained Deck installation supplied only the exact Proton 11.0 and
Steam Linux Runtime 4 directories. The transfer excluded app manifests, Steam
userdata, compatdata, Arturia software and all account or authorization state.
Complete path/type/mode/content/symlink/hard-link closure digests matched first
in private staging and then on the Pi:

- Proton: `b2fa64a44abe0db6b1aae35c6a402c0e4a4f9603a208c6dcf4b936bc1dd4f9fb`;
- SLR4: `0ca1ee5a23ec82489fb1305ffcd2eed62723bc126c37838934fc037d5e10fca3`.

SLR4 contains its native AArch64 pressure-vessel implementation, but its
emulator interface invokes the configured emulator with explicit x86-64
dynamic-loader vectors and invokes the top-level Proton Python script through
that same boundary. Direct Box64 failed both shapes. The experiment-owned
`rpi1/box64-emulator-adapter.c` now performs only those two reviewed
translations and refuses malformed or unknown loader vectors. It does not
replace SLR4, Proton, Wine or the bridge.

With that adapter, the exact Pi completed the x86-64 Linux probe, fresh Proton
prefix initialization, the source-owned Windows self-test, and the
source-owned Windows VST host start/natural-close lifecycle. Every launch used
an exact systemd user unit; the final host PID/start identity was observed in
its cgroup and zero private descendants remained after retirement.

This is not a clean preflight. SLR4 still runs an i386 setup probe that the
selected 64-bit-only Box64 does not support, its static x86-64 `ldconfig`
helper exits by signal 11 before pressure-vessel selects its declared
`LD_LIBRARY_PATH` fallback, and several optional native wrappers are absent
from the Pi image. The successful markers do not erase those faults. Exact
attempts, hashes, thermal observations and nonclaims are retained in
`evidence/rpi1/runtime-transfer-preflight.json`.

The exact source-owned UI Automation fixture then ran through the same
SLR4/Proton/Box64 path in two bounded systemd user units. Normal execution
caught the intended access violation and returned the fixture's expected exit
42. The otherwise-identical process with operation-local
`WINEDLLOVERRIDES=uiautomationcore=` returned the expected exit 0. Both units
retired with zero private processes. This passes the account-free comparison
and completes Phase A with the preceding setup faults still preserved; it does
not prove ASC behavior or repair the runtime. Exact fixture, log and cleanup
facts are retained in `evidence/rpi1/account-free-uia-preflight.json`.

Before executing ASC or commercial plug-in bytes:

1. Verify the complete exact Proton runner closure against the revision-18
   profile.
2. Establish the Box64 launch topology without changing the accepted RPI0
   installation.
3. Run the x86-64 Linux probe.
4. Initialize a fresh Pi-specific Proton prefix through the accepted Proton
   initialization path.
5. Run the source-owned Windows probe.
6. Start and close the existing source-owned Windows VST3 host.
7. Establish exact unit/cgroup/process/session ownership and cleanup.
8. Run the account-free UI Automation fixture both normally and with the
   process-local `WINEDLLOVERRIDES=uiautomationcore=` policy.

Failure at any step is retained with the exact runner, Box64 and process cohort.
Do not proceed to ASC merely because `wine --version` works.

## Phase B — ASC

Use the official user-supplied Windows ASC installer. The known accepted Deck
installer identity is retained for comparison, but RPI1 may use a newer exact
official version only after recording that explicit fixture change.

The user supplied `Arturia_Software_Center__2_12_0_3157.exe`. Its size and
SHA-256 exactly match the accepted Deck installer, so no fixture-version change
occurred. The same bytes were verified after SSH transfer into the Pi's private
RPI1 installer staging directory and left mode `0400`. No installer or Arturia
process was launched during admission. See
`evidence/rpi1/asc-installer-admission.json`.

Required sequence:

1. Bound immediate installer candidates; hash and identify without execution.
2. Create a private inactive rollback snapshot of every allowed mutable root.
3. Launch the selected installer in one exact experiment-owned cgroup.
4. Verify installed ASC main/Agent/updater identities and normal relaunch.
5. Launch ASC with the demonstrated process-local accessibility policy.
6. Pause for operator login, MFA, consent, installation and authorization.
7. Never inspect credentials, account identifiers, cookies, tokens, serials or
   licensing payloads.
8. Retain only non-secret installation, process, version and cleanup facts.
9. Require normal close or exact bounded cleanup before continuing.

The process-local accessibility policy is inherited from the accepted Deck
result, not rediscovered from the incomplete Ubuntu environment.

## Phase C — Pigments discovery and appliance qualification

After the operator installs and authorizes Pigments through ASC:

1. Discover the exact module inside the selected Pi environment.
2. Hash it and perform supervised factory/class census.
3. Require the exact Pigments class rather than selecting by filename.
4. Compare version, buses, parameter count, precision, latency, editor and state
   facts with the transfer baseline; retain differences rather than forcing the
   old profile.
5. Bind the observed class/module and accepted Pigments compatibility policies
   into the ARM standalone host.
6. Start at 2,048 bridge frames on the 48 kHz ShieldXL graph.
7. Test physical notes, distinct velocity where available, release, sustain and
   all-notes-off.
8. Establish audible and measured nonzero stereo output.
9. Open the real editor, make one physical-mouse parameter change, close while
   DSP continues, and reopen on the same instance.
10. Load one factory preset through Pigments, save state, change it, restore it,
    and establish the intended audible and visible state.
11. Stop normally and prove exact cohort/cgroup/session/mapping/editor cleanup.
12. Start a second fresh session, play, stop and prove cleanup again.

Only after the complete 2,048-frame run may RPI1 try 1,024 and 512 bridge frames.
The Deck's recommended 512 setting is transferable prior art, not a Pi result.

## Hard boundaries

Do not:

- launch Bitwig or another DAW;
- modify `CURRENT_SLICE.md`;
- modify, merge or relabel RPI0;
- copy the Steam Deck prefix or machine-bound activation state;
- commit ASC, Pigments, Proton runtime bytes, presets, paid content or account
  data;
- automate or inspect sensitive Arturia account interaction;
- use global `wineserver -k`, `pkill` or broad process-name cleanup;
- reinterpret an arbitrary Wine configuration as the accepted Proton baseline;
- claim Pigments, ASC, performance, latency, polyphony, sustained stability,
  production readiness or general ARM support before their exact gates pass.

## Handoff

Retain exact source/head/tree, runner/Box64 identities, installer/product/module
identities, Pi fixture state, deterministic validation, every physical step,
editor/state/audio results, timing/gap/fault/resource/thermal observations,
cleanup/restart, failures and nonclaims in one draft experimental PR. Do not
merge it.
