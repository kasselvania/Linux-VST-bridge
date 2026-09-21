# FRG1 — Efx FRAGMENTS 1.3.1.6566 coherent product profile

FRG1 is the canonical-product handoff selected by the accepted Ubuntu UA1 result.
It does not deploy to Ubuntu and never contacts the Steam Deck. Its authority is the
exact input lock at `compatibility/frg1/input.lock.json`.

## Attempt 001 disposition

The exact retained Efx FRAGMENTS module was copied read-only from its Ubuntu-lab
custody path into a mode-700 disposable `/tmp` root. The original module remained
byte-identical. The reviewed Windows host and source manifest were downloaded from
their exact accepted Actions artifact. Before launch, the product supervisor
recomputed and matched:

- module, host and source-manifest bytes;
- GE-Proton11-7 archive and ownership receipts;
- the 8,900-entry installed runner tree;
- the 638-entry Ubuntu runtime overlay tree;
- the fixed runner entry files;
- the process-scoped `uiautomationcore` policy;
- the closed input roster.

The census ran inside a fresh Bubblewrap namespace with a synthetic home, a new
prefix, no host-home mount and no shared network. It did not use the retained ASC
prefix or launch ASC, Bitwig, the manager, a published proxy or any Steam Deck path.

The Windows scanner did not publish its exact ready handshake within the declared
90-second bound. The supervisor stopped its namespace. The post-failure readback
found zero operation-owned processes, no ready or gate file, an unchanged original
module, zero Ubuntu-lab publication payloads and the existing manager service still
active. The disposable prefix reached 2,079 entries before the timeout.

Attempt 001 is therefore:

```text
FRG1_FACTORY_CENSUS_READINESS_TIMEOUT
```

No factory/class result exists. Historical 1.0.0.2925 metadata is not substituted.
No native proxy or compatibility profile is generated, and no Ubuntu continuation
is authorized from this result.

The attempt exposed one supervisor evidence defect: its readiness-timeout path
terminated the process but did not retain the bounded stdout/stderr streams. The
source repair retains those streams privately and emits a sanitized failure record;
it does not authorize another product execution. Any retry requires independent
review of this retained first failure.

## Nonclaims

FRG1 does not establish factory discovery, class identity, controller association,
DAW discovery, editor, audio, parameters, automation, state, restart, retirement,
ordinary publication, Ubuntu compatibility or Steam Deck compatibility for this
module. The exact admitted inputs remain useful custody facts, not a coherent
compatibility profile.

## Evidence

- `evidence/frg1/attempt-001/result.json`
