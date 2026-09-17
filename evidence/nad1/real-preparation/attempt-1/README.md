# NAD1 first authorized real dependency preparation

**Initial projection, corrected by [diagnosis.md](diagnosis.md).** The original owner reported a timeout, but later offline interpretation recovered exact installer exit 100 and numbered-control-set service registration. `receipt.json` preserves the original readback rather than rewriting it.

The operator waived installation-receipt review and explicitly authorized one bounded dependency-preparation test. The installed schema-7 manager admitted exactly one `DependencyPrepare` operation, `6345df5fd7ea38a5f07bfc6a2300e999`. Native Access was not launched. No retry followed.

## Result

Preparation failed with `dependency_command_timeout`: the exact bundled installer had not returned its completion acknowledgment within the owner's 200-second command bound. Its exit and completed installer-root receipt remained unavailable. The installed daemon file now exists with SHA-256 `e20b3d30b72d6a12e0a37b5fbd1a5c21e0db9e53459b4aab165270f7f739343b`. File presence does not establish a qualified installation or readiness. No prepared record exists.

The first exact SCM query reported service absence. After the installer timeout, one exact SCM stop returned registered STOPPED, zero endpoint mask, and confirmed generation retirement. A subsequent SCM query again returned STOPPED (service exit 1077). No explicit start action occurred. The final offline system.reg reader did not find the exact service key. Retain that distinction: live SCM registration during retirement does not establish durable offline registration.

The manager retained the original timeout, refused dependency qualification, and restored the bridge. Service retirement and process cleanup were confirmed, forced dependency cleanup was false, survivors were zero, and the exact unit/cgroup and resume obligation were absent.

## Preservation and custody

The exact merged NAD1 software generation remained unchanged. Service active; two keepers; zero DSP/maintenance leases, pending transactions, or stale transports; capture off. All 307 retained witnesses, five protected projects, 23 predecessor files and 81 existing protected records remained unchanged. Products, publications, import records, environment manifests and earlier application history were preserved. The real prefix did change through this authorized installer attempt; no whole-prefix preservation claim is made.

Private before/after records, installed-owner result, operator result and per-file operation evidence hashes remain retained. Public receipt contains only bounded result identities and state. No raw commands, environment, account data or vendor logs are published. This evidence-only amendment preserves every earlier observation and generated campaign.

Stop here. Native Access remains closed. Installer completion, durable service registration, admitted artifact recovery and readiness require investigation from retained evidence before another transition. No blind retry, direct daemon execution or timeout increase was performed.
