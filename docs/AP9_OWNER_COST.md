# AP9 owner-cost follow-up

PR #69 review [5127122244](https://github.com/kasselvania/Linux-VST-bridge/pull/69#pullrequestreview-5127122244) identified repeated descriptive process reads in the running Python supervisor. This follow-up reduces that measured cost without changing audio binaries, supervision cadence or broker design. Original AP9 source and evidence remain attached to their original results; AP8/#67 is unchanged.

## Demonstrated work and the change

A separate 12-second, eight-note Serum run profiled the actual owner thread around `supervise → descendants → process_census`. Across 83 calls, the collector attempted 30,462 `stat`, 30,455 `cmdline` and 30,455 `comm` opens, about 367 visible processes per census. `process_census` accounted for 6.206 seconds of cumulative profiler time; pathname construction, opens and reads dominated. The wrapper measured 7.866 seconds of thread CPU. Profiling and the audit hook perturb execution: these times locate unnecessary work and are **not** the uninstrumented CPU baseline. The profile run itself retained zero bridge gaps and positive cleanup.

Ongoing tracking now enumerates fresh `/proc` identities from `stat` only, then traverses parent relationships. It retains PID, parent, process group, session and start ticks. The parser handles spaces and closing parentheses in the stat name field. No command lines or separate names are fetched on this path. Cleanup shares that parser so the same PID/start-time identity is checked before signals and in the final positive-absence check.

The full metadata collector remains available for startup, topology and held-gate diagnostics. Root-exit detection, descendant discovery on every pump, polling constants, bounded waits, stop behavior and owned-group signaling remain unchanged. Identity observations are never cached across scans; a recycled PID with a new start time remains a different process. The parent scan continues across all visible processes because the existing owner unit is not an individual plug-in-instance ownership boundary. This follow-up does not replace the broker or change ownership policy.

## Matched unprofiled CPU comparison

Same Steam Deck, retained Serum 2.0.18 environment, native/Windows binaries and pinned runner as AP9. Both runs: 48 kHz, float32, 128-frame host blocks, 512 added frames, eight simultaneous notes every two seconds with one-second hold/release, two MainVol points per block. Each processes 864,000 frames over 17.9974 seconds. Measurements use the SDK host's existing audio begin/end interval, excluding startup/state/retirement. The profiler is absent.

| Cost over the audio interval | Reviewed owner | Optimized owner | Reduction |
|---|---:|---:|---:|
| Python CPU seconds | 9.00 | 3.87 | 57.0% |
| Python percent of one core | 50.01% | 21.50% | 28.50 percentage points |
| Whole owner cohort CPU seconds | 22.459 | 17.508 | 22.0% |
| Whole cohort percent of one core | 124.79% | 97.28% | 27.51 percentage points |
| Separate native SDK-host CPU seconds | 1.288 | 1.221 | descriptive only |

Python comes from PID/start-matched per-process `stat` tick deltas; the cohort comes from systemd CPU accounting and includes Python, Wine, Proton services and the Windows host. Process tick resolution is 10 ms. Other owned costs remained similar: wineserver 3.83 → 3.83 seconds, Windows device services 4.12 → 4.17, Windows plug-in host 3.12 → 3.13, Xalia 2.37 → 2.51. This is a material Python reduction, not a claim that all owner CPU was census work.

Both runs have zero missing frames, gaps, callback rejections, callback deadline misses, host schedule misses and terminal faults. Actual state capture/restore and positive owned cleanup pass. Admission-to-publication mean is 0.666 → 0.681 ms and p99 bucket bound 1.152 → 1.280 ms; observed maxima are 2.670 → 2.681 ms. The change saves CPU; this pair does **not** establish lower service latency. The bridge remains 512 frames and vendor-reported latency remains zero.

## Focused Bitwig comparison

The material CPU saving justified exactly two new desktop checks, both using normally Applications-launched Bitwig 6.1, the same saved clip, 48 kHz float32 and actual 128-frame host/graph blocks. They reuse the final AP9 native and Windows binaries. Moonlight remained connected for control; audio evidence comes from the local Deck speaker monitor.

