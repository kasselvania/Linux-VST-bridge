# AP1 — Fresh silence-repair verification on Proton 11.0-2c

Linux independently verified **1502/1502 returned float32 samples with maximum absolute error 0.0**, using exact equality and zero tolerance. This is the fresh second acceptance candidate, on the explicitly approved installed Proton 11.0-2c runtime. Both new valid-request cases passed with their actual output silence masks retained independently of input flags. The [first acceptance](RESULT.md) and its immutable evidence remain unchanged.

| Request | Stereo frames | Gain | Input silence mask | Actual output mask | Maximum error |
|---|---:|---:|---:|---:|---:|
| 1 | 1 | 0.5 | 0 | 0 | 0 |
| 2 | 16 | 0.25 | 0 | 0 | 0 |
| 3 | 63 | 0.75 | 0 | 0 | 0 |
| 4 | 256 | 0.5 | 0 | 0 | 0 |
| 5 | 16 | 0.75 | 0 | 0 | 0 |
| 6 | 63 | 0.25 | 3 | 3 | 0 |
| 7 | 1 | 0.5 | 0 | 0 | 0 |
| 8 | 256 | 0.75 | 0 | 0 | 0 |
| 9 | 16 | 0 | 0 | 3 | 0 |
| 10 | 63 | 0.5 | 1 | 0 | 0 |

Requests 1–8 preserve the original numerical recipe. Request 9 supplies non-silent stereo input at gain zero: AGain returned all-zero samples and mask 3. Request 10 supplies a correctly zeroed left channel, non-silent right input and gain 0.5: AGain returned mask 0; Linux verified the zero left output and every gain-scaled right sample. Unflagged zero-valued channels are valid. The existing all-channel-silent request also passed with mask 3.

Linux selected seed `7627698536824973777` only after Windows Ready. Windows did not receive the seed or expected answers. The packet retains actual Linux inputs, output words and flags; Python independently recomputed every expected sample. Guard words, input ownership and unused output capacity remained intact. Zero nonfinite, stale, unwritten or swapped samples were accepted.

One Windows host, one AGain instance, one 4192-byte shared mapping and one authenticated loopback connection served all ten changing blocks. Both endpoints witnessed the same mapping before module load. Setup and activation stayed on the owner thread; processing used the distinct processing thread, which stopped and joined before deactivation, release and unload. The existing AP0 interpretation of AGain's kNotImplemented setProcessing notifications is unchanged. Windows unmapped before Closed; Linux received Closed and confirmed its own unmap. No replay occurred.

Both endpoints exited zero. Owned descendants and both process groups were empty; the disposable stage and backing file were absent. Protected-state digests before and after both equal `307e53f193155ee471e5a7d8ae5ab106ddc9ea253e0a42d203f5429b66eef305`. The explicitly bound runtime composition matched at preflight, launch and post-run.

## Repair and runtime binding

`MappedSession::done` calls the production native result handler, which validates the full 64-bit output mask and corresponding finite zero samples independently of the input mask. Actual flags are retained before validation and carried in the version 1.1 Done response. Linux saves actual returned data before comparison can fail. Exact numerical checks, guards, correlation, no-replay, timeouts, retention and owned cleanup remain intact.

Steam's log recorded `Priority Auto Update` and installed Proton 11.0-2c/build 25118279 in place of 11.0-2/build 24867889. Initial preflight correctly refused the changed version/launcher. The operator and the technical lead then explicitly authorized this AP1 runtime transition. `tools/ap1_runtime.py` binds the same 33 launch-critical paths and new exact build/depot records; historical WR0/PC0/AP0 and first AP1 selections remain unchanged. This is explicit selection of the completed update, not trust in an ambient latest runtime.

Steam retained completed transfer counters (160/160 download bytes and 747498077/747498077 staged bytes). The narrow AP1 verifier permits those only when totals match, installed/target builds agree, staging size and update result are zero, no update is scheduled, and installation state is fully installed. It still rejects partial/active updates and every bound-file mismatch. The original zero-counter policy remains the default for historical callers. No runtime installation, update-policy change, accepted seed modification or additional endpoint rebuild occurred.

