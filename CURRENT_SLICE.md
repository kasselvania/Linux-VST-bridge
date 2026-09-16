# IS2 — Exact application-presence and close-handoff attribution

Operator-selected implementation on exact IS1 evidence head
`a0fde40e4c2a0536aa9b3271c3b945a94aa60d9b`, tree
`a6c08ddab4964e3df672628b307de98fea4facb3`. Main remains `be9a7e9`; PRs
#110/#111/#112 are open. IS2 explicitly stacks on #112 without changing or merging
any dependency. Reviewed installed IS1 implementation is `0543c3d`.

Primary claim: a managed installer separates exact Windows launch-root authority,
application-presence queries, attempted close and post-close recheck from helper
success, outer exit, installation witnesses and cleanup. Missing facts stay
unavailable. No product-name rule or vendor workaround is authorized.

Read-only attempt-3 forensics comes first. Preserve all IS1 evidence bytes and the
three retired environments. Privately inspect the immutable import, retained
bounded logs and temporary-file census; publish only classifications/hashes.

Implement a source-owned, operation/epoch/token/artifact-bound launch observer and
bounded generic presence/close records. Prove direct-loader/child/wrapper and
identity-negative cases, plus real source-owned detection/close cases through the
production installer supervisor and pinned runner. A launch adapter must preserve
manifest enforcement, working directory/environment, exit and cohort ownership;
it must not add elevation or security authority. No observer work touches audio.

Scope: installer runtime/ownership, closed manager integration, installer result
presentation, Windows launch/close fixtures and their build inputs, focused tests
and sanitized evidence. Existing products, runner, profiles, authorization,
projects, publication and installed software stay unchanged. No Bitwig, Serum,
Native Access launch, broad IS1 matrix replay, or accessibility experiment.

Run affected runtime/ownership, manager/frontend tests and strict Clippy;
AP8 for Windows inputs, AP12 and PX2. AP10 out of scope. Return one draft PR.
Independent source review and immutable installation are required before any
commercial confirmation. At most one later linked human-only attempt is possible;
none is authorized during this implementation. Keep all dependencies and IS2
unmerged.

Exact-head rereview at `e7c2a9547fd025a45ab07e67bde447dffeb8e21f` accepts the
Windows/root/close evidence and requests only setup rollback compatibility:
explicit packages select their own installer adapter capability; an omitted
adapter must omit `installer_launch` from Software for legacy schema readers.
Only `package=None` acceptance retains the verified prior adapter. Preserve prior
immutable generations. Verify strict legacy deserialization, current-package and
acceptance retention, and unchanged old bytes. No installation, commercial run,
Windows matrix replay, IS3 implementation or merge belongs to this repair.
