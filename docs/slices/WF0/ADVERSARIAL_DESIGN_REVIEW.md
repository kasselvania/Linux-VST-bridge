# WF0 V1 Adversarial Design Review

## Review identity

```yaml
reviewed_design_commit: aa07e63368dddc2fdb51242326a0de127c6ffbee
reviewed_design_tree: 8583a0a4846cae953af515aee350d8d3a7cb7c30
reviewed_design_path: docs/slices/WF0/IMPLEMENTATION_DESIGN.md
reviewed_design_blob: acfb73f962fcd8423b936c67f8011913136dd701
reviewed_design_sha256: f8e4d92602810ce78f0817ca55b06fc435d21befb933290511e26f996eac98b3
reviewed_revision: wf0-design-v1
github_review_id: 5082639684
review_result: DESIGN_REPAIR_REQUIRED
implementation_authorized: false
```

This file materializes the independent owner review supplied as GitHub review `5082639684`. The design-authoring agent did not independently review or approve its own card. The V1 architecture remains sound and bounded; four connected attribution corrections are required before a new independent review may consider V2.

## Finding 1 — Missing pre-call attribution

**Violated invariant:** A retained stage result must identify the exact untrusted module or factory call that was in flight. The scanner must publish that identity before control can enter code that may hang, crash, terminate, or fail to return.

**Concrete failure or false-acceptance state:** V1 can leave `module_opened`, `factory_obtained`, or another prior success as the last validated stage while `InitDll`, `getFactoryInfo`, a factory-interface query, `countClasses`, an ordinal/tier class-info call, interface `release`, `ExitDll`, or `FreeLibrary` is actually in flight. A timeout or abnormal termination can therefore be assigned to the wrong owner, and the retained timeline can falsely imply that the later call never began.

**Required V2 change:** Add one synchronously flushed, monotonically sequenced `call_started` record before every potentially blocking module/factory call, using a closed operation enum and exact interface/ordinal/tier fields. Add one exact completion/result record for ordinary returns. Classify timeout, exception, NTSTATUS exit, signal, malformed termination, and output closure from the last fully validated attempt. Add `WF0_FACTORY_RELEASE_BLOCKED` without allowing cleanup failure to erase an earlier primary failure.

**Adjacent operations audited under the same invariant:** `LoadLibraryExW`, optional `InitDll`, required `GetPluginFactory`, `getFactoryInfo`, both factory `queryInterface` calls, `countClasses`, every class-info tier at every ordinal, release of factory 3, factory 2, and base acquisitions, optional `ExitDll`, and `FreeLibrary`; scanner result mapping, supervisor timeout law, stage-timeline evidence, crash/hang fixtures, and cleanup precedence are included.

**Additional live reconnaissance required:** false

## Finding 2 — DLL-search API contracts conflated

**Violated invariant:** Dependency-search authority must be explicit, valid for the exact Win32 API being called, and independently checked before untrusted module code can run.

**Concrete failure or false-acceptance state:** V1 associates `LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32` with both `SetDefaultDllDirectories` and `LoadLibraryExW`, although `LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR` is not a valid `SetDefaultDllDirectories` flag. An implementation following that wording can fail before load or can repair the mistake by silently widening search authority to the application directory, user directories, current directory, or ambient `PATH`.

**Required V2 change:** Require checked `SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32)` followed by checked `LoadLibraryExW(exact_absolute_module_path, nullptr, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32)`. Retain only the operation, bounded unsigned `GetLastError`, and exact scanner/module identities on failure. Prohibit altered-search-path, application-directory, user-directory, current-directory, and ambient-`PATH` authority.

**Adjacent operations audited under the same invariant:** Canonical Windows module-path validation, copied runtime-DLL closure, loader return handling, identity ledger, module-open transition, loader-adapter negative tests, and sanitized evidence fields are included.

**Additional live reconnaissance required:** false

## Finding 3 — Conflicting first-prefix initialization ownership

**Violated invariant:** Every physical mutation and workload must have one declared owner and one ordered transition; no undeclared bootstrap process may prepare state outside the supervised scanner session.

**Concrete failure or false-acceptance state:** V1's lifecycle copies and verifies payload before the Runtime/Proton/scanner launch, while mutation operation 7 also says it initializes compatdata. Following both descriptions either invents an undeclared `wineboot`, `cmd.exe`, Proton, Wine, or other bootstrap workload, or makes prefix mutation appear to belong to `WF0ScanEnvironment` when it actually occurs during the supervised launch.

**Required V2 change:** Freeze the sequence as marker creation, payload copy/verification, `environment_payload_ready`, and then one exact Runtime 4/Proton 11 scanner launch that owns all required first-prefix initialization. Apply the 180-second first-prefix spawn-to-readiness bound to that launch. Classify staging/copy failures as `WF0_SCAN_ENVIRONMENT_BLOCKED` and initialization/scanner-start failure before readiness as `WF0_SCANNER_LAUNCH_BLOCKED`.

**Adjacent operations audited under the same invariant:** Environment lifecycle, mutation ledger, legal transitions, process topology, readiness identity, prefix-root confinement, accepted-WR0 preservation, timeout law, cleanup, and retry reconciliation are included. A separate ungated Windows bootstrap run remains prohibited.

**Additional live reconnaissance required:** false

## Finding 4 — Incomplete implementation-source and causal proofs

**Violated invariant:** A live result must be causally bound to one exact clean committed implementation and must prove both that the gate controls module loading and that the factory-only path never reaches class creation.

**Concrete failure or false-acceptance state:** V1 names a source manifest but does not define its schema or complete governed path roster. A final evidence-only head can therefore appear acceptable after source/configuration drift, an extra tracked WF0 source can escape the manifest, pre-gate waiting can be inferred without proving AGain is unmapped, and `no createInstance` can remain a source-reading assertion without a causal tripwire.

**Required V2 change:** Define `linux-vst-bridge-wf0-implementation-source/v1` over the exact lexically sorted 26 non-evidence paths, binding path, Git mode, and Git blob at the clean implementation commit and requiring byte/digest equality at the final evidence-only head. Invalidate all live results after any governed-source change. Add a real held-gate proof with no module-open attempt or AGain mapping and a source-owned class-creation tripwire that completes census with `create_instance_called=false`.

**Adjacent operations audited under the same invariant:** Source/configuration roots, clean-commit build entry, artifact manifest separation, two-build receipts, final evidence commit, extra/missing-path rejection, scanner command/schema surface, module-map observation, gate-held cleanup, tripwire fixture, evidence schema, and protected-WR0 equality are included.

**Additional live reconnaissance required:** false

## Disposition

```text
DESIGN_REPAIR_REQUIRED
```

The repair must preserve the WF0 primary claim, AGain fixture, Runtime 4/Proton 11 route, exact MinGW selection, ten-owner architecture, disposable-environment law, normalization bounds, 40-path implementation envelope, accepted-WR0 protection, and factory-only ceiling. V2 requires a fresh independent adversarial review. This record grants no implementation or approval authority.

```text
implementation_authorized=false
```
