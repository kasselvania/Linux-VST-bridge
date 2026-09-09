# Compatibility Profiles

Compatibility profiles are portable, closed-schema policy. They bind exact observed module/class/build and environment/runner/host/native constraints to separately implemented and reviewed capabilities. Local environment IDs, absolute paths, publication targets, process IDs, credentials and license state remain local bindings rather than portable profile identity.

Profiles must not contain arbitrary commands, scripts, hooks, executable expressions, credentials, proprietary binaries or state, DRM workarounds, unreviewed downloads, destructive repair actions, or an untyped capability escape hatch. Schema versions, identities, hashes, strings, collections and nesting are bounded. Unknown fields, unsupported capabilities, malformed identities, duplicate profile revisions and zero/ambiguous exact matches fail closed.

## Claim lifecycle

```text
review_candidate
-> locally exercised against an exact matrix
-> evidence retained
-> independently reviewed
-> verified_exact_fixture
-> superseded / withdrawn when appropriate
```

`Claim` and `SelectionPurpose` own eligibility. Ordinary managed preview/publication and current installed-host policy require `verified_exact_fixture`. `review_candidate` may be evaluated only through the explicit nonactivating qualification route; `withdrawn` authorizes no new selection. Exact retained history is separate: rollback may activate an immutable prior revision only through the existing artifact, ancestry, physical-target, performance and lease laws.

## Accepted AP14 profiles

The two JSON files in this directory are immutable schema-1 revision-3 `verified_exact_fixture` profiles for the exact installed Pure LoFi and Efx FRAGMENTS Steam Deck/Bitwig fixture. They retain the exact module/class/vendor/build/role, runner/environment/host/native/descriptor constraints, process-scoped accessibility choice, AP13 state behavior, float32-only scope, 512 recommendation, 256 nonqualification and known limitations.

Revision-2 `review_candidate` profiles remain byte-for-byte in regression fixtures and installed publication history. They were not relabeled or overwritten.

The AP14 editor capability is explicitly:

```text
editor = detached_owner_thread_with_native_panel
```

That means the verified revision accurately includes the visible native open/focus/close intermediary. It must not be silently reinterpreted after AP15.

## AP15 transition

AP15 changes the native/editor behavior to a direct detached vendor-window lifecycle. Any changed native artifact or editor contract requires:

- a new closed editor capability value;
- a new immutable profile revision and exact native/source identity;
- preserved revision-3 known-good publication and rollback target;
- candidate qualification without weakening ordinary activation authority;
- exact Steam Deck evidence and independent review before a new verified claim.

A candidate build may be deployed only through an explicit bounded, reversible engineering qualification path. It must not become normal activation authority merely because it is committed or embedded. The final verified revision is selected after review; candidate and verified records remain immutable.

A profile claim never generalizes across a changed module digest, runner, environment, host/proxy revision, DAW, machine or capability matrix. Repository profile data is not permission to copy or redistribute a matched commercial module.