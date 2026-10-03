# Presentation gaps with sample tracing disabled

The queued callback retains the first 16 missing presentation spans in fixed
instance storage. It does not change the delivery result, delay, epoch, silence
posture or expiry of late results. Further missing spans increment an omission
counter. The existing whole-lifetime delivery totals remain authoritative for
the number of missing frames.

At instance close, after callback leases retire and the worker joins, the native
report receives `audio_presentation_gap` records and an
`audio_presentation_gap_summary`. These records do not require `trace-enable`,
an observer thread, audio copies, SDK timing queries or a diagnostics reader.
The callback performs bounded scalar/atomic reads and two monotonic clock reads
only when retaining a missing span. Formatting and file access occur at close.

Schema 1 records:

- `clock_monotonic_ns`: the bracket around the progress reads immediately after
  output absence was observed; zero on platforms without the Linux clock adapter.
- `generation`, `epoch`, `position`, `frames`: the missing source sample span,
  before bridge presentation delay is added.
- `parent_callback`: host call number, actual host frames, chunk offset and
  host-entry monotonic timestamp, using the existing callback identity.
- `worker_thread`: Linux TID in the native process's PID namespace, captured once
  on worker entry; zero if absent. Correlating a host-side scheduler capture
  requires retaining `/proc/<host-pid>/task/<host-tid>/status` `NSpid` mappings
  while those exact process/thread generations are alive.
- `worker`: operation, epoch and source position.
- `requests`, `results`: independently read published/consumed queue counters.
- `control_pending` and `ready_epoch`: independently observed control and
  processing-readiness state.

The progress reads are **not an atomic snapshot**. A concurrently progressing
worker can change between reads. A pending read-only state capture does not prove
that control blocked audio. A worker operation does not establish SDK CPU time.
Use the clock bracket, exact request positions and external scheduler coverage
to test an attribution; refuse to infer a cause from this record alone.

Records survive ordinary stop/start and supported in-instance recovery, retaining
their original generation/epoch. Close-time export is best effort: an abruptly
terminated DAW process can lose these in-memory records. Neither an empty journal
nor a successful callback proves end-to-end speaker continuity.

`queued::tests::untraced_missing_span_retains_bounded_context_until_close` drives
the production ABI with a deliberately nonresponding test peer, no observer and
two epochs. It checks counted silence, a nonterminal result, no callback heap
effects or file writes, bounded retention and deferred export. The old source
fails because the close-time evidence is absent. This is an observation repair,
not a regression claiming that the physical audio deadline has been repaired.
