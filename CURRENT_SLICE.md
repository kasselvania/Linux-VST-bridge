# CPI2 — catalogue-free exact quarantine retry

Base: canonical `main` commit `9f72fb27e5f1399e1533d678824d5bde81c77ade`,
tree `6c890fe843fab990fe8b921262185e0cde953ed8`.

## Primary claim

An exact `QuarantinedModuleRetry` offered for a current quarantined inventory in
the sealed, adopted, prepublication FRG1 state can execute without an ordinary
native catalogue. It uses the same managed-environment authority as ordinary
`EnvironmentRescan`, retains one final `registry.lock` through scan and inventory
replacement, and preserves all prior scan/module/report identity checks.

## Scope and basis

- `docs/ARCHITECTURE.md` sections 5.3 (Environment manager) and 5.6 (Scanner service);
- existing FRG1 adoption and catalogue-free registry rules;
- manager operator action projection, worker dispatch, exact retry, inventory mutation;
- source-owned deterministic manager tests.

Only the canonical product manager and test fixture may change. The Ubuntu
adapter PR #5, installed generation, runner alias, vendor prefix, scanner,
Windows host, native proxy, and commercial module are outside this slice.

## Acceptance

- With no native catalogue, exact adoption, valid empty prepublication registry,
  and one current quarantined inventory, the snapshot offers no ordinary refresh
  and exactly one enabled identity-bound retry.
- The real operator-worker route performs exactly one selected-module scanner
  launch, preserves unrelated module records, replaces current inventory,
  retains the failed inventory in history, and removes the retry action after a
  healthy report.
- Missing or changed adoption; foreign environment, scan, module, report, host,
  or source identity; and a foreign populated catalogue-free registry refuse
  before scanner launch and without inventory mutation.
- Manager library and binary tests, strict manager Clippy, AP12 manager and
  frontend, and PX2 pass. Test stubs prove control-path behavior only, not a
  successful Wine load or Ubuntu physical qualification.

## Stop and handoff

Open one canonical product PR against `main` and leave it unmerged for review.
Do not deploy or retry on Ubuntu until this product PR is independently reviewed
and merged. Ubuntu PR #5 remains draft and unmerged at
`f22c30908400ab9ab59a87bd4c9e60f7cc84c4d7`.

## Current status

The source-owned binary fixture passed the exact retry and ten refusal cases.
Default manager library tests passed 160/160; default binary tests passed
112/112; the opt-in binary fixture suite passed 114/114. Manager and frontend
strict Clippy passed; frontend tests passed 37/37; PX2 policy tests passed
122/122. The AP12 runtime suite passed 294 tests with 51 Linux-only skips on
macOS. A concurrent, unchanged native backend test run had one instance-capacity
failure; the affected test passed alone and the serial backend suite passed
73 tests with one ignored. None of these runs executes Wine or qualifies Ubuntu.
