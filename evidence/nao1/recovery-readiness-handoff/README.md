# NAO1 recovery-readiness handoff repair

Executable source: `88247de9c79c565b17b88af277df4cbe819c53db`

Tree: `3746cc05f7a27f177d08b4c99b378b139cd295fc`

Package seal: `8685f43dd463aba356f758b3e5362e74f4f0a6c234c3c6f329f0d109ac5bf9ca`

The first real attempt remains a failed admission, not a blank-window or cleanup
result. Its sealed record admitted the exact `qualified_recovered_installation`
origin and reverified the current installer and daemon bytes. The first and only
SCM query returned exact service absence (`1060`); no start/readiness loop or
application launch followed. A single retained query cannot establish whether that
absence was transient or persistent.

The repair carries the complete closed dependency-session authority into the
operation-owned `Nad1Owner`. The owner independently revalidates the bound origin,
application/software identity, installer, daemon, environment, physical prefix,
fixed service, and fixed listeners. Exact qualified-recovery authority permits one
bounded one-second re-observation in the same owned runtime when the initial exact
query reports absence. A fresh exact registration continues through the existing
start/readiness path. A second exact absence or unavailable observation refuses
before application launch. The repair does not register a service, replay an
installer, execute the daemon directly, or manufacture an artifact/preparation
receipt.

The sealed source-owned campaign ran 15 generated sessions through the
artifact-absent, preparation-absent recovery topology. The physical-topology case
observed `absent -> exact`, established readiness, launched the fixture afterward,
retired with one Stop request, and completed. The persistent-absence case observed
`absent -> absent`, launched no application, issued no Stop, and retained
`dependency_qualified_registration_absent`. A fresh second session also completed.
All adverse cleanup and application outcomes retained their original terminal
truth.

All 703 evidence files present at the merged base remain unchanged. The generated
campaign preserved protected/runtime/prefix/private-home state, five projects, 307
retained witnesses, the active bridge, two keepers, zero leases, zero pending
transactions, zero stale transports, and capture off.

This repair was not installed. No second click, commercial Native Access launch,
real NTKDaemon transition, installer, DAW, plug-in, update, or production operation
occurred. The next physical step, only after independent review, merge, and exact
installation, is one new Native Access launch attempt.
