# AP2 — Fresh SDK-hosted offline processing result

The Linux SDK host received **1,730 of 1,730 exact float32 samples in its own
buffers**, with **maximum absolute error 0.0** at zero tolerance. This was a
fresh acceptance run against actual Windows AGain, after a separately retained,
acceptance-ineligible diagnostic. AP1 remains accepted pending AP2 review/merge.

## Observed processing

The host loaded `AGainOfflineBridge.vst3` with the pinned SDK hosting helpers,
created IComponent and queried IAudioProcessor. It used standard ProcessData
and parameter queues; it never called the private C ABI or accessed the mapping.
Fresh signed binary-fraction inputs were selected after activation. Both the
host and retained-result validator independently compared every returned sample.

| Case | Frames | Gain | Input / actual output silence mask | Max error |
|---|---:|---:|---:|---:|
| Original changing-input cases | 1, 16, 63, 256, 16, 1, 256 | 0.5, 0.25, 0.75 | 0 / 0 | 0.0 |
| Whole-block silence | 63 | 0.25 | 3 / 3 | 0.0 |
| Non-silent inputs, zero gain | 16 | 0 | 0 / 3 | 0.0 |
| Left-channel silence | 63 | 0.5 | 1 / 0 | 0.0 |
| Empty queue retains previous gain | 17 | 0.5 | 0 / 0 | 0.0 |
| Audio after zero-frame, zero-bus gain flush | 31 | 0.25 | 0 / 0 | 0.0 |
| Same-channel in-place buffers | 47 | 0.75 | 0 / 0 | 0.0 |
| Fresh module reopen, absent gain queue | 19 | default 1.0 | 0 / 0 | 0.0 |

The persistent activation made 13 Windows process calls and returned 1,692
samples through one AGain instance, mapping and loopback control connection.
The zero-frame update consumed no audio sequence. Ten malformed/unsupported
VST3 calls were rejected before dispatch and did not change pending gain.
Fresh unload/reopen made one additional Windows process call and returned 38
samples, with a fresh mapping/session/instance and no inherited gain.
Output masks were preserved independently of input masks; every flagged sample
was zero. Latency and tail queried from Windows AGain were both zero samples.

## Lifecycle, failure behavior and containment

Both sessions completed host-requested setup/activate/start/process/stop/
deactivate/terminate/release. Windows start/process/stop used its processing
thread; stop acknowledgement followed thread join. Deactivation and object
release preceded module unload; endpoint closure followed Windows unmap.
Both native module unloads returned successful termination and released all
host references. Owned process groups were empty, owned descendants zero,
and the disposable stage absent. Protected state was unchanged:
`307e53f193155ee471e5a7d8ae5ab106ddc9ea253e0a42d203f5429b66eef305`.

Focused tests through the actual SDK-loaded proxy used local substituted peers
for discovery, backend unavailability, zero-call start/stop, the positive path,
disconnect, timeout, stale sequence, invalid mask, false silence and nonfinite
output. Each fault after dispatch produced exactly one request, explicit failed
processing/teardown, defensive output clearing, reference release and unload;
no retry or local gain fallback occurred. [Compact results](LOCAL_TESTS.json)
are local test evidence and consumed zero Windows workloads. Rust ABI/ownership,
AP1 codec/result-handler, AP0/AP1 worker/runtime/client and classified
policy/backend regression suites also passed. Worker tests exercise original
error retention despite normalization/reporting/retirement failures.

## Exact result and cumulative cost

- Execution source: `6c3ccd5888c17f2d236f433d45fe0ee0b8dc70fb`.
- Windows producer source: `f1402568d20d8f3e3331db7cae65aab6953edb9a`;
  successful run `33978589618`, artifact `9973099999`.
- Native source: `47673cc268e2c4ac15729a43083e7dce8220b1aa`; unchanged native
  input digest `5a8cadddfe8cd12e03cbb2e604c15dba960e49432e4519a1e89c910a8edc9b93`.
- Acceptance reservation:
  `d12c4783e0f6906e0625c9c20e57213385d0f7c6ee671ec4843c2322bee4f704`.
- Retained acceptance result SHA-256:
  `67631a8816aae2936a6c6714890148a4aaf570bf6346140866be14ce537fe60f`.
- Diagnostic reservation:
  `9e20709de8389f8200dfc281bf0cb863c7f95738758aefb62a8e62cb50aed30a`;
  result `ae30de632d32a82ac5f9d322530103eb9131b352c705b3cd8e61b1c0105894c1`;
  permanently acceptance-ineligible.

| AP2 allowance | Consumed | Remaining |
|---|---:|---:|
| Windows producers | 2 / 6 | 4 |
| One diagnostic campaign | 1 / 10 | 9 |
| Acceptance candidates | 1 / 2 | 1, only after a specific tested repair |

Producer attempt one (`33978352979`) failed before compilation because the
existing call-surface verifier did not yet allow AP2 latency/tail queries.
The narrowly scoped verifier repair and regression preceded successful attempt
two. Each live batch contained two supervised Windows launches and one SDK-host
batch: four Windows launches across diagnostic plus acceptance, no live retries.
The acceptance candidate's one-batch allowance is fully consumed. All historical
counts, locks and accepted PC0/AP0/AP1 evidence remain unchanged.

The five-file [acceptance packet](../../../evidence/ap2-native-vst3-offline-bridge/FINDINGS.md)
contains actual request/return words, lifecycle and cleanup observations, exact
source/artifact/runtime bindings and external hashes. Producer/build artifacts
remain in the existing private immutable stores. Native and Windows artifact
bindings are in `docs/campaigns/AP2_NATIVE.json` and `AP2_ARTIFACT.json`;
the selected runtime remains AP1's exact Proton 11.0-2c/Runtime 4 input set.

This is offline float32 stereo processing at 48 kHz only. Realtime/prefetch and
64-bit operation are refused. No Bitwig publication, audio-device timing,
controller/editor, saved state, commercial plug-in or general compatibility
claim is made. The implementation and fresh observation are ready for review.
