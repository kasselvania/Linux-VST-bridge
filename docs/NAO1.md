# NAO1 — Native Access owned-session launch and cleanup

## Focused repair amendment: qualified-recovery readiness handoff

The first physical owned-session request at merged source
`a544645792aa1428d0388191e4fee45e7801b921` failed before readiness and before Native
Access launch. The renderer operation contained the complete validated
`qualified_recovered_installation` session record, but the application path passed only
its daemon identity to `Nad1Owner`. The owner therefore lost the admitted origin before
its first SCM query and returned the historical `dependency_prepare_required` error
after that query reported error 1060.

The operation-owned owner must receive and independently revalidate the complete closed
session record: origin, fixed recovery sources when selected, installer, daemon,
application, software, environment, physical prefix, service, and listeners. Presence
of daemon bytes alone never grants recovery authority. The retained artifact and fixed
qualified-recovery origins remain the only two choices, and an invalid present artifact
never falls back to recovery.

The sealed physical result contains only one SCM query, so it does not establish
whether error 1060 was transient during private-runtime initialization or represented
persistent missing registration. The smallest authorized repair is one fixed one-second
delay with continuous runtime draining followed by exactly one fresh query in the same
operation-owned runtime. This is observation only:

```text
initial exact 1060
→ retain full session authority and the same private runtime
→ one bounded fresh query
→ exact registration: revalidate prefix and run a fresh bounded process census
→ RUNNING/START_PENDING: continue only with one exact renderer-owned generation
→ STOPPED: continue only with no candidate, through one operation-owned start
→ absent or unavailable: refuse before application launch
```

Exact registration is not process ownership and does not itself grant retirement
authority. An unavailable or ambiguous census, foreign or deleted-prefix candidate, or
same-prefix unowned candidate refuses before application launch. A readiness failure
before one exact generation is owned leaves retirement authority false, so terminal
cleanup cannot submit a Stop request.

No installer is replayed, no service configuration is written, and no artifact or
preparation receipt is created. Persistent absence is reported distinctly as
`dependency_qualified_registration_absent`. Initial observation failure,
re-observation failure, readiness failure, application-launch failure, and cleanup
failure remain separate results. A future direct registration repair would require the
complete exact service configuration and separate tech-lead review; this amendment does
not guess or introduce it.

## 1. Product outcome

The purpose of this slice is to make Native Access usable.

The exact application operation must be able to:

```text
admit the retained exact NTKDaemon installation
→ establish fresh exact daemon readiness
→ launch Native Access with the selected software-rendering policy
→ own the application and dependency for one session
→ close the application
→ attempt one graceful service stop
→ complete exact-owned cleanup when graceful stop is not confirmed
→ leave the bridge ready for the next operation
```

NAO1 is not another daemon-characterization slice. NAD2 is complete. This slice uses
that characterization to define the narrow compatibility behavior needed by Native
Access.

## 2. Exact basis

- merged NAD2: `a86f03e8a5d5302d9872f995a0a5ba376a0ab6d5`
- merged tree: `11a9fefbbe8b184c52058f4b4610da3a5e3b06e4`
- qualified NAD2 executable source: `81bbf198e8bbe7df337312f2529c84b318c1ed64`
- selected application: exact Native Access 3.26.0 identity retained by NAUI2
- selected rendering: operation-scoped `--disable-gpu`
- selected dependency: exact installed NTKDaemon 1.32.0 image and
  `NTKDaemonService`

The real dependency operation proved exact readiness and both owned listeners. Its one
submitted SCM stop request produced `NAD2_STOP_SUBMITTED_NO_TRANSITION`: SCM remained
`RUNNING`, the exact process remained alive, and both listeners remained. Exact-owned
fallback cleanup removed the cohort and restored the bridge and keepers. That operation
correctly did not create a successful dependency-preparation receipt.

## 3. Remove the historical preparation gate from application launch

Native Access launch currently requires `prepared.json`, whose result must prove both
readiness and graceful service retirement. That is the wrong gate for an
application-owned dependency session.

Replace it with an exact **session admission** that validates:

- the canonical Native Access application identity;
- the canonical installed software generation;
- exactly one closed installation origin:
  - the retained installation-artifact record for the exact application and its
    historical software generation; or
  - while that artifact pointer remains absent, the fixed qualified-recovery record,
    including its exact historical operation source digests;
