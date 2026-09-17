# Suspended-child access after dependency startup

The real application attempt reached exact daemon readiness, then Native Access's
shared application log reported renderer launch failure 72. Its operation outer
exit was 2 and its window never appeared. This is distinct from the earlier
missing-daemon failure and the accepted software-rendering A/B/C comparison.

The source-owned x64 probe creates its own child suspended, queries its PEB and
reads/writes eight bytes at offset 0x2d8. It always terminates and waits for that
exact child. These are the process-memory operations in Chromium's AppHelp-disable
startup path. It contains no Chromium or vendor implementation.

The before repair campaign proves:

- cold application-only route: query, read and write pass;
- after the production service-first route: query passes, read and write fail
  with Windows error 5 (access denied);
- exact service readiness and retirement pass; no forced cleanup is needed;
- scratch prefix and unit are removed, and real state is preserved.

This reproduces a process-memory regression introduced by the combined launch
route. It does not prove physical RAM exhaustion or that Native Access will work
once the regression is repaired.

The original staging package included macOS archive sidecars and was refused by
the sealed package validator before any session or Windows launch. The clean
archive carried the same sealed inputs; `before/` retains the completed result.

The repair keeps one operation-owned runtime container for SCM helpers and the
application. The host remains process/cgroup authority. A private local launcher
socket is never exposed through the operator API; the pinned launch client
forwards only owner-built commands and four diagnostic environment settings.
There is no runner replacement, sandbox disabling, registry change or vendor
binary mutation.

## Repaired comparison

`after/` retains the campaign executed at `e0ea7297bcfae67b74b1a19d67f302f374de00c7`.
Both the cold control and service-first application now read/write the exact
suspended child successfully. The application exits zero, readiness and SCM
retirement pass, the cgroup/unit/prefix are absent, and protected state is unchanged.

Development refusals are separate: the first container could not bind the private
socket directory; the next launcher exited on stdin EOF before its host client
could connect. An isolated Linux-only check initially reproduced that failure,
then confirmed the correction after stdin was removed as a lifetime authority.
The earlier commit message about the socket-location repair being qualified was
premature: that first Linux-only result was a refusal, not a pass. Both results
are retained. Final runtime lifetime belongs to the exact operation ledger.

The subsequent normal and repeated application runs both completed successfully.
Their campaign then stopped on a manager race: systemd collected the completed
unit between query and Stop. Fresh checks proved exact absence and clean results.
The manager amendment accepts that race only under fresh unit/cgroup absence and
exclusive writer custody; uncertainty still refuses and the signal is not retried.
