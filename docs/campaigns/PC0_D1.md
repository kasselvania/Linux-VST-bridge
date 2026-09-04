# PC0-D1 — Run the Existing AGain Probe

## Authority

```yaml
status: active_diagnostic_campaign
authority_phase: proof_harness_maintenance
change_class: PROOF_HARNESS_MAINTENANCE
campaign_id: PC0-D1
repository: kasselvania/Linux-VST-bridge
product_implementation_authorized: false
maintenance_implementation_authorized: true
live_execution_authorized: true
permitted_execution_class: DIAGNOSTIC_NON_AUTHORITATIVE
classified_backend_core_ready: true
classified_backend_ready: true
acceptance_eligible: false
authorized_source_commit: 2ff23fe74b5501b633b575f7a903289ec2db1866
authorized_source_tree: c82270184beab47bcfd4e9676a701bede8e9df14
authorized_plan_id: pc0-pre-setup-processing-contract-diagnostic-v1
authorized_product_contract_identity: pc0-selection-v2
authorized_product_contract_sha256: daa042e4184cb5fffdf1ff08d59cc51f7755b4635c85e02597adc2584c4b4c1d
authorized_plan_content_sha256: a5303e12d644fefba2ca4003555ebe30f578e60c0a334e5f0bb96b7498decc92
diagnostic_campaign_identity: be62c45243de799f2474b26d4b20ac3593f16fc0392f58f42a10ce922500281b
diagnostic_batch_budget: 2
diagnostic_campaign_authorized: true
acceptance_candidate_authorized: false
windows_workload_authorized: false
deck_workload_authorized: true
accepted_product_frontier: WA0
current_product_target: PC0
```

## Finish this job, not another process task

The operator directed the technical lead to remove the repeated handoffs and
get the project moving. This ruling supersedes D1's previous execution-only
restriction and its requirement to stop for separate authority after an ordinary
preflight or diagnostic-harness defect. The same agent owns investigation,
narrow local repair, targeted tests, and the existing plug-in diagnostic.
Do not select another infrastructure slice, rewrite PC0, or reopen product design.

Immediate objective: load the retained AGain probe, initialize it, observe its
pre-setup bus/sample-format contract, and report the actual result. Reuse artifact
9915439437, the accepted AGain fixture, and the existing containment mechanisms.
No Windows producer, artifact download, custody reconstruction, fixture reseed,
installed-file restoration, acceptance execution, or product evidence is authorized.

## Troubleshooting and maintenance scope

Read-only investigation of connectivity, files, declarations, stores, processes,
and underlying error chains is already authorized. A prerequisite failure is a
reason to investigate, not to ask the operator to say "continue" again.

Diagnostic Python error reporting, verifier metadata handling, orchestration,
and their focused tests may be repaired locally while product calls, artifact,
fixture meaning, process ownership, cleanup, and security boundaries stay unchanged.
Commit actual source before a diagnostic. Bind its commit/tree in this same receipt
on the authority branch; do not reset the campaign or rewrite old observations.
These diagnostic-only edits may be exercised before PR merge after targeted tests.
One focused code review is required before their eventual merge, not a separate
selection/design/approval sequence before every development observation.

For the Steam manifest mismatch: inspect the actual changed file and remaining
locked files, and compare parsed application/build/depot/install/runtime selection.
Do not assume a size mismatch means either corruption or harmlessness. If the
only established difference is incidental metadata, a diagnostic-only verifier
may record that metadata separately while enforcing the same declared executable
and launch-relevant inputs. Record observed identities truthfully; never return
the old full snapshot digest as though changed bytes matched. Do not alter Steam
files, blindly update hashes, bypass checks, or monkey-patch a verifier. Unknown
launch-relevant differences or changed executable/deployment identities require
a concrete explanation and decision; they are not silently covered by this ruling.

## Use the existing command

Use a clean checkout of authorized_source_commit and this receipt from the current
authority checkout. The normal command now supports explicit scoped authority:
`python3 tools/proof-run.py diagnose --authority <this-receipt> --source <source> --plan <plan> --campaign <campaign>`.
Append `--preflight-only` for read-only checks: that path cannot create a reservation
or launch a workload. Do not use the old downloaded D1 launcher or invoke legacy
host-proof.py run. The default CURRENT_SLICE CLI circuit breaker stays disabled.

The two-reservation campaign ceiling is unchanged. Read-only checks consume neither
slot. Same-reservation recovery never relaunches; a second reservation is for a
specific tested repair, never blind repetition. No new campaign resets the count.

## Completion

Retain one bounded private observation/failure pair and truthful cleanup, protected
state, and effect counts. After required cleanup, validate that pair once and
report at most 15 lines. No success packet, full audit, or polished failure dossier.
Ordinary troubleshooting is part of this job. Escalate only a genuinely new scope,
safety, deployment, or budget decision, stating exactly what decision is needed.
Even diagnostic success does not advance the accepted frontier beyond WA0.

## Diagnostic metadata repair binding

The source above separates observed Steam bookkeeping from the unchanged declared
runtime inputs. Read-only comparison found 32 of 33 locked files exact; only the
Proton app manifest differed (710 to 691 bytes). Installed app/build/depot/install
selection and both tool manifests matched; Steam recorded a pending target build.
The diagnostic verifier rejects changed deployed files, alternate selections,
unknown manifest fields, active download/staging state, and non-regular files.
It records the actual complete snapshot digest and declared-input digest separately.
The historical WR0 verifier and all installed files remain unchanged.

Diagnostic copies of five frozen PC0 functions accept the observed runtime identity
explicitly. A structural regression compares their bodies with stopped source
309b8918c128c0b9e6701d0453dc841a111d5ac5, permitting only runtime dependency changes;
product calls, supervision, containment, and retirement logic remain identical.
The verified stdin worker carries these helpers without changing the Deck worktree.
Targeted runtime/adapter/CLI/policy/backend tests: 52 passed before this source bind.
The campaign identity and two-reservation ceiling above are unchanged.

## Remaining-slot repair binding

The first reservation, 4008dc6b0628dea1157628a3e04108caaba478e6954c56b900a37bb0c3d65a6f,
is closed at source 1e4be8a1386385380b7d240241d3cabd6793cea2. The host reached
readiness; supervision stopped before any plug-in call, with complete containment
and retirement and unchanged protected state. One slot was consumed, one remains.
The retained observation is not rewritten or relabelled.

The source above repairs the missing runtime-identity argument at the ready-gate
verifier and retains bounded sanitized supervision errors. A mocked-process test
reproduced refusal before gate publication and passed after the exact call-site
repair. All 54 runtime/adapter/CLI/policy/backend tests passed. The remaining slot
is bound to this specific repair; this is not a blind repetition or budget reset.
