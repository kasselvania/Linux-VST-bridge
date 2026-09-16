# IS2 software metadata rollback repair

Review basis: `e7c2a9547fd025a45ab07e67bde447dffeb8e21f`, review 5225887708.

Explicit package selection now derives installer-launch capability solely from
that package. If `installer-launch.exe` is absent, the selected `Software` has
`installer_launch=None`, and serde omits the field. This keeps the manifest
readable by the pre-IS2 manager's `deny_unknown_fields` schema. An explicit
package containing the adapter selects that exact input for immutable staging.
Only an in-generation acceptance transition (`package=None`) retains and verifies
the prior adapter. No previous generation is removed or rewritten.

Three new tests exercise the production selection helper and stable setup commit:

- IS2 to explicit legacy package: no adapter field, successful strict legacy
  deserialization, exact current manager, byte-identical prior generation.
  The same strict legacy struct rejects the original IS2 manifest.
- Current package: its own exact adapter is selected and persists, independently
  of the prior adapter.
- Acceptance without a package: the exact prior adapter persists and still
  verifies; changed retained bytes are refused.

The legacy schema is copied from IS1 base `a0fde40e4c2a0536aa9b3271c3b945a94aa60d9b`.
Restoring the reviewed fallback in the helper made the legacy rollback test fail
at `retained.is_none()`; restoring the repair made all five setup tests pass.

Local verification: 125 manager library and 70 macOS manager binary tests passed;
strict all-target manager Clippy passed with Rust 1.95.0. Initial local commands
mixed Homebrew and rustup compiler tools and failed dependency/doctest loading.
Selecting a consistent rustup toolchain resolved that tooling mismatch; doctests
also completed successfully. Linux binary coverage and frontend/runtime checks
remain owned by the final-head AP12 workflow.

No Windows/runtime, product, installed-software or historical evidence bytes were
changed. No PowerShell, close-matrix, Native Access or other commercial run was
performed. AP8/AP10 inputs are unchanged; AP12 and PX2 cover the amended head.
This source-only repair does not establish historical Native Access causality or
implement a Windows scripting capability repair.
