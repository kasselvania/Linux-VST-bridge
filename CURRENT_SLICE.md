# IS3 — Truthful Windows scripting capability

Base: merged main `8d306d8f211a66ff5f140705b8c7bc1f5bcd1de9`, tree
`cf3cd37a5a34653142e93d6e3c0bdf49743fcc30`. PRs #110–#113 are accepted and
merged. IS2 immutable installation and idle readback passed; see
`evidence/is2/integration-installation.json`.

Primary claim: executable availability and helper exit status are separate from
verified script behavior. A source-owned capability consumer must not interpret
missing side effects or unexecuted predicates as a successful capability.

Reuse IS2's accepted pinned-runner baseline, exact adapter, installer ownership,
private HOME and scratch-prefix supervision. Compare the pinned false-success
stub with honestly unavailable scripting and, only where selected as necessary,
a genuine managed scripting dependency. Any genuine interpreter must prove the
requested side effect, exit code, process query and exact close; a mock cannot
stand in for that proof. Missing or unperformed comparisons remain explicit.

The first implementation cut supplies a bounded source-owned A/B probe and
truthful result classification. It tests a documented Wine loader override only
inside a generated child environment, never in an installed product profile or
managed prefix registry. No vendor-name dispatch, patched installer, fake
interpreter, privileged operation, account dependency or runner replacement.

Scope: `tools/is3`, source-owned Windows fixture build/CI, sanitized evidence and
documentation. No product/native/audio source changes. No Native Access,
Bitwig, Serum, ASC, installation, discovery, or commercial UI session. Preserve
all old attempts/imports, product publications, both keepers, and protected
projects. A future commercial confirmation requires independent review and
separate authorization; none belongs to this initial comparison.

Use exact operation/root binding, bounded records and positive cgroup cleanup.
Run source-owned parser/negative tests before the pinned fixture, AP8 for Windows
source inputs, AP12 and PX2. Keep the new implementation PR draft and unmerged.
Do not label source-owned fallback selection as proof of vendor fallback behavior.

## Continued checkpoint: Windows delivery and Unix loader authority

Preserve checkpoint `e173c998321d19c3f966ac9ca49ed21faf718a2b` and all prior
receipts. Canonicalize and verify the Windows child environment separately from
loader authority. Compare three new independent source-owned sessions: baseline,
fixed Unix target-runner override after prefix initialization, restored baseline.
The only runtime seam is a keyword-only development fixture invocation; normal
installer specs, CLI and operator actions cannot select it. It binds the staged
source-owned payload digest and exact operation. No replacement software install.
If this operation-scoped route is ineffective, a separate disposable-prefix
registry comparison is permitted. Do not install a genuine interpreter before
honest absence is established. Add x86 and x64 CI self-tests and keep PR #114 draft.
