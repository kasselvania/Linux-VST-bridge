# WR0 findings

## Result

`WR0_COMPLETE` — The exact runner/runtime executed the tracked command workload twice through one replacement transaction, propagated exit 37, proved strict command identity, and safely forced the actual held topology to zero without touching an unrelated sentinel.

## Preservation and cleanup

Protected fixtures remained exact. Final WR0 descendant count is zero. The new environment was durably committed before predecessor retirement; the predecessor is absent and the verified replacement remains available for a separately authorized later slice.

## Claim ceiling

WR0 proves only controlled Windows command execution and isolated environment ownership on this exact fixture. It does not prove that Proton can host a Windows VST3; it does not load or inspect Serum, launch Bitwig, scan a plug-in, process audio, implement a bridge/proxy/IPC/shared memory/manager/broker, establish licensing, select a product runner, prove real-time safety, or generalize beyond this Deck.
