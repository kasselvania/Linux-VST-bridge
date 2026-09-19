# Manager library presentation — local result

2026-09-19; base `813a179355c9e22159e3adaaa6e93004e3ef2923`.
Implemented on `codex/manager-plugin-library`. Local macOS arm64,
Rust 1.95.0, eframe/egui 0.36.2. This result covers frontend presentation,
not Deck deployment or plug-in compatibility.

- Frontend build (binaries and examples): passed.
- Existing frontend checks plus three focused library tests: 35 passed.
- Frontend Clippy, all targets, warnings denied: passed.
- Manager `cargo check --locked --offline`: passed.
- Whitespace/diff check: passed.

The three added tests cover multi-term search/type/status filtering and unknown
status visibility; installation/preparation versus publication; and exact
installation association with preserved disabled reasons. Existing frontend
checks cover queued actions, changing snapshots, uncertain receipts and recovery.
No new DAW/audio campaign was run.

The actual widget rendered at 960 and 560 logical pixels. The narrow search
preview starts with `native 8.13` and shows only the matching record. Images were
visually inspected for wrapping, legibility and clipping. These are synthetic
records, including synthetic publication and problem states; they do not report
the user's installed inventory or new acceptance of Kontakt/Serum/FRAGMENTS.

- [Wide library](library-wide.png)
- [Narrow library](library-narrow.png)
- [Filtered library](library-search.png)

The first sandboxed preview launch could not connect to macOS window services
and produced no image. Its owned process was stopped. Rendering succeeded with
permitted local window access; each successful example saved its own frame and
exited. No pointer/keyboard input was sent to another application, no Jev calls
were made, and the Deck was not contacted. The production manager was not launched.

The installed Deck software, Bitwig session, projects, prefixes, libraries and
publication records are unchanged. End-to-end Deck interaction remains untested.
