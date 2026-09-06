# Current work: AP4 — Save and reopen a Bitwig project

## Goal

Make the current bridged Windows AGain setting survive normal project save, application exit and reopening. The restored control and actual audio processing must agree without a driver or agent re-entering the saved value.

The AP4 execution agent is already working on `codex/ap4-plugin-state-project-recall` under the operator's direct outcome-focused instruction. Continue that work. No new selection, execution receipt, diagnostic-to-acceptance replay or timed GUI ceremony is required by repository policy. Normal tool approvals and explicit spending limits still apply.

## Enough evidence to finish

Use a disposable Bitwig project, set a clearly nondefault value, save it, close the DAW and owned plug-in processes, and reopen the same project with fresh processes. Observe restored state and actual processing before touching the control. Preserve the saved project and useful supporting state/audio results. Add focused regression tests for implementation defects discovered.

Use the real Windows component state, preserve audio-thread behavior and clean up the owned session. Do not fabricate restoration from a test-driver gain value or claim success from a serializer alone. Prior SDK and local results may be used with their exact provenance; they need not all be rerun because a GUI step or report failed. Additional cases are driven by a concrete risk, not a compulsory multi-stage campaign.

Open one reviewable PR when this outcome works. Include a concise result and current-status update; leave unmerged for technical review. AP4 is not accepted by this process cleanup.

## Accepted baseline

AP3 is accepted through PR #56, merge `953217dd50c120a86c93acb69c2baf8cdfbea35f`; PC0 and AP0–AP2 remain accepted prerequisites. AP3 demonstrated a single-instance Windows AGain preview in Bitwig, 6,366,144 independently checked samples with maximum error 0.0, and explicit 1024-sample added latency. The reported configuration is float32 stereo, 48 kHz and 1–256-frame callbacks, not universal real-time or commercial compatibility.

See [AP3 result](docs/slices/AP3/RESULT.md) and [retained evidence](evidence/ap3-sustained-native-audio/FINDINGS.md). The old packets retain their publication-time labels. Save/recall, lower latency, vendor editors, instruments and commercial support are separate claims.

## Access and concurrent work

Reuse [Moonlight/Sunshine and SSH](docs/DECK_REMOTE_DESKTOP.md). Preserve ordinary permissions, existing music projects and installations. Restore only settings/publications changed by the test.

Repository-process cleanup is independent of AP4. It does not alter that branch's code, receipts, running sessions or evidence. Do not interrupt the experiment or rebuild unchanged binaries to adopt documentation changes. During merge, retain these outcome-led rules rather than restoring the retired permission choreography from the older branch.

`tools/proof-run.py` is legacy/optional. Its closed default is stored separately in `tools/legacy-proof-default.md`; it is not this task's status or a revocation of the operator's instruction.
