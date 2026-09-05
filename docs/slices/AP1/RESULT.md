# AP1 — Fresh mapped audio verification

This records the first acceptance on Proton 11.0-2. The later silence-repair acceptance and current cumulative totals are in [RESULT_A2.md](RESULT_A2.md); all measurements and costs below retain their original first-result scope.

Linux independently verified **1344 of 1344 returned float32 samples**, with **maximum absolute error 0.0** and zero tolerance. This was a fresh acceptance session on the retained Steam Deck / Runtime 4 / Proton 11 / Windows AGain fixture, following a separately retained successful diagnostic. AP0 remains the accepted frontier pending AP1 review and merge.

| Request | Stereo frames | Gain | Input | Maximum error |
|---|---:|---:|---|---:|
| 1 | 1 | 0.5 | Distinct signed channels | 0 |
| 2 | 16 | 0.25 | Changed data | 0 |
| 3 | 63 | 0.75 | Changed data | 0 |
| 4 | 256 | 0.5 | Changed data | 0 |
| 5 | 16 | 0.75 | Changed data at repeated length | 0 |
| 6 | 63 | 0.25 | Silence | 0 |
| 7 | 1 | 0.5 | Changed data at repeated length | 0 |
| 8 | 256 | 0.75 | Changed data at repeated length | 0 |

The native Linux caller chose seed `7977146040396510381` after Windows Ready. Windows never received that seed or expected answers. Actual Linux request/input/output words are retained in the acceptance packet. Python independently recomputed each expected sample from the Linux-owned request. Inputs, boundary guards and unused output capacity remained unchanged. No nonfinite, unwritten, stale or swapped output was accepted.

One host, AGain component/processor instance, 4192-byte shared mapping, slot and authenticated loopback connection served the full session. Both endpoints witnessed the same mapped bytes before module loading. Only control metadata crossed the socket; audio came back through the existing live mapping. Setup/activation used the owner thread, processing used a distinct thread, and stop/join preceded deactivation and interface/component/DLL retirement. The pinned SDK returned kNotImplemented for its two setProcessing notifications; the same AP0 interpretation is retained. Processing and other required lifecycle calls succeeded.

Windows unmapped and closed its borrowed file/mapping handles before Closed. Linux received Closed and checked its own unmap result. Both endpoints exited zero, both owned process groups were empty, in-process shutdown completed, and the disposable stage/backing file was absent. Protected-state hashes before/after both equal `307e53f193155ee471e5a7d8ae5ab106ddc9ea253e0a42d203f5429b66eef305`.

## Exact evidence

- Executed source: `28ea95d04db5c1ef93f14d3d3fa74dfd9eb4d219`, tree `ddb13931aeeb57b474fadeff06b3c4848ec701a2`.
- Windows producer: `3d27ec6e451a7d03363eb0881ec24cab3da7cc5a`; [run 33946080273](https://github.com/kasselvania/Linux-VST-bridge/actions/runs/33946080273), artifact `9963399733`; host manifest `d9ccb5eadb73e368bbb799753b79b307a0ba3a63dd28c856003f082c51f8565d`.
- Static Linux caller: Rust 1.90.0, `x86_64-unknown-linux-musl`, input source `69158d6d7244d4fef02d66e101c5f2f72c237be9`; binary SHA-256 `06de88d95db83f076f07fe3246f4c7f139525efb37a3f8be6900cddaa6da044c`; build manifest `65f416b4ffaedcd249d511d4dc8c7efb277530d6c2cf2b3813cf9ded06cb58aa`. Execution-source native inputs match this retained build exactly.
- Fresh acceptance candidate: `4609ac14775d65ceff15bf1d62e237b723ce1c92d6ca2f10c513143c9b972905`; reservation `bcd0a2488b36e0bd71682bc6405815ca64e1454d38bb4010638a42dd87d70e71`; result `e0135c9734c3edc43a3b8d9403533f252dbe2607b97b66d00b51690445e8bdc9`.
- Sanitized packet: [evidence/ap1-linux-windows-audio-roundtrip](../../../evidence/ap1-linux-windows-audio-roundtrip/FINDINGS.md). The packet retains exact identities, request words, actual mapped output, call/thread/retirement facts, and file hashes.
- The diagnostic remains private and acceptance-ineligible: campaign `78c32c41080c2afb3f8bf85967474dfe49016eb84c76f62ac0bb272118fbd755`, reservation `ecd47f31545ee6a21d43d087e680d979f8228eb990ce49fba3889864323857f6`. It independently observed 1344 exact samples with a different seed and complete cleanup.

## Tests, repairs and cost

The real Rust client passed local mapped-file/loopback-peer tests for the complete changing-block recipe, corrupted output retained before rejection, stale Done, disconnect and deadline. Faults terminate after one request without replay or slot reuse. Rust codec/state tests cover fragmentation/coalescing, malformed frames, wrong identity/sequence/extent, duplicate completion, bounded partial-write failure and deadlines. The C++ codec/sequence test uses the same golden vector and tests invalid descriptors and duplicate/out-of-order requests before processing.

AP1 worker tests exercise actual classified command/retention/rendering, original mapped words surviving normalization/reporting failure, native companion ownership/cleanup composed with a supervisor exception, and lost acknowledgement without duplicate execution. AP0/PC0 and policy/backend regression suites pass. These negative tests used local peers/platform substitutes; no hostile plug-in or live fault campaign is claimed.

Local tests found that an accepted socket can inherit nonblocking mode on macOS; the client now explicitly selects blocking mode with absolute I/O deadlines. Returned mapped words are saved before comparison rejection, native unmap success is checked, and the existing worker's bounded checkpoint includes both endpoint observations before stage retirement. Shared cleanup remains responsible for owned PID/start-time identities. The native process uses its owned session as cwd and a relative session argument; aggregate cleanup separately verifies its group before retirement.

Read-only preflight found that the existing artifact transfer deliberately installed files without execute permission. After exact binary/owner verification, only the new native caller received owner-only execute permission. No bytes, fixture or runtime changed and no batch was consumed by this prerequisite repair.

| Allowance | Consumed | Remaining |
|---|---:|---:|
| Windows producers | 1 of 6 | 5 |
| AP1 diagnostic batches, one campaign | 1 of 10 | 9 |
| Fresh acceptance candidates | 1 of 2 | 1 conditional candidate |

Candidate one used its one permitted batch. There were two read-only standalone preflights, plus the normal preflight inside each live command. The separate diagnostic and acceptance each published once and executed one batch. No retries, source revisions after live failure, extra mapping probes, fixture rebuild/download, runtime replacement, or changes to historical budgets were needed.

This proves the selected offline native Linux / Windows audio crossing on this exact fixture. It makes no DAW, real-time performance, general IPC platform, editor, state, commercial plug-in, arbitrary-memory-safety or sandbox-security claim.
