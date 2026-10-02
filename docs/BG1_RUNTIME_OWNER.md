# BG1-R0 — selected native command-session runtime owner

## Claim and provenance

This source-only slice starts at public main `d20d143b5e1beb8009d7ae9f4bc9b4bf18556204`, tree `bbfd32cfed8007264ab8f361f7017da64dd0ba9d`. It restores the launch and retirement owner required by the existing Lunacy environment `7d8fce354e68595cdc20485f754a892d`, whose selected runner is `proton-11.0-2c-dcomp-bg1-v4`. The exact runner roster pins `native-command-session.json` at SHA-256 `1fca37746647a975d80666f75f84c050770820b2c0f7de58352bbf7ed4f40c30`.

The implementation basis is the installed private UI2 source `8bf70de6b442bcac4086de4e27dadbdc7e76ea43`, specifically the native command-session owner introduced at `c4aa34666fb7b8f08467c39e99d272db77dd8438`. Private BG1 PR #5 supplies the retained V1/V3/V4 lineage. Only `bridge-manager/runtime/session.py`, `bridge-manager/runtime/ownership.py`, their selected tests, the Rust session binding and exact historical interpretation are brought forward. Renderer tools, Wine builds, WebView2 payloads, graphics observers and private physical evidence are not imported.

## Runtime selection and custody

- A shared-runtime native session selects this path only when its exact runner file roster contains one `native-command-session.json` at the expected Proton-relative location. An ambient file does nothing. Duplicate, malformed, unsupported or digest-changed components refuse. The component pins the exact Steam Runtime command client and launcher service digests. A runner without the component keeps its existing launch path.
- The manager supplies the canonical serialized runner key in the private session specification. The shared keeper admission owner verifies the selected V4 environment's complete retained BG1 history before returning an existing keeper or creating a session, lease, endpoint or process. Inspection, vendor access and ordinary plug-in launch all use this owner. The keeper starts the command service inside its initialized Proton command, through a private bounded Unix endpoint. The graphical-session and session-bus restrictions remain in effect. The keeper publishes its command descriptor only after both environment and service readiness; the descriptor binds keeper ID, owner PID/start identity, runner key, graphical context and socket identity.
- An instance requires one matching live keeper, exact owner and a ready report bound to the selected environment, marker, context and endpoint. It launches through the pinned command client, forwards only the named runtime variables, and uses an inherited socket with a nonce handshake before the Windows target executes. A missing or changed keeper never falls back to another Proton container.
- The local client and remote process group are separate custody facts. A live remote identity cannot authorize signaling the local client's group. Local signaling requires the live local root or an exact retained local PID/start identity observed in its original group and session. The remote PID/start identity is added to instance ownership; TERM and bounded KILL signal the remote group only while an observed member with retained PID/start identity still occupies it. A surviving member, missing handshake or ambiguous custody leaves cleanup unconfirmed. An unrelated sibling is outside this owner. The keeper endpoint and descriptor are removed only after positive keeper retirement and exact identity recheck. A descriptor or socket identity dispute preserves both files and persists cleanup uncertainty.

This code does no audio-callback work and exposes no manager action accepting a caller command, executable, endpoint, PID, process group, runner or environment.

## Acquired-runtime delivery extension

The integrated delivery successor also selects this same owner for the exact
`managed-ge-proton11-7-slr4-20260805-r3` runner. Its verified `runtime-tree.json`
already pins the command client and launcher service. The supervisor derives
only those two fixed paths and digests from that declared manifest, checks the
exact entry-point layout, and independently verifies both tools. No component
is written into an existing runtime and no prefix identity changes. An unknown
runner, nonshared operation or undeclared ambient marker cannot select this
extension. Missing, duplicated or changed manifests/tools refuse; a missing live
keeper never falls back to another container. BG1's existing marker and retained
history contract remain independent. See the installed52 delay and source
regression in [restart validation](RESTART_VALIDATION_2026_10_01.md).

Internal53's installed attempt exposed two delivery defects in that reused
owner. Keeper creation assumed the application cache parent existed and claimed
its endpoint before exclusive creation; finalization consequently disputed a
directory that never existed. The packaged child launch also sent host Python
bytecode into runtime Python 3.13.5, which rejected it with `Bad magic number`.
The corrected owner creates and verifies the private application/command roots,
claims only a successfully created exclusive directory, and rechecks its inode
before removal. Existing or disputed paths are preserved. A precreation refusal
has no endpoint to retire.

Only a fixed, isolated Python text bootstrap crosses the runtime boundary. It
imports no supervisor or ownership module, reports its kernel PID/start identity
on the inherited descriptor, requires the existing nonce acknowledgment within
five seconds, closes that descriptor, then executes the exact selected command.
The parent keeps its existing live-keeper, graphical-context, socket and process
group checks. Actual pinned client/service forwarding and bootstrap execution
passed in the acquired runtime with positive process retirement. This is a
runtime interface test, not a Windows plug-in or DAW result. No startup, audio or
retirement bound changed.

## Retained transition and rollback meaning

The Deck's read-only PB0-R2 record set contains three completed Lunacy transitions: standard runner revision 1 to BG1 V1 revision 2, V1 to V3 revision 3, and V3 to V4 revision 4. The public read-only verifier binds each archived before-environment, onboarding record, removed-publication registry backup, prepared transition and completed result to its exact revision, runner key, manifest, tree, predecessor candidate and vanilla BEAM class. It also requires the current V4 runner's one pinned command component. Ambiguous duplicate rollback records refuse. The rollback root and transition directories must already exist, belong to the current user, be private and have stable canonical non-symlink identities throughout inspection. Missing or disputed directories refuse without being created or repaired. This check runs at the keeper boundary before V4 keeper creation **or reuse**; unrelated standard, DComp and touch runners retain their existing behavior.

Those backups are recovery evidence, not an automatically offered rollback. Public source has no action that executes rollback from these historical BG1 directories. The verifier never writes them and does not import the private experimental runner transition/build command. A future rollback mutation would require its own reviewed closed owner.

## Validation and limits

Source-owned tests cover selection, component/client/service changes, keeper binding, endpoint and descriptor identity, handshake refusal, remote process-group cleanup and the exact V4 transition result. The full Linux runtime and manager suites, strict Clippy, package tests, AP12, PX2 and paired Linux build are reported on the PR at its final head.

BG1-R0 does not replace the Deck generation, launch BEAM, make a new graphics or audio claim, redistribute a runner, or qualify a customer package. The next read-only PB0-R3 gate must run complete canonical Snapshot and Activity readbacks and prove PKG0 predecessor preservation against the unchanged installed state.
