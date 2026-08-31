# SR0 fixture capture

This directory contains the bounded, read-only capture for **SR0 — Steam Deck Host and Serum 2 Pre-Installation Reconnaissance**.

Run it from the repository checkout on the Steam Deck:

```bash
tools/sr0-fixture-capture/capture.sh --self-test
tools/sr0-fixture-capture/capture.sh
```

`--self-test` runs seven deterministic cases beneath `$XDG_CACHE_HOME` (or the
user cache fallback): timeout propagation, global artifact-cap exhaustion,
directory-row truncation, search-root self-match exclusion, Flatpak runtime
branch matching, symlinked-ancestor rejection, and a normal bounded
non-finding. It does not generate or replace retained fixture evidence.

The capture:

- uses no root privileges and installs nothing;
- does not launch Bitwig, Wine/Proton/UMU workloads, yabridge hosts, Serum 2, Splice, Native Access, or any installer;
- reads only allow-listed host commands and declared bounded paths;
- records missing commands as `not_installed`;
- requires a root-owned, non-writable GNU `timeout` executable before collection;
- limits command duration/output, directory depth, rows, and candidate count,
  and retains completion, timeout, command-failure, and truncation status;
- never converts a timeout, failure, truncation, skipped contributing root, or
  rejected root into a zero count or `not_found_in_bounded_locations`;
- excludes every artifact search root itself with `-mindepth 1`;
- validates every existing search-root component, rejects symlinked ancestors,
  canonicalizes the real home and candidate, and requires canonical home containment;
- writes raw normalized observations only transiently in the ignored directory `evidence/raw/sr0-steam-deck-fixture-reconnaissance/` and removes them when capture exits;
- writes the sanitized review packet to `evidence/sr0-steam-deck-fixture-reconnaissance/`.

Declared artifact-search roots are:

- `~/.local/share/linux-vst-bridge-fixtures/serum2` (recommended private fixture input);
- `~/Downloads` to depth 2, matching only names containing `Serum` or `Xfer`;
- `~/.vst3`, `~/.vst`, and `~/.clap`;
- exact Windows VST/VST3 directories beneath already-discovered prefixes in the declared prefix roots;
- user-local yabridge paths only when a read-only `yabridgectl list` result can be validated as a local directory beneath the real home.

Runner roots are inspected at one directory level. Prefix roots are inspected at one directory level; prefix contents are not inventoried. Only known Windows VST directories beneath each discovered prefix receive a bounded Serum/Xfer name search.

Each directory collection requests the retained row limit plus one so row-cap
truncation is explicit. Artifact roots continue to be searched after the global
retention cap is reached; an unretained candidate or incomplete contributing
root makes the affected installer/module/content result `unknown` rather than
absence. Linux Audio extensions match Bitwig only when their branch equals the
exact Freedesktop runtime branch.

`sanitize.sh` is normally invoked by `capture.sh`. It converts normalized raw observations into the committed Markdown/JSON packet, replaces local identifiers, and regenerates `hashes.sha256`. It does not make a bounded non-finding into a global absence claim.

Volatile facts such as timestamp, filesystem capacity, and current process presence can change between runs. All other structured differences require review.
