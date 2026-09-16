# Validation custody

Implementation head: `294350b9941e86cad3fdb29db6ddb3683acfb728`.

- Linux installer/ownership tests: 46 passed on the Deck.
- AP12: 125 manager-library, 70 manager-binary, 18 frontend and 109 runtime tests
  passed, plus the workflow's existing source/ownership regressions.
- Strict manager and frontend Clippy passed locally and in AP12.
- AP8 Windows build, adapter contract tests and self-tests passed.
- PX2 passed. AP10 was not selected; no native/audio behavior changed.
- Final-head workflow receipts are linked from PR #113; the evidence commit does
  not recursively embed its own future hash.

Pinned-runtime case records identify exact supervisor/ownership and PE digests.
The x64/x86 noncommercial payloads and adapter were built with Zig; the
requireAdministrator payload came from AP8 run 35117962915 at earlier source
`84950a5c478a427381d1e6c5a66203f722a03fa5` (GitHub PR merge checkout
`9da770a1cc28921a119bb8bca4a0f90eb52545e7`). Its contract body and manifest are
unchanged by subsequent fixture fixes. Current-source Windows contracts are also
covered by AP8 run 35119087202. These binary/source identities are not conflated.

Two development fixture/harness failures are retained rather than erased:

1. An all-runtime test invocation outside the source tree lacked a UIO2 profile
   input and used Deck tmpfs where an unrelated storage negative test expected
   durable storage. No fixture launched in that invocation. Focused installer
   tests subsequently passed on the Deck; AP12 ran all runtime tests in its proper
   complete-tree fixture and passed.
2. Initial fixture window enumeration did not distinguish its owned source window
   from auxiliary windows; its cooperative case returned source-owned exit 244.
   The scope was narrowed by exact owned process plus the source-owned class.
   A wrapper case also fell through to the close case rather than completing its
   payload. Both source defects were fixed; the corrected close cases and Windows
   contract tests passed. The failed session cleaned up and remains in evidence.

One transient-unit collection race in the fixture harness was handled by checking
terminal result, positive cleanup and inactive unit before accepting an already
collected unit. It did not require relaunching that already-completed session.

Early development direct/bound comparisons remain separately identified. The
current-source 32-bit sentinel comparison supplies the final before/after root and
script-execution result. No old development binary is presented as a final build.

The renewed Tailscale check completed. Final readback after all 26 generated
sessions verified their exact retirement, removed scratch prefixes, unchanged
installed artifacts/products/publications and retained files. The final X11
button/modifier mask was zero; no input was sent.
