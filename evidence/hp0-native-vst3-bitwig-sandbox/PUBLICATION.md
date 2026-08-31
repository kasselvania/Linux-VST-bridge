# Controlled HP0 publication

Classification: `passed`

```text
destination: <HOME>/.vst3/linux-vst-bridge/LabHostProbe.vst3
publication type: exact owned copy, not a symlink
source module SHA-256: 3abaa8a2051ea179e28986cc6fbad4cde2d0d5e9a844b18aca91cdf7c59e69d7
published module SHA-256: 3abaa8a2051ea179e28986cc6fbad4cde2d0d5e9a844b18aca91cdf7c59e69d7
source manifest SHA-256: ec8a4531a3b73da3c3e29c6e1f3c90c989fb61e3f3397af32d4c1576b3624ea9
published manifest SHA-256: ec8a4531a3b73da3c3e29c6e1f3c90c989fb61e3f3397af32d4c1576b3624ea9
ownership schema: linux-vst-bridge-hp0-publication-owner/v1
receipt schema: linux-vst-bridge-hp0-publication/v2
ordinary receipt: <HOME>/.cache/linux-vst-bridge/hp0-publication.receipt
receipt SHA-256: d328adb27fef326b8e5b1104f072c246a803389a77d35ee5babc9834067668d1
published complete-tree manifest SHA-256: d8258abbedf789f97b6e32292131ad3e58b3c0b028905244a0a9487905b28d1d
```

The publisher required the exact bundle name, module path, generated metadata,
vendor, processor CID, controller CID, regular-file shape, and absence of
symlinks. It staged to a temporary sibling, wrote an ownership marker and
source-file manifest, verified every source hash and the complete regular-file
roster, then replaced only a previously verified HP0-owned destination. The
prior owned publication and prior receipt remained recoverable until staged and
destination inspection, complete source comparison, symlink-safe receipt
staging, atomic receipt commit/readback, and final publication readback had all
succeeded. No stage or backup sibling remained after commit.

Ordinary operation accepts only the one project receipt path above. Before
replacement, an existing receipt must be a regular non-symlink file with the
exact accepted schema, bundle name, fixed processor/controller CIDs, transaction
identifier, complete one-per-key critical field set, and hashes matching the
currently verified owned publication. An unrelated regular file at the exact
ordinary receipt path was refused before destination mutation; its bytes and
mode, size, and timestamps remained unchanged. An alternate absent ordinary
receipt path was also refused.

Alternate publication, receipt, and build-fixture paths exist only in explicit
test mode with one declared test root. That root must be an absolute canonical
non-symlink descendant of the canonical user cache; all accepted descendants
must remain canonically inside it. Dot/dot-dot components, symlinked ancestors,
escapes, non-directory ancestors, and a receipt or build root outside the exact
test root are refused before publication or validator execution.

Deterministic failures after destination swap, during receipt staging, and
after atomic receipt commit proved that the replacement is removed and the
complete prior roster/hashes plus exact prior receipt bytes are restored. The
same post-swap and post-receipt paths with no prior publication restored exact
absence and left no successful-looking receipt. A receipt symlink was refused
before publication state changed.

The publisher refuses an unknown destination. Its documented `remove` action
first repeats the complete ownership/hash check and then moves only this exact
owned bundle to a recoverable cache location. Removal was not run: the verified
publication remains for HP1.

No SDK-created link was used. The publisher did not enter or touch
`<HOME>/.vst3/yabridge`, `<HOME>/.wine`, the existing Serum proxy, or the
existing Serum Windows module.
