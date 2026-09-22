# RPI1 initialization readiness repair — 2026-09-22

**The repaired standalone passed one measured and audible stereo-audio and clean-retirement
session on the already-connected MacBook supply. The operator confirmed:
"Yup! I heard it!"** The earlier audible CC123-repair pass and later startup-timeout failure
are preserved separately. No further physical gate was started.

## Source and scope

Executed head: `8eb89230410a1b42531bfb0fc252cddb166b1d5d`; tree:
`ff138a43d6d9be9c517e8aa0209cf268690fa6c9`. Native SHA-256:
`2689b5a0af961d8637007c6be7a1af58ce55458b26ebb16662c2455007202553`.
All 51 transferred source files matched local hashes before the ARM build.
The result-only documentation commit does not change the tested source identity.

The prior attempt sent Configure after the mapping handshake, while Pigments was
still initializing. That consumed its ordinary 10-second reply budget. The RPI1
standalone now observes the pinned host's existing lifecycle records before
opening its JACK client and sending Configure. This is a pre-activation operation,
with no work added to the audio callback.

The observer follows only the current random session's exact systemd user unit.
It requires a matching commercial-host session, successful initial-state capture,
and the completed inspection record with float32 support and a bounded state
extent. Mapping readiness by itself is insufficient. Inspection completion only
permits sending Configure; the actual response remains the setup authority.

Initialization has its own 120-second limit, a nonblocking pipe, 2 MiB total input
and 64 KiB line limits. The supervisor checks the exact leader PID, start time and
cgroup membership while waiting. Host failure, invalid records, missing state,
observer exit, cohort exit or deadline expiration refuse startup. The observer
child is killed and reaped on every return; existing cohort ownership handles
failure cleanup. The enclosing unit's existing limit remains in force.

The ordinary 10-second Configure reply budget, protocol, audio deadlines, bus
contract, bridge delay, Proton, Box64, Windows host, Pigments and editor behavior
are unchanged. RPI1 adds pinned `serde_json` 1.0.150 to parse the source-owned
records; its resolved dependencies are locked. The prior bounded MIDI repair
remains unchanged.

## Verification

All 12 RPI1 tests passed locally and on ARM with JACK enabled; the ARM release
build passed. The five new tests cover fragmented and wrong-session receipts,
host failures and input bounds, initialization delayed for 10.1 seconds, missing
receipts/cohort loss, and reaping the observer child. Local all-target Clippy passed
with only the pre-existing `clippy::new_without_default` exception. No new hosted
check result is claimed.

The physical session retained 48 kHz, JACK period 512, bridge delay 2048, vendor
quantum 256 and reported vendor latency 48 frames. The MIDI controller remained
unplugged and the editor closed. No machine reboot or supply change occurred in
this attempt. Host inspection completed after a 5739 ms observation, followed by
actual processing setup and readiness. The greater-than-10-second case is proven
by the offline regression; it was not re-induced in this physical session.

The unchanged fixture sent note-on, note-off and CC123. All three were accepted.
Each output measured 240,000 samples, 72,928 nonzero samples, 144 nonzero windows,
peak 0.19765145 and RMS 0.076079156529. Internal output counters agreed. Nonfinite
samples, JACK xruns, callback deadline misses, process failures and terminal
transport faults were zero. Note-off precedes CC123, so this still does not
independently demonstrate active-note CC123 expansion on Pigments.

The host completed 1172 blocks / 300,032 frames, stopped processing, joined without
a worker exception, deactivated and unmapped the endpoint. Native retirement
reported milestones 127, sequence 1173 and position 300032, followed by
`RPI1_CLEAN_SHUTDOWN`; outer exit was 0. Exact sanitized records are retained in
`evidence/rpi1/startup-readiness-audio.json`.

## Power, timing and cleanup

Across 23 one-Hz active samples, temperature peaked at 57.85 C and voltage ranged
4.78112–5.01428 V. Firmware samples remained `0x0`, and the kernel recorded no
undervoltage event. The sampled minimum is retained; absence of warnings does not
establish sustained power margin. The machine remained responsive without a
restart. MemAvailable stayed at least 5,554,516 KiB; outer/translated memory peaks
were 56,246,272 and 2,081,824,768 bytes.

Startup still accumulated 2560 missing/expired frames in one gap episode.
Maximum callback duration was 110,519 ns. This is a functional short-run result,
not glitch-free timing, performance or sustained stability qualification.

The successful session's native/translated processes, transport directory,
command FIFO and experiment JACK ports/routes retired. Independent health and
remote journal observers stopped; the original 60-second runtime watchdog was
restored. The prior inactive outer-unit configuration was restored. Earlier
failed sessions and evidence remain intact. An initial health-recorder path
mistake was corrected and the recorder verified active before the only Pigments
launch; it did not cause another audio attempt.

The next planned functional layer remains editor-closed state round-trip. Repeated
machine-cold-start reliability, state, soak, editor, physical controller, their
combination and overall RPI1 remain open. No installer, authorization or runtime
replacement is indicated by these results.
