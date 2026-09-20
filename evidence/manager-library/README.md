# Manager library presentation — local result

2026-09-19; base `813a179355c9e22159e3adaaa6e93004e3ef2923`.
Implemented on `codex/manager-plugin-library`. Local macOS arm64,
Rust 1.95.0, eframe/egui 0.36.2. The initial result below covers frontend presentation; the later authorized
Deck installation is recorded at the end. Neither result qualifies plug-ins.

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
- [Expanded crash-capture controls](library-crash-capture.png)

The added crash-capture preview expands Details and management and System status
and diagnostics. It includes the existing arm action, a synthetic armed status,
and the existing disarm action. The diagnostics widget is shared with production;
its location, default collapsed state and capture behavior are unchanged. No
capture was actually armed. The local build, 35 existing frontend tests and
all-target Clippy passed after this preview addition.

The first sandboxed preview launch could not connect to macOS window services
and produced no image. Its owned process was stopped. Rendering succeeded with
permitted local window access; each successful example saved its own frame and
exited. No pointer/keyboard input was sent to another application, no Jev calls
were made, and the Deck was not contacted. The production manager was not launched.

The installed Deck software, Bitwig session, projects, prefixes, libraries and
publication records are unchanged. End-to-end Deck interaction remains untested.

## Authorized Deck installation

The operator subsequently approved deployment after the Blackhole Immersive
installer completed and the old UI was closed. Sol 5.6 at Extra High built the
Linux x86_64 release manager and frontend from `ef9f36a29d6af70538bf3a2db6be6016faf530a3`
using `tools/mf1/build-linux.sh`, Rust 1.95.0 and Zig 0.14.1. The first sandboxed
build could not write Zig's cache; the permitted identical build succeeded.

The installer operation was completed, with zero DSP/maintenance activity and no
pending transactions or unresolved cleanup. The existing setup transaction
installed the two verified binaries and restarted the idle bridge service.
Its unchanged verified host, host-source manifest, supervisor, ownership helper,
installer-launch adapter and preparation kit were retained. All eight native
proxy hashes and catalogue environment bindings were preserved. Hash comparisons
also preserved 34 registry/environment/installer records and two Kontakt project
files. Prior immutable software plus private setup/launcher records remain on
Deck for rollback.

[Sanitized readback](deployment.sanitized.json) records the installed hashes,
matching desktop launcher, resolved frontend dependencies, running service
identity and schema-7 inventory. Prior product statuses were preserved and both
vendor applications expose their exact environment associations. Crash capture
remains off. No GUI was opened or operated; this installation performed no new
Blackhole scan, licensing, audio or plug-in qualification.

## Desktop crash-capture guide

2026-09-19: the operator requested a PDF guide on the Deck desktop. The
[three-page PDF](../../output/pdf/Plug-in-Crash-Capture-Guide.pdf) and
[editable source](../../docs/user/plug-in-crash-capture.md) describe the installed
`ef9f36a` manager controls and CA1 behavior. ReportLab generated the document;
Poppler rendered all pages for visual review. Its text, page breaks, legibility,
button labels, export folder and final page numbering were checked.

SSH installed `Plug-in Crash Capture Guide.pdf` on the user's Desktop with no
existing file overwritten. The 71,197-byte local and remote PDFs match SHA-256
`5810fc2c691d9a98928ee74c29bc36f825cc798eb696c832104859c1e26ba42d`.
The installed manager hash still matches the deployment record. Its read-only
projection offers the arm action for Pigments, Pure LoFi and Efx FRAGMENTS;
Kontakt 8 and Serum 2 are disabled with `An exact ordinary publication is required`,
and Serum 2 FX offers no capture action. Capture was off with no active retention.
This verifies current action availability, not a new captured vendor failure.
No capture was armed, no plug-in was launched and no GUI was operated.

## Offline manager-clarity preview

2026-09-20; base `f164048f81045c76711e8ce1886a9dcd6f30dee7`, tree
`2855d972cfbc9c9939627a15b3fd1fae96296723`. The existing production-widget
preview was rendered again at 960 and 560 logical pixels with synthetic records.
It now keeps publication, current instance state, a previous failure, cleanup,
available actions and management actions visibly separate. The synthetic
Blackhole card also demonstrates that a prior Windows-host failure and confirmed
cleanup do not imply a current failed instance.

- [Manager clarity, wide](library-status-wide.png)
- [Manager clarity, handheld width](library-status-narrow.png)

Both images were produced by the existing `library_preview`; its buttons remain
inert and it has no manager client. They were visually inspected for wrapping,
clipping and status hierarchy. No installed inventory was read, no capture was
armed, and no vendor application or Deck session was used.

The editable crash-capture guide and its existing three-page PDF were updated
together. All three PDF pages were rendered with Poppler and visually inspected;
the resulting PDF is 10,300 bytes with SHA-256
`0686d293d674f2b9d13cf981167c227152aba73a7b79f374b1a1f7aa11ca5b5d`.

```sh
cargo run --manifest-path manager-ui/Cargo.toml --locked \
  --example library_preview -- \
  evidence/manager-library/library-status-wide.png 960 "" --diagnostics
cargo run --manifest-path manager-ui/Cargo.toml --locked \
  --example library_preview -- \
  evidence/manager-library/library-status-narrow.png 560 "" --diagnostics
```
