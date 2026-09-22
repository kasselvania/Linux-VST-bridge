# RPI1 audio preflight — 2026-09-22

This preserves the initial preflight disposition. The operator then authorized
a short test on the replacement supply already connected, without another
power-supply purchase or swap. The subsequent result and current next step are
in [`RPI1_AUDIO_RESULT.md`](RPI1_AUDIO_RESULT.md).

## Result and remaining boundary

The source-owned stereo tone reached the exact ShieldXL playback ports, measured
nonzero on both channels, and the operator confirmed hearing it. The client
retired with exit zero, no xruns, and no remaining qualification ports or routes.
The deterministic Pigments gate was prepared but **not started**.

During preflight, the Pi logged boot undervoltage while Pigments was absent and
the USB MIDI controller was disconnected. The operator then changed its power
supply; that restart was intentional. The replacement also logged boot
undervoltage. Neither event establishes the cause of the earlier whole-machine
failure. A known fixed 5 V / 5 A supply is the next useful comparison.

The supplied prior-run disposition remains substantial partial success: exact
Pigments load, architecture handshake, state initialization and bus setup,
18,743,296 completed frames (about 390.5 seconds), and a real editor opened,
without a published terminal bridge/host/editor fault. Those are operator-handoff
facts, not rerun here. They do not establish MIDI acceptance, nonzero audio, or
normal retirement.

## Source and scope

- Base head: `6abbcb75bdcd3263412b27ab2320d1b25782dbd7`
- Base tree: `47a6307a0c7432780ce400d01270639e0bc047c8`
- Branch: `experiment/rpi1-pigments-arm64-appliance`; PR #139 remains draft.
- Basis: the operator's immediate Gate 0–2 instructions, `RPI1.md` Primary claim
  and Selected inherited baseline, and the real-time callback laws.
- Narrow claim: make the existing JACK MIDI/audio path objectively observable
  and qualify its hardware playback path before one editor-closed Pigments run.

Changes are limited to a Rust JACK qualification fixture, a bounded independent
Python health recorder, shared JACK observation counters, RPI1 live status, and
this evidence. No Proton, Box64, Windows-host, plug-in, bus-contract, bridge-delay,
activation, or editor-lifetime behavior changed. No state test, soak, editor,
physical controller test, installer, or watchdog recovery was run.

The fixture sends channel-1 note 60 at velocity 96 after 0.5 seconds, note-off at
2 seconds, and CC123 at 2.5 seconds. Its measurement window is five seconds.
It registers one MIDI output and two stereo meter inputs, accepts only the exact
`lvb-arm-pigments` port names and types, and connects both outputs to its meter
and `system:playback_1/2`. It refuses a missing/renamed port, does not start JACK,
and removes only connections it added. Successful JACK event writes are
`midi_sent`; parser acceptance is a separate `midi_accepted` counter. CC123 uses
the inherited parser mapping; this work does not claim Pigments-specific CC123
semantics.

The shared callback observes accepted MIDI and finite nonzero output counts,
per-channel absolute peaks, nonfinite counts, and JACK xruns. Observation uses
bounded loops and scalar atomics; it does not allocate, log, or access files.
RPI1 `status` publishes these beside completed processing blocks and request/result
queue high-water occupancies. These are cumulative independently sampled counters, not an atomic
cross-process snapshot. Final audio counters also go to stdout/journal before
the existing retirement sequence; diagnostic read failure does not prevent
retirement.

## Physical tone result

Fixture: Pi 5 Model B Rev 1.1, ShieldXL/CS4270, kernel
`6.18.50+rpt-rpi-v8`, JACK 1.9.22, 48 kHz, 512-frame period, three periods.
No USB MIDI device, Box64, Proton, or Pigments was running.

| Observation | Left | Right |
| --- | ---: | ---: |
| Measured samples | 144,000 | 144,000 |
| Nonzero samples | 95,998 | 95,998 |
| Nonzero callback windows | 188 | 188 |
| Absolute peak | 0.05 | 0.05 |
| Aggregate RMS | 0.0287709408 | 0.0287709678 |
| Nonfinite samples | 0 | 0 |

The source emitted two seconds of ramped 440/660 Hz audio inside a three-second
window. Exact routes were `lvb-rpi1-qualification:tone_l/r` to both
`lvb-rpi1-qualification:meter_l/r` and `system:playback_1/2`. The meter audits
JACK samples, not an analog hardware loopback. The operator's separate audible
confirmation establishes the physical listening outcome. Tone service runtime
was 3.159 seconds, exit zero, zero xruns, complete port removal.

The physical tone used the initial fixture build. Subsequent fixture changes
were `rustfmt` formatting only; both executable hashes are retained. Final
source hashes match the Pi source copy. The final ARM standalone was built but
was not launched.

## Crash-observability findings

1. The image's journald drop-in forced volatile storage. An isolated override
   now enables persistent storage capped at 64 MiB. A marker survived a normal
   reboot and was read from the preceding boot.
