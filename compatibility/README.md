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

`Claim` and `SelectionPurpose` own eligibility. Ordinary managed preview/publication and current installed-host policy require `verified_exact_fixture`. `review_candidate` may be evaluated through the nonactivating `managed check-candidate` route; only the separately bounded AP15 engineering route described below may exercise a candidate publication; `withdrawn` authorizes no new selection. Exact retained history is separate: rollback may activate an immutable prior revision only through the existing artifact, ancestry, physical-target, performance and lease laws.

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
### AP15 implementation checkpoint

The hosted gate was cleared in technical review `5161191208`. The exact revision-4 profiles in `ap15/` now populate the finite sealed roster as `review_candidate`; the additional closed limitation `direct_editor_under_qualification` describes their current claim. Revision 3 above remains byte-identical and the only ordinary activation authority. Every earlier limitation is retained, including FRAGMENTS redraw. The qualified parent comparison permits only this exact appended engineering limitation, not removal or replacement of existing limits.

`tools/ap15_package.py` assembles the existing `qualify-editor stage` directory layout from the verified GitHub ZIP and exact retained native bytes. Its host-source manifest records the real build commit separately from the reviewed head and common tree; `SOURCE_COMMIT.txt` is never rewritten. Package paths supply bytes only; compiled profile hashes remain the authority. `qualify-editor publish` and `restore` use the existing transactions/physical parents. This is engineering qualification, not ordinary support or acceptance.

Historical source-only checkpoint (retained below):

`detached_direct_vendor_lifecycle` is now a closed editor capability. The ordinary installed profiles remain the byte-identical verified revision 3 above. There is currently **no complete AP15 candidate profile**: its exact Windows/native artifact roster awaits the Windows build. The separate engineering `qualify-editor` command therefore returns `qualification_artifacts_pending` before publication work.

When the finite compiled roster is complete, this engineering route can activate only an exact review candidate descended from the installed verified revision-3 parent. The immutable revision is explicitly marked `ap15_editor` qualification, never ordinary support. Reconcile/service startup restores its exact verified parent through retained-history rollback, subject to inactive DSP and physical ownership checks. Normal `managed preview/publish` eligibility and nonactivating `managed check-candidate` are unchanged. A later independent review must select a new immutable verified revision; neither candidate nor revision 3 is relabeled.

AP15 source repair tightens **runtime serving**, including the same-host case: retained review candidates require the exact `ap15_editor` qualification marker, direct-editor capability, compiled sealed artifacts and verified revision-3 parent. Retained revision-2 history remains immutable and rollbackable under existing physical/ancestry laws; it cannot grant serving authority merely by being retained. The compiled AP15 roster remains absent while Windows builds are blocked. No AP15 publication or support is installed by this repair pass.
