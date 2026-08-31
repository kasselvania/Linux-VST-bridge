# HP0 native-probe tooling

These Bash tools build, validate, publish, and explicitly load the project-owned
`LabHostProbe.vst3` fixture. They refuse a running Bitwig process and do not
launch the Bitwig application, start a Windows workload, alter a prefix, or
change Flatpak overrides.

The build path is always the exact user installation of
`org.freedesktop.Sdk/x86_64/25.08`. Configuration requires an explicit clean
checkout of official `vst3sdk` commit
`3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`; all recursive submodule commits
are checked. CMake performs no dependency download and SDK-created plug-in
links are disabled.

Each retained build writes a v2 receipt whose operative source identity is the
SHA-256 of a deterministic `linux-vst-bridge-hp0-build-source/v1` manifest.
The manifest contains the stable repository-relative path, tracked mode, and
Git blob identity for top-level/CMake configuration, all tracked
`native-probe/**` files, the dependency lock, and the four scripts defining
build, dependency, and validator behavior. Missing, untracked, dirty,
non-regular, duplicate, and unexpected tracked source is refused. Historical
repository commit/tree remain provenance only. `build.sh
--print-source-manifest` emits the canonical manifest; `--verify-receipt`
recomputes it from the current clean checkout. An evidence-only amended head
can therefore verify the exercised receipt, while any covered source change
cannot.

Typical HP0 flow, from the repository root:

```sh
tools/hp0-native-probe/verify-dependency.sh \
  "$HOME/.cache/linux-vst-bridge/dependencies/vst3sdk/3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96"
tools/hp0-native-probe/build.sh --build-dir build/hp0-a
tools/hp0-native-probe/validate.sh --build-dir build/hp0-a
tools/hp0-native-probe/build.sh --verify-receipt --build-dir build/hp0-a
tools/hp0-native-probe/publish.sh publish \
  --bundle build/hp0-a/VST3/Release/LabHostProbe.vst3
tools/hp0-native-probe/publish.sh inspect
tools/hp0-native-probe/sandbox-probe.sh --build-dir build/hp0-a
```

`publish` accepts only this exact bundle and writes an owned copy at
`~/.vst3/linux-vst-bridge/LabHostProbe.vst3`. It rejects an unknown existing
destination, verifies a staged copy before replacement, retains a prior owned
copy until the replacement, source comparison, atomic receipt commit, receipt
readback, and final publication readback all succeed, and embeds an ownership
marker plus a source-file hash manifest. A failed post-swap operation removes
only that transaction's replacement and restores the exact complete prior
roster and hashes, or restores prior absence. The v2 receipt is staged beside
its cache destination, rejects symlinks and symlinked ancestors, and is replaced
atomically. `inspect` rechecks ownership, every retained source hash, and the
complete regular-file roster. Ordinary operation uses only
`$XDG_CACHE_HOME/linux-vst-bridge/hp0-publication.receipt` (or the equivalent
default user-cache path), and refuses any alternate receipt path. An existing
receipt must have the exact v2 key set, bundle/CID/transaction identity, valid
hash fields, and a complete-tree identity matching the owned publication; an
unrelated regular cache file is never adopted.

Alternate publication, receipt, build-fixture, and sandbox-log paths exist only
under `--test-mode --test-root PATH`. The test root must be an existing
canonical non-symlink descendant of the canonical user cache. Every dynamic
descendant is checked component-by-component, rejects empty, `.`/`..`, and
symlinked components, and must remain canonically beneath that exact test root.

Safe removal is explicit and limited to a verified owned publication:

```sh
tools/hp0-native-probe/publish.sh remove
```

Removal moves the verified bundle into the user cache for recovery. HP0 does
not run that operation after acceptance because the fixture is intentionally
left published for HP1.

`sandbox-probe.sh` first binds the bundle, module, validator, repository source,
SDK, class IDs, and parameter IDs to the exact `hp0-build.receipt` and verifies
every build-bundle hash. It then fails closed unless the system app is Bitwig
6.0.11 at app commit
`7b66aed37386ffff99e40bbd486f90ceee7ac1d5c59508c7952094122cebf50e`
with system runtime commit
`bd44a6230581917d04f89812a4c21090c304d390edb73995af1c2f9fd8abf4e8`,
no user-scope shadow app, and the exact retained user/system override hashes.
It enters that system application sandbox by replacing the application command
with `sh`, rechecks app/runtime/override identity afterward, and requires the
exact effective `VST_PATH`, empty `VST3_PATH`, and empty `CLAP_PATH`. It proves
the explicit published path can be read and validated by the receipt-bound
validator; it does not launch Bitwig, request a scan, or prove that Bitwig
automatically discovers the bundle.

`negative-tests.sh` creates fixtures only beneath the user cache. It exercises
evidence-only and covered-source manifest behavior, stale provenance edits,
unexpected tracked source, wrong and dirty SDK state, absent user SDK state,
ordinary/test receipt ownership, canonical containment and symlink escape,
unknown publication, staged/post-swap/receipt failures, wrong Bitwig
app/runtime identities, validator substitution, missing and modified bundles,
wrong ELF architecture, the Bitwig-running refusal, and exact override
preservation. Complete prior rosters and receipt bytes are compared, and
temporary fixtures are removed when the test exits.
