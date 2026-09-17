# NAUI2 generated production-owner proof

Executed source `56ed25b03ceee9a57143f9f0f7cecd7c6e150cdc`, tree
`88736f20cfad388b04c8152042b4a8880efe6cad`.
Sealed manifest SHA-256:
`9b8c2ab34981a6c5b046cf7049f75b407f2c9896eeb1f86762a18fdfe467d2e4`.

The source-owned x64 application is padded to 67 MiB. It creates separate GPU,
renderer and utility role children; the inherited GPU child deliberately exits 5,
while the software-rendering child exits 0. These are instrumentation results,
not a test of Chromium rendering or Native Access behavior.

The committed campaign calls the Rust production binding, then the production
application supervisor under the pinned runner and an exact dedicated systemd
unit. Prefix initialization belongs to the fixture wrapper and retires before
application launch. The production application owner never initializes or repairs
an existing prefix. No installed software was replaced.

| Case | Exact operation | Result |
| --- | --- | --- |
| Inherited | `8dec904125dabf848549c7e48fc28e16` | Bound root; GPU child 5; renderer/utility 0; outer 0 |
| Software rendering | `eead1400840d9aa9a0308ed9678bd02c` | Bound root; all three children 0; outer 0 |
| Restored inherited | `ed18ac0b1da007d7879e2000236dfcb4` | Bound root; GPU child 5; renderer/utility 0; outer 0 |
| Stop after failure | `409a5d2e2fac3d2ab08dcf9ec1ea0653` | GPU child 5 retained before exact unit Stop; cancellation and cleanup separate |

Every session retained a complete Windows trace with zero dropped observations,
positive cleanup, zero survivors, an inactive/collected exact unit and deleted
scratch prefix. The large application cache performed one bounded supervisor hash
and rechecked the open file identity for descendants. The adapter independently
verified the executable and returned child identity before resuming it.

Before/after readback is identical: active service, two keepers, zero DSP and
maintenance leases, zero pending transactions and stale transports, cleanup
unblocked, capture off, 307 retained file witnesses and five projects unchanged.
Installed software, products, publications, onboarding and environment inventory
remain unchanged. Exact private before/after records and diagnostics remain in the
owned proof directory on the Deck; the public result contains only sanitized facts
and hashes.

`source-manifest.json` binds the source-owned package, private context digest and
compiled owners. `generated.json` is the committed campaign's immutable result.
`validation.json` records tests and the three pre-session development corrections.
No Windows campaign failure was discarded; all four admitted Windows operations
passed their stated case.

Original NAUI1 observation and seal remain untouched. Native Access's historical
application generation and blank-window cause remain unavailable. No real Native
Access launch, presentation observation, account action, installation, registry
change or software-rendering remedy is claimed. The generated fixture has no GUI,
so EWMH focus on the real application remains a later physical check.
