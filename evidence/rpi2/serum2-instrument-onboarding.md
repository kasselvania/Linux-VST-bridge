# RPI2 Serum 2 instrument onboarding: partial result

This is a bounded Pi 5/ShieldXL observation on 2026-09-24, not a Serum 2
compatibility qualification. The original editor sessions rendered a white
client area. A later private graphics comparison reached Serum's authorization
screen, but the primary usable-instrument claim remains incomplete.

## Exact identities and setup

- Source branch: `codex/rpi2-serum2-instrument`, based on
  `223e02ddc3c24ef47ef00da7e3af95adb93f4df1` (draft PR #159), tree
  `4656219dca3db9a693f9e3a6dfd96b0634b7cfab`.
- Owner-supplied official Xfer Serum 2.1.5 installer SHA-256:
  `507b726d97bf78920157f3817aff003b9ee38ee961f4efd318cf43216370f695`.
  The owner completed installation in a separate private prefix. The installed
  x64 Windows VST3 module SHA-256 is
  `501e7bb3dd9cafe416b3412df3d4e084c01b7468201e5690b9009d7ecd4e5283`.
  Its bytes match the separately recorded Deck 2.1.5 module; the Pi is a
  distinct fixture and authorization state is not inferred from that match.
- Canonical instrument class: `56534558667350736572756D20320000`; the
  distinct FX class was not selected. Source-owned factory inspection found a
  stereo output, 16-channel event input and output, float32 support, 2,623
  parameters, and zero reported algorithmic latency and tail. The binding
  declares the exact inspected bus wire values and selected parameter IDs.
- Existing native candidate SHA-256:
  `78452d81317aa2a4ac78bf1bc6b8a608250e32188d525e0ccb22f65a84f16895`.
  Existing Windows host SHA-256:
  `4ab203ab8aa25555eebce6782cd52a11643a1b935abe8a2ce0cb04baeb453161`;
  source manifest SHA-256:
  `ab61f36922f01797fdb7702a16dca7f96bb06b459da930ef62c684784b203c69`.
  The launcher used the private pinned GE-Proton11-7-aarch64/UMU 1.4.4 route;
  its SHA-256 was
  `28bfe15ec2fd285f35d0e75a6ea0d754718773d460f497d45c4da8e0e2a69ecb`.
  No working runner or other installed plug-in was replaced.
- 48 kHz, JACK period 512, bridge presentation reserve 512, map-v2 capacity
  512, and Windows processing quantum 256 in every session. Runtime reported
  `jack_frames=512 bridge_frames=512 vendor_frames=0 total_frames=512` and
  `RPI1_LATENCY tail_frames=0`. These are reported digital buffering values,
  not measured analog or MIDI-arrival latency.

## Source-owned tests and physical observations

The focused Rust binding regression passed 1/1. It selects the exact instrument
class and verifies stereo output, event buses and bound parameter IDs. It does
not establish rendering or licensed usability.

The first ordinary Pi session accepted three MIDI messages from the existing
source-owned qualification fixture, captured 240,000 stereo frames at 48 kHz,
and produced finite nonzero audio on both channels (peak 0.28368157, RMS
0.06242436). There were no fixture JACK xruns or bad blocks. This is captured
sound, not an operator-confirmed listening result.

In a separate headless control session, the same note fixture at the default
Main Vol normalized 0.50 measured RMS 0.06242436 and peak 0.28368157. Setting
the inspected Main Vol ID 0 to 0.25 produced controller readback 0.25 and
RMS 0.01571659/peak 0.07092041. A private 44,303-byte opaque state was saved.
In a fresh process, the initial readback was 0.50; restoring that state yielded
readback 0.25 and repeat-note RMS 0.01560609/peak 0.07092039. Both captures
were finite and stereo nonzero. The private state and audio stay off Git.

| Session | Purpose | Processed frames | Missing frames / gaps | Result |
| --- | --- | ---: | ---: | --- |
| `serum-first-01` | MIDI audio and editor | 12,335,616 | 1,792 / 2 | White editor; clean shutdown |
| `serum-dcomp-02` | One renderer preference comparison | 1,973,760 | 512 / 1 | Still white; clean shutdown |
| `serum-state-03` | Main Vol and state save | 2,691,072 | 1,792 / 2 | Control level changed; clean shutdown |
| `serum-recall-04` | Fresh-process state restore | 2,269,696 | 1,792 / 2 | Parameter and level recalled; clean shutdown |

These are completed work and delivery counters at session end, not a
work-normalized CPU comparison. Every session reported zero JACK xruns, zero
processing/callback failures, zero terminal bridge faults and `throttled=0x0`.
Maximum sampled temperatures were 50.15, 50.15, 49.60 and 51.25 C respectively.
The recall session also reported 5,632 paused frames during state restoration.
The available aggregate counters do not give an individual longest gap, so
longest uninterrupted delivery loss is **unavailable**. No gap-free or latency
claim is made.

## Editor gate and cleanup

The actual editor returned successful native lifecycle open/shown events for
two view epochs in the first session. Both the agent's read-only Screen Sharing
view and the operator's observation showed a fully white client area. Closing
and reopening the view did not change it. Pi Tahoma links resolved to the
pinned runner's font files, and the expected Tahoma registrations were present.
One comparison changed only Serum's private `Disable DirectComposition`
preference from true to false for a new process; its editor remained white.
The preference was restored byte for byte to SHA-256
`5e5d1f8482819d32047532d9bc73561364d933864f2f6d7957dbeb9f7f93a173`.
At that stage, no renderer cause was established and no activation screen was
visible. No account or serial operation occurred.

All four original onboarding sessions returned exit code 0, recorded
`RPI1_CLEAN_SHUTDOWN`, left no owned session running and returned JACK to
system-only ports at period 512. Their final power/thermal indicator was
`0x0`. The private prefix, logs, installer, module, state and audio remain
local; no proprietary files or license data are added to this branch.

### Bounded graphics follow-up

The four developer-documented Wine preference values were already effective:
`Disable DirectComposition=true`, `Disable Partial Redraw=true`,
`Show Help Tips=false` and `Show Value Tips=false`. No further preference edit,
font installation or plug-in reinstall was made. The original private launcher
and config retained SHA-256 values
`28bfe15ec2fd285f35d0e75a6ea0d754718773d460f497d45c4da8e0e2a69ecb`
and `fced453fde80b1d43acb7c9858e94c20cf5ffa9979c0f23339a4200d541637e5`.

An exact loaded-module check found Wine built-in `d2d1`, `dcomp`, and
`wined3d`, but the default route loaded DXVK `d3d11` and `dxgi`. Its private
startup trace initially failed Vulkan instance creation. A narrow temporary
Pi package installation added `libdisplay-info2` 0.2.0-2 and
`mesa-vulkan-drivers` 26.2.2-1~bpo13+0~rpt1, with no other package upgrades.
DXVK then saw V3DV but rejected it for missing `multiDrawIndirect`; it skipped
the software Vulkan device and reported `No adapters found` and failed D3D11
device creation. The editor remained white. Package installation alone was not
a rendering repair.

A private Serum-only launcher selected Proton's WineD3D route with
`PROTON_USE_WINED3D=1`. The live host loaded Wine built-in `d3d11` and `dxgi`,
but its hardware-facing OpenGL session still showed white. Adding only
`LIBGL_ALWAYS_SOFTWARE=true` to that private launcher made the actual Serum
2.1.5 authorization page legible in two separate bounded sessions. The second
live host inherited both variables and mapped Wine built-in `d2d1`, `dcomp`,
`d3d11`, `dxgi`, and `wined3d`. On the same X display with the software variable,
Mesa reported `llvmpipe (LLVM 19.1.7, 128 bits)`. The fallback launcher SHA-256
was `e2c2f7baf3d1d581e50992aad6afb2bc9f52711d86afcdfefbc36c6b0558fb03`;
its private config SHA-256 was
`89f53780f4395880f0905755fcdaf64dcf0f641f132b8f65642cbc827ee8d6ba`.
The candidate continued to select the same Serum module, Windows host, native
bridge, 48 kHz, JACK 512, quantum 256 and reserve 512.

The visible page states that this Pi is not yet authorized for Serum 2 and
offers an Xfer browser-based license retrieval flow. No credential, license
or authorization material was entered or captured. This proves first paint
through a private software OpenGL fallback, not a usable editor or acceptable
playback CPU cost. It narrows the white-window failure to the selected graphics
path; it does not identify the first failed WineD3D call. Private graphics
traces, screenshots and vendor data remain off Git.

The graphics sessions did **not** provide a clean-shutdown or audio-use result.
The first 150-second session spent about 122 seconds in first-use startup; its
bounded wrapper timed out waiting for the requested shutdown and stopped the
owned unit. The repeated session reached ready in about 11 seconds, but was
left open for user authorization beyond the Windows-host supervisor's fixed
300-second `RuntimeMaxSec`. The host unit expired. The native bridge then
reported terminal `fault=3`, 8,116 processing failures, 15,360 missing frames
and two gaps before the owned native unit was stopped. There were no MIDI
messages in either graphics session. These failures are retained, not counted
as normal Serum playback or blamed on the renderer without causal evidence.
The second run restored JACK to system-only ports, but had no
`RPI1_CLEAN_SHUTDOWN` marker. A future operator session must be shorter than
the host supervision bound unless that lifecycle policy is separately repaired.
Maximum sampled temperatures in the two graphics sessions were 49.05 and
50.70 C; both observed `throttled=0x0`.

A short final session with the unmodified launcher restored the original DXVK
selection and recorded `RPI1_CLEAN_SHUTDOWN`. All 19 backed-up prefix graphics
paths, including `config_info`, matched their pre-test hash or symlink target.
There were no remaining owned units; JACK returned to system-only ports at
512 frames and power flags were `0x0`. The two temporary Vulkan packages were
removed with no other package changes. Mesa software OpenGL still reported
llvmpipe on the same X display after their removal. The private fallback
launcher/config and sanitized observations remain available for a later,
shorter operator-owned authorization session; the packaged runner, original
launcher/config, Windows host, native bridge and Serum module were unchanged.

Pi CC64 sustain remains unsupported by this adapter. Four useful controls on
a selected user sound, preset selection, actual editor control, audible user
recall, and stable lower JACK periods are not established.
