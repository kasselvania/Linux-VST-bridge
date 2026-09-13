# Sanitized CA1 incident example

**Selected Windows fixture failed with access violation `0xc0000005`.**
Its self-exit trace carries the same status. The outer Proton launcher separately
returned 5; there is no native DAW host in this fixture.

- Thread: `windows-thread-1` (private PID/TID omitted).
- Exact fixture module SHA-256: `778f1c083e526219019b7a4195e5465afacf87965d5692c7f5e281279ee29378`.
- Fault: actual loaded module + `0x1006`, nearest exported `ap18_fault_site + 6`.
- Caller: module + `0x101f`, nearest exported `ap18_fault_caller + 15`.
- Further available frames are retained in the companion JSON; one frame lacks
  an exact module identity and remains unavailable. No guessed symbol.
- Startup output exceeded the former 64 KiB sink before the fault. The separate
  exception/module custody survived. Capacity and exclusion counters are retained.
- Owned process and transport cleanup both completed.

This fixture intentionally writes through null. It proves the reporter, not the
cause of any historical Pigments crash. The handled-exception companion exits
Windows normally and is not classified as a terminal crash. No raw paths, process
identifiers, vendor payloads or account data are present here.
