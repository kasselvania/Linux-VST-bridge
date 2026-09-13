# CA1 reporter fixture

`fixture.py --runtime <session.py directory> --fixture <source-owned EXE>
--sha256 <exact digest> --output <new private directory>
--mode handled|normal|exit|fatal|fatal-late`

Run on the exact admitted Deck runner with zero DSP/maintenance owners. The
existing keeper stays owned and the shared environment lock excludes installers.
This is a development fixture, not a product launch interface. It substitutes
only `command()` and the exact source fixture registration; collector, finalizer,
pipes, Popen, process tracker and cleanup are the actual production path.

Build `tools/ap18-tests/crash_capture.cpp` with the existing Windows CMake target
`ap18-crash-capture`, or pinned-development compiler with exports and optimization
disabled. Fatal-late emits known non-secret padding beyond old first-N capacity,
then takes the actual unhandled exception. No vendor module is loaded. Raw output
stays private. `proof.json` is the explicitly sanitized result.
