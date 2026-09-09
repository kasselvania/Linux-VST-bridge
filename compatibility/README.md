# Compatibility Profiles

AP14 / [#83](https://github.com/kasselvania/Linux-VST-bridge/issues/83) owns the first versioned declarative profile schema and the first exact profile-driven publication workflow for the installed Pure LoFi and Efx FRAGMENTS fixtures.

**No AP14 profile is accepted or verified until its implementation, exact local fixture result and technical review are complete.** Files created on the implementation branch are candidates, not support claims merely because they are committed.

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
## AP14 candidate files

The two JSON files in this directory implement schema 1 with closed capabilities and exact local-fixture constraints. Both remain `review_candidate`; neither is a universal Arturia support claim. The manager embeds this reviewed candidate set; arbitrary diagnostic profile input cannot activate a publication. See [implementation and verification](../docs/AP14.md).
