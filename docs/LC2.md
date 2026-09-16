# LC2 — Correct runtime generation for processing restart

LC2 starts from merged `be9a7e9` and integrates the installed MF3 history/readback
repair from PR #110 at `e662c2d` as an explicit dependency. It does not merge that
PR on GitHub or replace its fixes with the older merged manager.

## What the retained failure selects

Candidate A's Windows host SHA-256 is
`3ee36fd34384b2c282ec9cb6ae6c48d17f83940eaaf6d8f9a1fa130b65274939`.
Its exact retained source manifest identifies `46c7b85fce2370644cccec6197a6f1a06c188be1`,
CI run `34418620336`. That processing owner predates the LC1 correction
`c7a58b63a65a7fdc7564cf6c29be272960d9871a`: after Activate it unconditionally creates
a worker that expects Start. It cannot accept an activation-only interval.

The privately retained native report independently records worker operation 14
(Deactivate), while the Windows report records a new worker and lifecycle
correlation failure. This is the same protocol boundary previously reproduced
with the account-free LC1 fixture. The old Windows report did not retain the
actual received tuple; the identification of the kind mismatch combines native
observation and exact old source, not a fabricated historical wire capture.
No vendor exception or preset-complexity cause is established.

The installed MF3 schema-2 kit already contains the corrected host
`348a4bbc6ea34f57fc5899c279d9e43999bf9ecae4563a6cfee88967d36f66be`, built from
`9434f8289be1f04206e20b76e62a3a1b958e3040`. Its exact lifecycle source matches
merged main (with the recorded Windows checkout line endings). The correction
selects Start or Deactivate on the owner before creating any processing worker.
It preserves session, sequence and epoch checks. LC2 therefore adopts the
existing runtime correction into a new candidate rather than adding another
workaround or weakening correlation.

## Source-owned proof

The actual native queued worker and Windows MappedSession/SDK owner ran under
the pinned runner in a new private scratch prefix, without a plug-in or DAW.
The existing normal restart, activation-only interval and deliberately wrong
sequence cases passed. Four processing blocks returned exact expected samples
in each positive case. The activation-only interval produced zero processing
calls and did not advance sequence or epoch. A wrong Deactivate sequence remained
refused. Every owned test unit retired with an empty cohort.

`evidence/lc2/generated.json` retains bounded scalar lifecycle records and exact
fixture identities. `tools/lc2/check_fixture.py` validates these facts without
turning process exit alone into a pass. Tests reject absent cleanup, wrong
correlation, an invented processing interval and false normal closure after a
malformed request. `fixture_launcher.py` uses private HOME, the pinned public
prefix initialization route and the existing supervisor environment constructor.
The native integration entry is
`queued::lc1_tests::same_session_reconfiguration_two_ended` with its existing
`LVB_LC1_PRESTART_DEACTIVATE` / `LVB_LC1_BAD_DEACTIVATE_SEQUENCE` switches.

## Exact-class preparation handoff

The first new-host inspection request was correctly bound to the selected Serum
instrument in the manager, but MF3 constructed its supervisor job with
`first_audio=true`. The scanner refused the multi-audio-class module rather than
guessing. This is a manager inspection handoff failure, not another Serum crash.
Its report and positive cleanup are retained; candidate A and publications did
not change.

The selected inspection owner now constructs an exact-class job. A regression
first failed on the old flag, then proves the persisted supervisor job retains
the exact class and disables factory-first selection. A runtime test independently
checks that both instrument and effect identities reach the command and handshake.
No product-specific dispatch or filename inference is added.

## Candidate and acceptance boundary

Candidate B must bind a fresh selected-class inspection under the corrected host,
its schema-2 recipe, exact native artifact and original instrument/environment.
Candidate A remains immutable with its failed processing_restart observation;
B does not inherit that failure or the old positive product observations as new
product measurements. The selected instrument and FX remain distinct.

No ordinary publication or commercial session is part of this preparation proof.
The source-owned restart result is not a claim that all observed Serum crashes
are fixed. A later deliberately enabled candidate-B session must establish its
real product behavior, preserving IF1/IF2/CA1 and positive retirement custody.

## Completed preparation

The corrected selected inspection completed with a separate exact controller.
The installed manager then created candidate B
`f6af02eba109d3632b2ecc786f2c9006ec6bc1d433772001d24d2969fb944b44` through
its offered `PluginPrepare` operation, using the schema-2 kit without another
product-specific route. B is a distinct immutable generation with candidate A as
predecessor. Neither candidate is published. The new native hash, descriptor,
inspection, source/recipe identities and healthy final state are in
`evidence/lc2/result.json`. The old failed inspection is retained separately.

This completes the lifecycle attribution, generated restart proof and corrected
candidate preparation. Product behavior of B is deliberately untested. The
manager's experimental controls still allow exact historical A; a later test must
select B explicitly and retain its own product evidence.
