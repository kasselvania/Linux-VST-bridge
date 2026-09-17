# IS4 installation-only receipt

PR #115 merged at `8fb2f20c822320340e44f9cbae399633dd6e8a53`, with reviewed
head `32a9e290726235c8c0a2e9996f3e9c11ca3edb08` as its second parent. The merge
produces reviewed tree `947461f96dc2648b13c9b450c761beae97880571` exactly.
No executable source changed in this installation continuation.

Merged-main checks passed against that merge commit:

- [AP8](https://github.com/kasselvania/Linux-VST-bridge/actions/runs/35165384039)
- [AP12](https://github.com/kasselvania/Linux-VST-bridge/actions/runs/35165386170)
- [PX2](https://github.com/kasselvania/Linux-VST-bridge/actions/runs/35165337502)

## Installation and identity

The manager and frontend were built from the exact merged checkout with locked
inputs, Rust 1.95.0 and the existing Zig Linux link recipe. The adapter was selected
from merged-main AP8; its SOURCE_COMMIT and SHA256 inventory were verified. Its
bytes match the previously installed adapter. The supervisor and ownership files
are exact merged-source bytes. Existing host/source-manifest, preparation kit and
product artifacts were retained through the reviewed immutable setup owner.

The operator applications were already closed. Canonical idle preflight passed
before staging and again immediately before one authorized service stop, setup and
restart. No keeper/environment was independently stopped, closed or modified.
Service-owned keeper retirement/restart used the existing route.

Predecessor generation:
`333fbcce988b13001d8922509db789a966a98729decaeb3ecfa4c1672df38071`

Selected generation:
`83cb77e21978b4f9688400a2b845babb436dbd1851309fea21e793c8f59f4e30`

| Installed artifact | SHA-256 |
| --- | --- |
| Manager | `36956341aa45a60fe869d09bc62443d59b1cfd864ba625c229b1de592187a066` |
| Frontend | `62d11f3ba9e41446e6d0a5b81c0a62351c05b899d71073b2e0e866f72ef7c089` |
| Supervisor/session | `d1263e48693b72a407914a398f3a5afba2fe26ae8b7f1e210b1b536b8a90e0ad` |
| Ownership | `74754557d1a732458910833e255b34e4d4cade7ccf97df67e94ea53744b01377` |
| Installer adapter | `0edb8946927a93cac10eb60a0b0567ba284a65d6c3f30255d2612d08f8ae1cd7` |

The installed software-record digest is
`c583046a82de9880f9740b8ce6df90184416eff1343e2434ada8b1d63d477fa7`.
The prior software-record digest is
`0707722fae194053276416649751daa1c6111022b0751c0cd6226b334de37ed8`.
All 23 predecessor generation files remain byte-identical and available for the
existing idle rollback route. The new native catalogue contains relocated immutable
software references; registry/product/publication bytes did not change.

The private installation receipt was fsynced, installed without replacement and
made read-only. Its SHA-256 is
`045c883f46c2f53288f900690faf65132a34b4378f1071c746d3a18fdd588842`.
Public evidence contains bounded hashes and states, not private software locations
or raw process/setup logs.

## Canonical readback

Installed operator model is **5**. A non-installed, read-only harness compiled the
unchanged merged `manager-ui/src/client.rs` and shared operator model, called the
production Snapshot and Activity paths, and decoded both schema-5 responses. It
submitted no action and opened no GUI. Exact source/helper digests are retained in
`build-identity.json`. The final helper has direct-child exit 0 and positive wait
custody. This is API/model agreement, not a new visual GUI acceptance campaign.

A supplementary report initially attempted an overly broad process census and hit
permission errors on unrelated process metadata. No installation was repeated.
The final report uses exact direct-child wait custody for the readback helper;
no broader process-access permission was requested or added.

Final state:

- Service active; two healthy keepers.
- DSP leases 0; maintenance leases 0; pending transactions 0; stale transports 0.
- Cleanup unblocked; capture off; no service-resume record.
- 307 retained-file witnesses and five protected projects unchanged.
- Pigments 18 and retained history, Pure LoFi 10 and Efx FRAGMENTS 10 unchanged.
- Serum 2 remains experimental at revision 1; Serum 2 FX remains separately
  installed and unqualified. Their module and publication identities are unchanged.
- Imports, environments, onboarding histories and registry unchanged.

No linked installer successor, policy selection, Native Access launch, interpreter
installation, runner replacement or product/publication change occurred. No
physical campaign was replayed. Installation is complete; this receipt is returned
for independent review. A commercial continuation remains separately gated.
