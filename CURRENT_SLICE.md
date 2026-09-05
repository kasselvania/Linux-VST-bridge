# Current Work: AP3 Accepted

**AP3 is accepted.** PR #56 merged as `953217dd50c120a86c93acb69c2baf8cdfbea35f` after technical-lead review `5122870538` of exact head `9bf1ab0b4457796791cb704fa00f547c471366dd`. The sustained-audio/Bitwig outcome (#54) and the separate remote-development-access outcome (#55) are complete. AP2, AP1, AP0 and PC0 remain accepted prerequisites. No implementation successor is active.

## Accepted audio capability

The native Linux VST3 preview delegates real processing to Windows AGain while its host callback uses preallocated native Rust queues and never waits for the Windows transport. A dedicated worker owns mapping/control exchange. The SDK edge reports a fixed **1024-sample additional latency (21.333 ms at 48 kHz)**. Epoch and absolute sample-position correlation preserve alignment across block lengths and stop/restart. Faults are explicit and latched; no replay, stale-output delivery or local gain replacement is admitted. The existing offline path remains separate.

Fresh Deck verification on the explicitly bound Proton 11.0-2c / Steam Linux Runtime 4 composition checked **6,366,144 samples at maximum absolute error 0.0**: 5,764,096 primary samples across two 30-second streams and 602,048 in a separately labelled stream-active check. The actual SDK host compares each returned sample, and the retained-result checker independently reconstructs timeline digests from the input seeds. Primary callback maxima were **221.634 microseconds at 128 frames** and **47.131 microseconds at 256 frames**, against periods of 2666.667 and 5333.333 microseconds. Reported overruns, unexpected queue faults and intercepted forbidden callback effects were zero. Callback duration is not total audio latency or a worst-case scheduling guarantee.

The same build ran in **Bitwig Studio 6.1 Flatpak**, with the existing 48-kHz/256-frame PipeWire route. Two disposable sessions demonstrated scan/load, playback/looping, gain/mute/restored output, stop/removal and fresh reopen. Native records report 3,453,184 and 2,866,432 processed frames, gain range 0–1, no callback rejections/queue faults and clean native shutdown; both app exits were zero. Those records corroborate the agent's GUI observations. Bitwig meters and streamed audio are not numerical or timing oracles.

All four reported Windows sessions stopped/joined processing before deactivation/release/unload. Both endpoints' mappings and owned processes retired, staging was removed and protected state remained unchanged. The original Bitwig preferences were restored and the temporary test publication removed; acceptance does not mean a permanent production plug-in installation was left behind.

This is a **single-instance, float32 stereo, 48-kHz reference-effect preview with 1–256-frame callbacks and added latency**. It is not arbitrary-host/machine compatibility, hard-real-time certification, exhaustive syscall instrumentation, saved vendor/controller state, preset/editor support or commercial-plug-in compatibility. Intercepted callback effects, measured timing, local faults and real DAW observations remain distinct evidence classes.

## Remote development access completed separately

Actual Mac-side Codex Computer Use through Moonlight was reported and demonstrated for live viewing, scratch-editor clicks/typing, disconnect/reconnect and the Bitwig GUI sequence. Sunshine runs as a separate user development service; SSH remains independent. Normal operator pairing/portal/security approvals were used, with no streamed terminal, public forwarding, audio-route replacement or immutable-base change.

[DECK_REMOTE_DESKTOP.md](docs/DECK_REMOTE_DESKTOP.md) records the observed versions, configuration, start/stop and reversal. The service was started without enabling autostart. It requires an awake, reachable, logged-in Desktop Mode session; it is not cold-boot, login-screen or unattended credential access. Reuse this established operational access under ordinary tool/app permissions rather than reinstalling or re-pairing it for each slice. It grants no new plug-in workload authority and is not part of disposable Windows cleanup.

## Evidence and review

- [AP3 contract](docs/slices/AP3/CONTRACT.md), [actual results and costs](docs/slices/AP3/RESULT.md), and [acceptance findings](evidence/ap3-sustained-native-audio/FINDINGS.md).
- Executed source: `5823ec68a719194e281b9f6f3b6d0853e2baf3b0`.
- Acceptance reservation: `0ea53ba4623b93379eb08417f383f433c5efbf046194e6b5b317e176ea5fc148`.
- Result SHA-256: `ce9c8f735801c7c427260d8b5fbd6aaab83c9fbbb00d49ca83833356a7c9dbfd`.
- Windows artifact: `9974517171`; native manifest: `609e5e6ce3ec9334fd8506ac19594f4546de828167cc092f973958035a02319d`.

Review inspected the callback/queue/worker and SDK host code, Windows lifecycle integration, independent checker, callback instrumentation, batch retention/cleanup changes, published findings/result, remote setup record and final CI. The executed-source-to-reviewed-head comparison changes only authority/status/result/evidence files. Final PR CI run `33989243099`, job `101368370595`, passed compilation, **118 Python/integration tests, eight native-client Rust tests, nine backend Rust tests and C++ protocol/result checks** on merge candidate `808ab43bcdb807ea882eab36be73da830eb91433`. SDK-loaded fault tests and physical/GUI observations are separately reported local/device evidence, not additional tests claimed from that CI job. The lead did not independently operate the Deck/Mac, rerun the workload or recompute the complete private sample stream.

The evidence packet retains its publication-time pending-review wording. This record and the merge review establish subsequent acceptance; do not rewrite immutable observations to update status. Prior runtime bindings and all accepted evidence remain unchanged.

## Completed task and costs

Cumulative AP3 usage remains **1/6 Windows producers, 5/10 diagnostics and 1/2 acceptance candidates**. Earlier failures and unknown details remain historical. D5 was recovered using its same reservation with launch disabled and remains acceptance-ineligible; only fresh A1 supports AP3 acceptance. No count, lock or observation is reset or promoted.

No additional AP3 build, diagnostic or acceptance workload is needed or authorized by the completed work order. Read-only review and supported retained-result reporting remain available without relaunch. Routine use of the established developer desktop access remains separate from product workloads and subject to its existing permissions.

## Next product direction

The next selection should turn this working DAW preview into a reliably recallable session: real plug-in state/parameter restoration, supported host lifecycle and a saved disposable Bitwig project reopened with fresh processes and verified behavior. Reuse the accepted transport and remote access rather than building another orchestration framework. State must come from the actual Windows plug-in and declared host contract, not a fabricated success or replacement gain calculation. Low-latency tuning, broader formats/rates, MIDI/instruments and vendor editors remain separate claims. This direction is not successor implementation authority.

## Authority — default command guard

The default guard remains closed for unspecified work. Historical scoped receipts do not renew completed task permission or permit duplicate execution.

```yaml
status: no_active_slice
authority_phase: no_active_slice
change_class: PROOF_HARNESS_MAINTENANCE
maintenance_id: AP3
maintenance_status: complete
repository: kasselvania/Linux-VST-bridge
maintenance_implementation_authorized: false
product_implementation_authorized: false
live_execution_authorized: false
permitted_execution_class: none
classified_backend_core_ready: true
classified_backend_ready: true
production_adapter_registry: pc0_ap0_ap1_ap2_ap3_diagnostic_and_acceptance
accepted_product_frontier: AP3
current_product_target: none
pc0_status: accepted
ap0_status: accepted
ap1_status: accepted
ap2_status: accepted
ap3_status: accepted
diagnostic_campaign_authorized: false
acceptance_candidate_authorized: false
successor_selection_authorized: false
```
