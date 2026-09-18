# NAO1 qualified-recovery launch-admission repair

Executable source: `8ae5cea569491471ee6b1fdf6d234e3276dac09d`  
Tree: `eb686bcf0f6faa6f1bb87f902587dbb7394b3e85`  
Package seal: `732b72bdf2ffaefdf3197bf75f378f06a2a8cff1e1899b39f2056cbb1c82ed6f`

The application-session admission now accepts exactly two mutually exclusive
installation origins: a valid retained installation-artifact pointer, or the fixed
qualified-recovery record while that pointer remains absent. A present malformed or
mismatched pointer refuses without recovery fallback. Pointer appearance or
disappearance during admission refuses. Both Rust and Python validate the selected
origin, fixed historical source digests, current installer and daemon bytes, current
application/software generation, environment, and physical prefix identity.

The sealed source-owned campaign ran all 13 application cases through the
artifact-absent recovery topology. Every session recorded
`qualified_recovered_installation`; every case kept both `artifact.json` and
`prepared.json` absent. The campaign retained the original graceful, exact-owned,
failure, and cancellation outcomes, including successful second-session
restartability. No commercial application or real dependency operation ran.

The sanitized campaign proof is
`recovery-origin-proof-8ae5cea.json`. The complete package manifest is
`recovery-origin-source-seal-8ae5cea.json`; local and hosted results are in
`recovery-origin-validation-8ae5cea.json`.

All 698 evidence files present before this repair remain unchanged, including all 693
files from the NAO1 starting head and the original five-file qualification. The
campaign preserved protected/runtime/prefix/private-home state, five projects, 307
retained witnesses, an active bridge, two keepers, zero leases/transactions/stale
transports, and capture off.

This repair was not installed. Native Access, the real NTKDaemon, a DAW, plug-in,
installer, updater, and every production operation remained untouched. Reliable
graceful NTKDaemon shutdown remains unqualified.
