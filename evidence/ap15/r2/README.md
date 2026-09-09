# AP15 focused R2 source repair

Technical review [5160946354](https://github.com/kasselvania/Linux-VST-bridge/pull/86#pullrequestreview-5160946354) selected only the XCB operation-state correction. Other source-hardening repairs are unchanged. Earlier evidence is retained in `../source-repair/`.

- [Exact source, commands and results](validation.json): same final production fixture fails against the reviewed header and passes against the repaired header. Full Linux native lane 6/6, backend 57, manager 31+3, both Clippy checks and Linux runtime 15 pass. macOS runtime separately skips 9 Linux-only cases.
- [Before](r2-exact-before-1.log) and [after](r2-exact-after-1.log): rejected Unmap on a live parent still yields one checked WM_DELETE after repair. Duplicate turns and exact removal/reference/connection-selection balance are checked. Exhausted Unmap and rejected SendClose are distinct.
- [Linux artifacts](native-artifacts.json): both registered products rebuilt through the retained exact route, not installed or published.
- [Installed readback](installed-state.json): revision-3 profiles, physical targets/parents, stable IDs, vendor/native/runner bindings, project hashes and retained records unchanged before/after and from the previous repair. Service/keeper active, zero DSP leases, no pending transaction, tracing off, CPUWeight unset/effective 100.

`r2-red-2.log` preserves the first insufficient one-shot fault (a later successful unmap masked the problem); `r2-red2-2.log` preserves its corrected persistent rejection. The final exact test was then run with both headers, with identical test SHA-256. These are Xvfb/SDK mechanics only. Retained logs redact local paths and normalize trailing whitespace.

Windows compilation/tests and actual candidate Bitwig qualification remain blocked and unclaimed. No complete candidate profile, finite roster, qualification activation, AP15 publication or acceptance. No Windows source changed in R2; previously added Windows fixtures remain pending. 512 remains selected/supported/recommended and 256 unqualified. Keep PR #86 draft, open and unmerged.

AP15_SOURCE_CLEAR_WINDOWS_BLOCKED
BITWIG_QUALIFICATION_BLOCKED
AP14_BASELINE_PRESERVED
