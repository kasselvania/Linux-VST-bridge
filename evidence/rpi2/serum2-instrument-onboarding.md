# RPI2 Serum 2 instrument onboarding: partial result

This is a bounded Pi 5/ShieldXL observation on 2026-09-24, not a Serum 2
compatibility qualification. The primary claim in `CURRENT_SLICE.md` remains
incomplete because the actual editor rendered a white client area.

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
No renderer cause is established. No activation screen was visible; license
posture is unverified and no account or serial operation occurred.

All four owned sessions returned exit code 0, recorded `RPI1_CLEAN_SHUTDOWN`,
left no owned session running and returned JACK to system-only ports at period
512. The final power/thermal indicator was `0x0`. The private prefix, logs,
installer, module, state and audio are retained locally; no proprietary files
or license data are added to this branch. Pi CC64 sustain remains unsupported
by this adapter, and four useful controls on a selected user sound, preset
selection, actual editor control, audible user recall, and stable lower JACK
periods are not established.