2. Memory accounting and PSI were disabled by boot defaults. The original
   command line was backed up privately; `cgroup_enable=memory cgroup_memory=1
   psi=1` was appended. A second controlled reboot enabled pressure files and
   current/peak memory files on a real user unit.
3. `rpi1/health.py` samples at one Hz for at most 900 samples, outside any Wine
   cohort. It records monotonic time, temperature, throttle flags, supply voltage,
   CPU frequency, available memory/swap, memory/I/O pressure, process counts,
   and exact RPI1 cgroup memory/CPU/process data when present. Missing fields are
   null, never fabricated zero. JACK xruns are available through fixture results,
   RPI1 live status, and retained JACK service output.
4. Ethernet was unavailable; the operator explicitly accepted Wi-Fi SSH.
   Journal/kernel output and ping were retained remotely. A lost Wi-Fi connection
   alone is not a machine-crash signal. During the operator power swap, local
   persisted health ended at monotonic 348.717 seconds; the remote copy retained
   samples through 357.717 seconds. Persistence does not guarantee the last write.
5. The image already enabled a 60-second runtime watchdog. Automatic approval
   review refused a persistent system-wide disable. After explicit operator
   approval, only the runtime manager property was temporarily set to zero and
   hardware state read back inactive. The operator power swap restored its
   original 60-second runtime and 120-second reboot settings. No persistent
   watchdog override was installed. A later diagnostic run must again explicitly
   arrange the approved temporary disable and restore it afterward.

Restore the backed-up boot command line and reboot to roll back kernel telemetry;
remove only the new journald override and restart journald to restore the prior
logging policy. These telemetry settings are intentionally retained for the
next run. The private backup contains machine-specific boot identifiers and must
not be published. No watchdog change remains to roll back.

## Power observations and interpretation

The original USB-C 3 source advertised fixed profiles of 5 V/3 A, 9 V/3 A,
12 V/2.5 A, 15 V/2 A, and 20 V/1.5 A. The Pi reported a 3,000 mA budget and no
high-current USB enable. After the telemetry reboot, undervoltage was recorded
at 6.240 seconds and voltage normalized at 8.256 seconds. Flags were `0x50000`:
historical undervoltage/throttling, with neither currently active when sampled.

The operator's replacement was described as a 30 W 5 V/9 V/15 V device, without
a 5 V current rating. The Pi recorded no PD profiles and retained a 3,000 mA
budget, which does not establish the adapter's full capability. Its boot logged
undervoltage at 8.608–10.624 and 14.656–16.672 seconds. Do not classify this
operator-confirmed power swap as a spontaneous crash.

The 315 retained post-accounting samples before the swap had a maximum
temperature of 51.8 C, minimum MemAvailable of 7,605,628 KiB, and sampled voltage
range 4.88698–4.99284 V. One-Hz samples do not resolve the boot transients or
prove the cause of the original failure.

Raspberry Pi's [USB Power Delivery white paper](https://pip-assets.raspberrypi.com/categories/685-app-notes-guides-whitepapers/documents/RP-009856-WP-1-USB%20Power%20delivery%20on%20Raspberry%20Pi%205.pdf)
states that Pi 5 requests only fixed 5 V profiles and does not support PPS.
A good 3 A supply can run a suitably light load; 5 V/5 A is recommended with
HATs/peripherals. Thus a charger's higher-voltage wattage does not establish the
Pi's available power. The [official 27 W supply](https://www.raspberrypi.com/products/27w-power-supply/)
provides 5.1 V/5 A and is the next controlled comparison, not a proven cure.

## Validation and next action

- 3 fixture tests passed (meter, exact note timing, bounded tone).
- 12 shared standalone tests passed, including allocation-free audio observation.
- 7 RPI1 tests passed locally and on ARM with `jack-runtime`.
- Native ARM release build and warnings-denied fixture compile passed.
- Strict local Clippy found the pre-existing `CallbackGate::new_without_default`
  lint outside this change. Both crates pass with only that lint allowed.
- ARM Clippy was unavailable because that toolchain lacks its Clippy component.

Current result: **hardware tone passed; Pigments deterministic audio pending**.
The Pi remains reachable with no RPI1 units, qualification ports, Box64, or
wineserver processes observed. No new spontaneous failure is claimed.

After a safe shutdown and known 5 V/5 A supply substitution, recheck boot power
and telemetry, then perform exactly one editor-closed Pigments session. Use the
already built qualification client, preserve 512-frame JACK and 2048-frame bridge
delay, retain exact routes/live counters, obtain audible confirmation, quit
normally, and require the existing retirement receipt. Stop there. State, soak,
editor, controller, and final combination remain later gates.

Fixture build on the Pi: `rustc --edition=2021 -O rpi1/qualification.rs -o
qualification`. Run `qualification tone` for the bounded hardware gate or
`qualification pigments` only after exact RPI1 readiness. Both require 48 kHz /
512 frames. Run the standalone itself in its own user unit with journal output
and a private command FIFO; the health recorder belongs in a separate unit.
