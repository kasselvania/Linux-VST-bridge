# Final recovery owner: saved no-reinstall mode

Executed source `634c17801e3ecb5dd8e4934bca69c2f36de44482`, tree
`82deae3eaa9c14f3742c7c25718990fc6cb46837`, sealed by
`dfc24971e65dc3aa312c3325c185651842d47daff479d2d158a05b0e357981d9`.

This generation closes the selection race found in final source review. Rust
persists `recover_installed` or `prepare` in schema-2 dependency specs before
submission. Python requires the closed saved mode. Missing/changed recovery files
can never select installation. A newly appearing unadmitted image cannot silently
change preparation into recovery. A broken artifact reference does not fall back.
Ordinary renderer specs remain schema 1; old dependency specs remain history.

The repeated sealed campaign used schema-2 saved recovery requests. Each seeded
fixture installer exited 100 after installing/registering its source-owned daemon.
The recovery operation never reran it:

| Case | Operation | Result |
| --- | --- | --- |
| Ready | `96d3ead24070ea4dc7f31c7f1807c36b` | completed, readiness verified |
| No readiness | `e6fb3d042abc0dc1be2d7ae5e86327a6` | failed, exact readiness cause retained |
| Exact manager Stop | `b9a4258d728ac01c83c7681bffe08f8a` | cancelled, no readiness claim |

All three requested SCM stop exactly once, confirmed service/process retirement,
used no forced cleanup and left no unit, cgroup or scratch prefix. First-generation
recovery evidence remains unchanged in the parent directory; no claim is made that
it exercised the later saved-mode law. Production-admission tests separately prove
schema/mode closure and the missing-file race without real application inputs.

Final readback reverified installed software, real daemon/registry bytes, original
real attempt sources, 307 witnesses, five projects, 23 predecessor files and 81
protected records. Native Access remains closed. Service active, two keepers, zero
leases/transactions/stale transports, capture off, resume absent. Runtime shared
cache ctime-only effects are reported separately in the campaign preservation data.

The source package and sealed private records remain retained. No proprietary
payload/log contents are published. No software installation or real dependency
transition occurred. Real service readiness and Native Access connectivity remain
untested; the next deployment can use this closed recovery path to test them.