| Added frames / ms | Processed / visible playback seconds | Service p99 bound / maximum ms | Missing frames / gaps | Python / whole cohort CPU, % of one core |
|---|---:|---:|---:|---:|
| 512 / 10.667 | 108.57 / 96.63 | 1.024 / 9.111 | 0 / 0 | 24.00 / 100.58 |
| 256 / 5.333 | 113.37 / 90.42 | 1.024 / 21.904 | 2688 / 2 | 24.25 / 96.30 |

CPU columns cover separate 20-second local-monitor windows during playback; lifetime gap counters also include stopped callbacks. The two playback durations differ slightly because of manual GUI control. Each capture contains 1,720,320 channel samples, 17.92 seconds of stereo audio after recorder startup, with nonzero output and RMS about 0.00166. Native DAW CPU was not separately instrumented. These desktop runs compare buffer settings after the optimization; the controlled CPU before/after comparison is the SDK pair above.

**Keep 512 frames recommended.** The 256-frame run still misses output. Its worst correlated request spends only 0.083 ms of 21.904 ms in Serum; the retained trace includes 13.947 ms queued and 7.900 ms awaiting reply. A preceding request waits 12.864 ms for reply, with subsequent work queued behind it. The 512-frame maximum likewise spends only 0.136 ms of 9.111 ms inside Serum. This supports the existing finding of substantial stalls outside the measured plug-in process call; it does not identify a particular OS, IPC or runner cause, or prove that reduced Python CPU prevents gaps.

Both bridge instances report zero terminal faults, callback rejections and discontinuities, plus normal Windows-host exit, positive containment and retirement. State remained at MainVol 0.1150 after save/reopen. **The 512-frame project close nevertheless produced Bitwig's “Audio Engine Crashed” dialog.** The application log records engine shutdown followed by EOF and engine exit code 1. This is a retained desktop-close failure, separate from the gap-free playback and successful Windows cleanup. A filename search after dismissal did not find the named engine stack report. Its exact cause remains unresolved. The 256-frame project closed without that dialog, and final application shutdown logged engine exit code 0. No clean-desktop-close claim is made for the first run and no broader crash investigation or binary change is folded into this follow-up.

The local PipeWire graph runs at 48 kHz/128 frames with float32 ports. The speaker sink remains S16LE at 48 kHz, ALSA period 1024 and headroom 1024. Speaker error snapshots are 3 for both windows; Bitwig snapshots are 0 then 1. These cumulative values are not a complete device-error log. Hardware latency is unmeasured and no bridge resampling or precision conversion is introduced.

After testing, both AP9 owners are inactive and both prepared session directories are empty. The scanner and owner marker, Bitwig preferences/cache and PipeWire minimum quantum are restored; the temporary proxy is unpublished and private bridge setting returned to 512. Original AP8 and original AP9 project hashes remain unchanged. The separately saved test copy and sanitized failure records remain available.

## Verification and limits

Five new focused tests cover fresh descendant/grandchild discovery, malformed/disappearing process entries, PID reuse, unexpected root exit, owned-only signaling, healthy siblings and refusal to report success with surviving descendants. Existing AP4/AP5/PC0 tests retain lifecycle and deadline checks with their synthetic collector fixtures updated. The frozen function-body comparison permits the collector substitution while preserving the lifecycle and wait logic. All 68 focused tests and both PR CI checks passed at `a30b4ac`. No native or Windows build was needed for measurement.

This is one before/after workload pair and two short desktop settings, not a statistical campaign or a general lower-buffer qualification. The remaining stat scan and other Python work still consume CPU. Wine services and host scheduling remain material costs; there is no causal attribution of a specific audio gap to the supervisor. No new commercial crash-recovery or simultaneous-commercial-instance qualification is claimed.

Sanitized measurements: [owner-cost evidence](../evidence/AP9/owner-cost.json). Original [AP9 report](AP9.md), [measurements](../evidence/AP9/measurements.json), [desktop results](../evidence/AP9/desktop.json) and [fixture/binary provenance](../evidence/AP9/provenance.json) remain intact.
