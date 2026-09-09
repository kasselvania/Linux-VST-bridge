# Compatibility Profiles

AP14 / [#83](https://github.com/kasselvania/Linux-VST-bridge/issues/83) owns the first versioned declarative profile schema and the first exact profile-driven publication workflow for the installed Pure LoFi and Efx FRAGMENTS fixtures.

[technical review 5158330660](https://github.com/kasselvania/Linux-VST-bridge/pull/85#pullrequestreview-5158330660) supports the AP14 architecture and exact fixture evidence and selected the R1 claim-lifecycle repair. Revision 3 records verification only for that exact fixture. The repaired implementation still requires focused rereview; no final merge acceptance is claimed. Committing a review candidate never grants ordinary activation authority.

## Profile boundary

A profile is portable, closed-schema policy. It may bind exact observed match conditions to a bounded set of separately implemented and reviewed capabilities, such as:

- stable profile identity and immutable revision;
- exact vendor/product/module build and VST3 class/metadata/role conditions;
- supported runner/environment/host/proxy constraints;
- detached-editor and process-scoped accessibility posture;
- supported performance and protocol/state capabilities;
- known limitations, fixture matrix and non-authoritative evidence references.

A local binding or publication receipt owns machine-specific environment IDs, absolute paths, installed artifact paths, module census results and active publication targets. Those do not become portable profile identity. Friendly names are presentation, not dispatch authority.

Profiles must not contain:

- shell, PowerShell, Python, JavaScript or other arbitrary executable code;
- generic command templates, hooks, expressions or an untyped capability escape hatch;
- credentials, tokens, cookies, serials or license data;
- proprietary binaries, presets, opaque vendor state or patches;
- DRM or authorization bypasses;
- unreviewed remote download URLs;
- destructive vendor/environment repair actions;
- machine-local mutable paths presented as portable identity.

Schema versions, identities, hashes, strings, collections and nesting are bounded. Unknown fields, unsupported capabilities, malformed identities, duplicate profile revisions and zero/ambiguous exact matches fail closed before publication mutation.

## Lifecycle

```text
proposed
-> locally exercised against an exact matrix
-> evidence retained
-> independently reviewed
-> verified for that exact matrix
-> superseded / withdrawn / known-regressed
```

A profile claim does not generalize across a changed module digest, runner, environment, host/proxy revision, DAW or capability matrix. Withdrawal must prevent new selection without deleting the exact prior local publication needed for safe rollback.

See [CURRENT_SLICE.md](../CURRENT_SLICE.md), the [design dossier](../docs/DESIGN_DOSSIER.md), [architecture profile boundary](../docs/ARCHITECTURE.md#9-compatibility-profiles) and issue #83. The implementation should create only the smallest profile layout needed by the accepted AP14 contract; do not prebuild an empty marketplace hierarchy.
## AP14 exact-fixture profiles

The two JSON files implement schema 1, immutable revision 3 and `verified_exact_fixture`. They retain every exact revision-2 module/class/vendor/build/role, runner/environment/host/native/descriptor constraint, capability and limitation. AP14 publication/rollback and installed-fixture evidence references were added. Revision-2 `review_candidate` profiles remain byte-for-byte in Git, regression fixtures and installed immutable publication history; they were not relabeled.

`Claim` owns eligibility through typed `SelectionPurpose`. Ordinary managed preview/publish and current installed-host policy require `VerifiedExactFixture`; `ReviewCandidate` and `Withdrawn` cannot authorize activation. `managed check-candidate PROFILE` may exactly qualify a review candidate but always returns `activation_permitted=false`, and creates no intent, publication revision, candidate bundle, registry mutation or discovery link. A fresh supervised census may be retained as observation data.

Retained-history loading/rollback is separate from new selection. An exact prior candidate revision remains eligible under the existing complete immutable record, artifact, ancestry, physical target, protocol/state and inactive-lease laws. Current service policy still requires exactly one verified profile, including same-host admission and the bounded retained older-host route.

This verifies no additional Arturia build, environment, runner, DAW, machine, precision or routing configuration. Float32 and 512 frames remain supported/recommended; 256 remains unqualified. See [implementation and verification](../docs/AP14.md).
