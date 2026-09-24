# Digitalis v1.1 Windows VST3: bounded Pi 5 effect check

Date: 2026-09-24. Base: `7cbf2490c601884ed84ef15ac79922c89ff22830`
(observer repair, draft PR #156). Fixture: Pi 5 / ShieldXL, pinned
GE-Proton11-7 ARM64EC route, 48 kHz JACK with 512-frame periods, 256-frame
vendor processing quantum, 2,048-frame bridge reserve. No FEX or working
runner change was made.

## Exact identities and staging

| Item | SHA-256 |
| --- | --- |
| Owner-supplied `Digitalis-v1.1-Windows.zip` | `c25e0f8682ddf57cb7bd7d3992dba45de223d58de1d0881b2287239638f78ff9` |
| Enclosed setup executable | `5a596efce6a864e83e0436dc9edfe3f2a75950348841eb6a4dac6560ca900d95` |
| Windows x64 VST3 module | `fb51eec9bda65f5c8ac3f4c7f9a99f6fc0034df3cd54fc1b54e32e598e2af335` |
| Private Windows host candidate (source `6f9d292`) | `4ab203ab8aa25555eebce6782cd52a11643a1b935abe8a2ce0cb04baeb453161` |
| Private native candidate | `78452d81317aa2a4ac78bf1bc6b8a608250e32188d525e0ccb22f65a84f16895` |
| Committed binding | `319861acc66940576e0573a6463948d967436bb5689d3958a5c25ebee0a2c12a` |

The owner obtained the paid archive from Aberrant DSP My Account. Aberrant
states that purchased plug-ins have no DRM or activation step. No serial was
entered or collected. The module did not report an edition identifier, so
full-versus-demo is based on the owner's purchase/download provenance, not an
independent binary edition flag or a test of the demo's timed mute behavior.

The official installer was attempted in the separate private environment and
ended with `Error: Path not found`; it did **not** complete. The VST3 module
and 193 factory content files were instead extracted from that exact official
archive into the private environment. This proves private staging and use,
not a managed installer flow. No vendor binary, content, serial, state, or
audio is retained in Git.

## Factory and host boundary

The factory exposed processor class
`ABCDEF019182FAEB4162726E44696769` and separate controller class
`ABCDEF011234ABCD4162726E44696769`. Its selected main input/output are
stereo; there is no event bus. Float32 processing was accepted, float64 was
refused. Reported algorithmic latency was 4,096 frames and tail zero.
The binding uses `lvb-arm-plugin-binding/v1` and actual automatable IDs:
Bitcrush Ratio `1095041790`, Master Mix `211308698`, Bitcrush Mix `93746784`,
and Corruption Section Bypass `1322828117`. The physical surface has three
encoders and a button for these selected controls. Host-owned linear output
trim and current-input dry bypass use the existing callback path.

Digitalis returned untouched `kNotImplemented` for controller
`setComponentState`. The generic host accepts that narrow response and
checks saved parameter readbacks after restoring component state. An actual
stale readback fails the source-owned state test. Digitalis also refused
explicit main-bus deactivation after component deactivation; final Close
retires the deactivated component, while reactivation retains the stricter
bus-deactivation requirement. Live in-process state replacement was not
validated and is not claimed.

The exact `6f9d292` Windows host-only workflow succeeded, including
`ap11-state-tests` (`36034559978`). With the committed four-control descriptor
staged, the Pi release native `rpi1/standalone` suite passed 26 library and 4
binary tests. An initial test run against a stale private three-control
descriptor failed only its exact binding-count assertion; the staged test
input was corrected to the committed SHA above before the passing rerun.
The Pi release `rpi0/standalone` suite also passed 22 library tests, including
allocation-free trim/bypass presentation.

## Physical observations

The initial physical-input active/bypass pair completed two five-second
stereo captures with finite nonzero output, zero helper xruns and bad blocks,
and clean shutdown. Dry bypass output equaled the captured physical input for
all 480,000 interleaved samples at the JACK observation points. The initial
effect setting was effectively subtle: after a 6,144-frame alignment the
active output differed from the input by only 0.0002215 RMS. The operator did
not hear a change, so this pair is **not** evidence of an audible effect.

For a direct sound check, the original physical inputs were routed to the
same Digitalis instance, its stereo outputs to system playback, and the vendor
editor opened. The operator adjusted the sound and confirmed that the live
effect was clearly audible. The host saved a 74,491-byte private slot. On a
fresh process with the editor closed, `RPI2_PANEL_RESTORED bytes=74491`
preceded readiness; selected parameter readbacks matched the saved setting
(section value 1, ratio about 0.2, bitcrush mix 1, master mix about 0.62).
The operator listened to the rerouted physical input and confirmed the same
effect returned. These are operator-confirmed audible and restarted-recall
outcomes; no vendor preset or audio is published.

| Session | Editor | Exit / retirement | Delivery and safety |
| --- | --- | --- | --- |
| Source-owned input pair | Closed | Exit 0, clean, system-only JACK graph | 0 xruns, 0 missing frames, 0 gaps, 0 processing/callback failures; max 50.15 C, no warning |
| Live sound edit/save | Open by request | Exit 0, clean, system-only graph | 0 xruns, 0 missing frames, 0 gaps, 0 processing/callback failures; max 52.9 C, no warning |
| Fresh headless recall/play | Closed | Exit 0, clean, system-only graph | 0 xruns, 0 missing frames, 0 gaps, 0 processing/callback failures; max 51.8 C, no warning |

The final live edit delivered 10,006,528 frames after 2,048 priming frames;
the fresh recall delivered 2,129,920 after 2,048 priming frames. Historical
`request_high` and `result_high` values are queue high-water marks, not current
backlog. Private logs retain the earlier failed installer and host attempts.

## Latency and limits

At 48 kHz, the vendor's 4,096 reported latency frames are 85.33 ms and the
2,048-frame bridge reserve is 42.67 ms. The reported combined 6,144 frames
are 128 ms. The first active physical capture correlated at 6,144 JACK frames;
the current-input host bypass had zero relative JACK-sample offset. No analog
output-to-input loopback measurement was made, so these are **not** analog
pedal latency measurements or a claim of sub-10-ms operation. The auxiliary
sidechain and general multi-I/O configurations remain unsupported by this
selected stereo binding. The working runner, other plug-in environment, and
original artifacts remained separate; no private candidate was installed
globally.