- the exact NTKDaemon image hash and size;
- the exact bundled installer identity;
- the expected environment and prefix;
- no alias, replacement, changed image, foreign-prefix daemon, deleted-prefix daemon,
  ambiguous candidate, or same-prefix unowned live generation.

The two origins are mutually exclusive. A present malformed or mismatched artifact
pointer refuses without recovery fallback, and appearance or disappearance of the
pointer during admission refuses. The recovery origin must not manufacture an artifact
pointer or preparation receipt.

The session-admission record must bind the selected origin into the exact renderer
operation. Rust and Python must independently validate the same exact origin and daemon
identity.

Do not treat a failed preparation result as successful. Do not synthesize or retain a
`prepared.json` record for this policy.

The standalone `DependencyPrepare` operation may remain as a strict diagnostic or
qualification action. It is no longer a prerequisite for opening Native Access.

## 4. Fresh readiness remains mandatory

Before Native Access starts, the operation-owned dependency owner must freshly:

1. query or start only `NTKDaemonService` through SCM;
2. bind the exact admitted Windows process generation;
3. bind the exact same-prefix Linux/Wine generation inside the renderer operation's
   owned cohort;
4. verify the admitted NTKDaemon image;
5. verify both expected loopback listeners are owned by that exact Windows generation;
6. reject foreign, deleted, ambiguous, unowned, changed, or unavailable candidates.

Historical installation or preparation evidence is never live readiness.

Native Access must not launch unless fresh readiness succeeds.

## 5. Application operation and browser return

Preserve the existing Native Access renderer behavior:

- exact application image verification;
- operation-scoped `--disable-gpu` and no `--no-sandbox`;
- exact renderer cgroup ownership;
- exact focus and Stop controls;
- browser-return delivery;
- diagnostic privacy cutoff before secret-bearing callback material is written to the
  Windows adapter;
- continuous runtime output draining;
- no independent normal-use deadline.

The daemon and application must remain under one application operation and one cleanup
boundary.

## 6. Session close policy

When the application closes or the operator selects exact Stop:

1. retire the exact mapped Native Access application generation;
2. submit at most one authorized SCM stop request for the exact daemon generation;
3. retain the NAD2 stop-response classification;
4. if graceful retirement confirms, complete the existing ordinary path;
5. otherwise, perform the existing bounded exact-owned cohort cleanup;
6. verify the post-cleanup result before completing the application session.

### 6.1 Graceful completion

The ordinary path remains:

```text
service_retirement_confirmed = true
process_cleanup_confirmed = true
forced_cleanup_used = false
```

### 6.2 Exact-owned compatibility cleanup

A Native Access session may complete after unconfirmed graceful service retirement only
when all of the following are true:

- dependency readiness was established for the exact admitted generation;
- the stop observation is present and classified, not unavailable;
- no second stop request was sent;
- the application itself reached a valid completed or operator-cancelled disposition;
- cleanup acts only on the exact operation-owned cohort;
- process cleanup is confirmed;
- the renderer unit/cgroup is empty;
- the exact NTKDaemon process generation is absent;
- both owned listener ports are absent;
- no matching foreign, deleted, ambiguous, unavailable, or same-prefix unowned daemon
  candidate remains;
- the Native Access application image and managed prefix remain unchanged except for
  expected vendor application state;
- bridge restoration and both keepers are healthy;
- no pending resume owner, stale transport, lease, or transaction remains.

The result must state truthfully:

```text
service_retirement_confirmed = false
process_cleanup_confirmed = true
forced_cleanup_used = true
dependency_cleanup_disposition = exact_owned_session_cleanup
```

This is successful **application-session cleanup**, not successful SCM retirement.

It must not:

- create or update `prepared.json`;
- claim `NAD2_STOP_CONFIRMED`;
- claim reliable or graceful NTKDaemon shutdown;
- generalize to another service, application, prefix, daemon image, or product.

### 6.3 Refusal conditions

The application session remains failed or cleanup-unconfirmed when any of these are
true:

- dependency readiness was not established;
- stop observation is unavailable or identity is unresolved;
- application launch/root confirmation failed;
- application failure occurred independently of cleanup;
- process cleanup is unconfirmed;
- any owned listener remains;
- the admitted daemon image changed;
- any foreign, deleted, ambiguous, unavailable, or unowned candidate remains;
- the renderer cgroup is not empty;
- bridge recovery is unconfirmed;
- private identity or callback custody is violated.

