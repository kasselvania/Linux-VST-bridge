# AP1 review repair — fresh verification blocked before reservation

The repaired ten-case path has **no new device numerical result yet**. Read-only preflight stopped at `verify_diagnostic_runner`: `RuntimeError: Deployed runtime input differs: runner/version` (SSH worker exit 1; command exit 2). The first acceptance still proves its original **1344/1344 samples, maximum absolute error 0.0**, with complete cleanup; its [result](RESULT.md), evidence and receipt remain unchanged.

## Implemented and locally verified

`MappedSession::done` now calls the production `processing_result` handler to validate the full 64-bit output silence mask independently of the input mask. Only stereo bits are legal; marked channels must contain finite zero-valued samples. Unflagged channels may be zero. Actual flags are retained before native validation and returned in the version 1.1 Done payload. Linux retains the output flags and mapped words before independently checking every expected sample, guard and unused output region. Ownership, correlation, no-replay, deadlines, lifecycle and cleanup are preserved.

The original eight cases are unchanged. Added cases are gain 0 on non-silent stereo input (input mask 0), and gain 0.5 on a correctly zeroed left channel with non-silent right input (input mask 1). The resulting recipe checks 1502 samples. Their actual AGain output masks remain unobserved in this repair run; local native-handler tests cover output masks 3 and 0 respectively without substituting those expectations for live results.

Focused tests passed: 90 Python tests including actual Rust mapped-file/loopback integration and worker/adapter/runtime/policy/backend regressions; eight Rust tests; and the C++ test invoking the production native result handler. Tests reject illegal low/high output bits, nonfinite output and false silence claims; the real client retains false claims and stops without replay. A deterministic nonzero witness prevents a random all-zero input from weakening the negative test. AP0/PC0 checks passed. These are local failure tests, not live fault experiments.

## Exact source and artifacts

- Tested executable source: `fd3d70a36b65e1bba9fefc4914ebb1685b10faae`, tree `63328b0b7936f5ea7ed2812cba19493dfec1ac2f`.
- Windows producer source: `34699b682439ed75ea9fd0992b98222253b6f1da`; [successful run 33971421519](https://github.com/kasselvania/Linux-VST-bridge/actions/runs/33971421519); artifact `9971070716`; host manifest `f38dee93baf0e4054f3d729e561f9aa8756605935174ccb4cd889cd9d74cf1dc`.
- Native caller source: `34699b682439ed75ea9fd0992b98222253b6f1da`; binary `9191a64e92faf92559cf0121208f73b52c2c2d5d4534d75f6c44ede27ebe2225`; manifest `cabc63622f73a9c541941a8483f605bc929c2a11b04467928f079a1760cb7ce8`.
- Both exact artifacts were delivered using existing custody/transfer. Only the verified new native binary received owner execute permission; no AGain or runtime bytes were changed.
- [AP1_D1.md](../../campaigns/AP1_D1.md) mechanically binds the tested repair under the original campaign and preserves the first reservation. No second acceptance receipt or reservation was created.

## Concrete runtime blocker and current containment

The deployed Proton installation differs from the pinned runtime:

| Fact | Pinned | Observed |
|---|---|---|
| Version | `1787334450 proton-11.0-2-x86_64` | `1788504981 proton-11.0-2c-x86_64` |
| Steam build | `24867889` | `25118279` |
| `runner/version` SHA-256 | `823833b4a22543efdd3b0822981a9518dff08282520eba16661bd9d59bf5e026` | `85597f4c274a7c4a6815805d65265b60e6d393a8bec858ffb032a162c40ec5e4` |
| `runner/proton` SHA-256 | `ccb67e21ef0d81cc4142ee3af74a16702292623bad11ec7047dbd6c97be62eed` | `787504a79bacf6b303984a9a846cf47463f36599248e8306b26d5faf78267aad` |

The sampled Wine/wineserver binaries and Runtime 4 VERSIONS file still match. Both Steam application states are installed/quiescent (4). Steam content_log records `Priority Auto Update` at 2026-09-05 03:35:16 on the Deck clock, followed by successful commit of build 25118279 at 03:35:22: 1632 files updated, one moved, none deleted. This establishes a Steam automatic update; the bounded readback is not a complete runtime audit. Tailscale's periodic SSH authentication was resolved before this preflight.

Current process guard counts are all zero; native client process count is zero; prior transaction stage count is zero. Protected-state digest is unchanged from first acceptance: `307e53f193155ee471e5a7d8ae5ab106ddc9ea253e0a42d203f5429b66eef305`. No new plug-in instance, stage, workload, diagnostic publication or acceptance candidate was created. The read-only preflight runs before backend construction, so the stored diagnostic plan and reservation count are also untouched.

| Cumulative allowance | Consumed | Remaining |
|---|---:|---:|
| Windows producers | 2 of 6 | 4 |
| AP1 diagnostics, original campaign | 1 of 10 | 9 |
| Acceptance candidates | 1 of 2 | 1 |

Next decision: restore the pinned Proton build, or explicitly authorize AP1 verification against the installed 11.0-2c runtime with its own exact binding. CURRENT_SLICE excludes runtime replacement and requires reusing the unchanged runtime; this repair does not authorize silently accepting changed launcher bytes or altering historical locks. Keep PR #51 unmerged. AP0 remains the accepted frontier.