Both runtime and host code differ from the first acceptance; this result does not isolate the causal effect of the Proton update. It verifies the complete explicitly recorded new composition.

## Exact evidence and validation

- Executed source: `a2e12c52df0db34c6476c1b337065e1f53e92638`, tree `33c582b8e40572732dc145b1626fc8d23c804e44`.
- Windows/native producer source: `34699b682439ed75ea9fd0992b98222253b6f1da`. [Windows run 33971421519](https://github.com/kasselvania/Linux-VST-bridge/actions/runs/33971421519), artifact `9971070716`, host manifest `f38dee93baf0e4054f3d729e561f9aa8756605935174ccb4cd889cd9d74cf1dc`.
- Native binary: `9191a64e92faf92559cf0121208f73b52c2c2d5d4534d75f6c44ede27ebe2225`; manifest `cabc63622f73a9c541941a8483f605bc929c2a11b04467928f079a1760cb7ce8`. Both endpoint artifacts were retained and reused after runtime rebinding.
- Runtime: `1788504981 proton-11.0-2c-x86_64`, build `25118279`, depot `2978628887517351791`; AP1 runtime contract `20064247dc297d0228bbf63d5a49050238c518d14fbb77b61a8c73351db5ebf7`. Runtime 4 and AGain remain byte-exact to their existing bindings.
- Acceptance candidate: `d3953b583ad60230deff0c306d7d6d015f5969ca8b48c03d2bb0937eebfc5eb6`; [authority](../../campaigns/AP1_A2.md).
- Acceptance reservation: `53e697f3cd58394d4c1d1c1b6a8e7f1867713673c7e764b1048197ce051ea467`; immutable result `c018835588a05a3e8fb7092ef95f262338b003060897b7f0d7a1c6f470f0a628`. The normal command returned `ACCEPTANCE_EVIDENCE_RENDERED`, success with no failure record.
- [New sanitized evidence packet](../../../evidence/ap1-linux-windows-audio-roundtrip-a2/FINDINGS.md), including the exact transaction, identities, call/lifecycle facts, mapped words and file hashes. The original packet remains under `evidence/ap1-linux-windows-audio-roundtrip`.
- Diagnostic reservation `55671028529aabb295fc7faff5e8f6d09415108bacf8e76634d45777e4a65260` separately observed 1502 exact samples with seed `659711247737960145` and complete cleanup. It remains private and permanently acceptance-ineligible.

Focused native C++ tests exercise the actual production result handler: zero gain, masks 1/2 for correctly formed partly silent input, ordinary/all-silent output, valid unflagged zeros, invalid low/high bits, false silence claims and nonfinite samples. Eight Rust tests and real Rust mapped-file/loopback integration passed, including retained invalid output, stale Done, disconnect and timeout without replay or premature reuse. The final shared runtime/worker/adapter/policy/backend suite passed 91 tests, including explicit old/new-runtime separation, changed-byte rejection, completed-versus-partial-update checks and lost acknowledgement without duplicate execution. AP0/PC0 regression checks passed. Failure tests are local platform/peer substitutions; no live hostile-plug-in fault campaign is claimed.

## Cumulative cost and boundary

| Allowance | Consumed | Remaining |
|---|---:|---:|
| Windows producers | 2 of 6 | 4 |
| AP1 diagnostics, original campaign | 2 of 10 | 8 |
| Fresh acceptance candidates | 2 of 2 | 0 |

The runtime refusals were read-only preflight failures and consumed no reservation. Diagnostic batch 2 and acceptance candidate 2 each executed and published once. The existing locked atomic plan revision preserved the first diagnostic reservation; both acceptance candidates retain their original source and authority. No historical count, result or unknown was reset or relabelled. No further acceptance run is authorized by the exhausted allowance.

This proves the selected offline native Linux / Windows numerical round trip and repaired silence handling on this exact composition. It does not prove DAW integration, real-time behavior, arbitrary transports, state, editors, commercial plug-ins or universal runtime compatibility. PR #51 remains unmerged for final review; AP0 remains the accepted frontier.