## 7. Result and operator presentation

Preserve top-level renderer terminal states (`completed`, `cancelled`, `failed`) so the
existing lifecycle manager remains compatible.

Add a closed dependency cleanup disposition, equivalent to:

```text
graceful_service_retirement
exact_owned_session_cleanup
cleanup_unconfirmed
```

The result must preserve the full dependency facts, including:

- `service_retirement_confirmed`;
- `process_cleanup_confirmed`;
- `forced_cleanup_used`;
- NAD2 classification and bounded observations;
- exact-owned post-cleanup verification;
- whether the session is restartable under generated qualification.

The frontend must distinguish:

- Native Access closed with graceful dependency retirement;
- Native Access closed with exact-owned compatibility cleanup;
- Native Access or dependency cleanup failed.

It must never label exact-owned cleanup as graceful service shutdown.

## 8. Generated qualification

Generated tests must cross the production Rust admission, production Python renderer
and dependency owners, result parsing, lifecycle terminal handling, and frontend
presentation.

Cover at least:

1. exact artifact admission and fresh readiness followed by normal graceful retirement;
2. qualified recovery admission with both `artifact.json` and `prepared.json` absent;
3. a full generated application session through that recovery origin;
4. present invalid artifact refuses without recovery fallback;
5. artifact pointer appearance or disappearance during admission refuses;
6. changed recovery source, installer, or daemon bytes refuse before launch;
7. wrong application/software/installer identity refuses before launch;
8. foreign or deleted-prefix daemon refuses before launch;
9. same-prefix unowned daemon refuses before launch;
10. ambiguous or unavailable process census refuses before launch;
11. submitted-no-transition followed by exact-owned cleanup completes the application
   session with truthful fields;
12. control refusal followed by exact-owned cleanup completes only under the same exact
   post-cleanup requirements;
13. STOPPED with process/listener residue followed by exact-owned cleanup completes only
    after residue is gone;
14. unavailable stop observation does not qualify compatibility cleanup;
15. process cleanup failure does not qualify compatibility cleanup;
16. listener residue after cleanup does not qualify compatibility cleanup;
17. foreign/unowned candidate after cleanup does not qualify compatibility cleanup;
18. application failure remains failure even when dependency cleanup succeeds;
19. operator cancellation remains cancellation, not completion;
20. no successful dependency-preparation receipt is created by either cleanup path;
21. a second fresh Native Access generated session succeeds after exact-owned cleanup;
22. browser-return privacy and continuous output draining remain intact;
23. Arturia application completion and ordinary VST paths remain unchanged.

Preserve meaningful failed qualification candidates rather than relabelling them.

## 9. Validation

Run:

- focused NAO1/NAD1/NAD2 runtime tests;
- renderer-session and renderer-CLI tests;
- manager and frontend suites;
- strict manager and frontend Clippy;
- Python compilation;
- Windows adapter/fixture cross-compilation;
- AP8, AP12, and PX2 at the exact executable source;
- final-head checks when the final commit is evidence-only.

AP10 remains out of scope unless native/audio source changes.

## 10. Hard boundaries

During implementation and PR qualification:

- do not install the candidate;
- do not launch the real Native Access application;
- do not transition the real NTKDaemon;
- do not run a DAW, plug-in, updater, or product installer;
- do not send more than one service stop request;
- do not increase the 12-second stop observation bound;
- do not use `net stop`, `sc stop`, direct daemon execution, `wineserver -k`, or
  process-name killing as a new production path;
- do not weaken exact identity, prefix, process-generation, listener, cgroup, or
  privacy checks;
- do not change the renderer policy;
- do not modify Arturia or VST behavior;
- do not rewrite prior evidence.

## 11. Delivery and immediate continuation

Return one draft PR, uninstalled and unmerged, containing the source, generated
qualification, exact source/head identities, validation, and explicit nonclaims.

After independent review and merge, the immediate next action is:

```text
install the exact merged generation
→ run one real Native Access owned session
→ verify rendered signed-in library behavior
→ exercise a bounded user-selected Native Access action
→ close or Stop the exact session
→ verify graceful or exact-owned cleanup
→ start a second short session to prove restartability
→ restore the bridge
```

That physical continuation is separately authorized after source review. Do not select
another daemon-engineering slice unless the real application session exposes a specific
new blocker.
