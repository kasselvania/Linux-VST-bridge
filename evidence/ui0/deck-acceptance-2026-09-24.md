# UI0 controlled Deck acceptance — 2026-09-24

## Provenance and claim

UI0 PR #166 was built and installed from source head
`1bdeb719a9a207db7d877fc6739b2fb937010f8b`, tree
`0ea958a46a82b5098d67caaaeea56aba62b506c3`. This is a
manager/frontend usability pass on the maintainer's Steam Deck, not a new
plug-in, audio-performance, or workspace qualification. The operator performed
the visual and input checks; read-only manager and filesystem readbacks
corroborated identity and lifecycle state. No screen automation was used.

The installed immutable software generation is
`29e52e7537c2e150a68db7f66799621a94d39b7aac7b20e22aded0d03a28facf`.
The stable launch paths resolve to its manager and frontend:

| Component | Installed SHA-256 |
| --- | --- |
| Manager | `fe896f366be932f001beb3c7901866a3a7925a46c056a2ec83b679961b58fba8` |
| Frontend | `6073179346586f11549de069c19776a7560ef8aad5cc7c1d18cd5723b394bf5d` |

The previously installed manager (`4f763cfbfda615364bf353d24dda0dbc52e5111a02448100b5dec17a101f5fb2`)
and frontend (`eecfb26853b19f310253441df4bd5e53c97b7416978d6c87d3c5b51b7b86625c`)
remain in their prior immutable generations as rollback targets. A staged
package containing those exact previous bytes was also retained privately on
the Deck. The installed host, source manifest, supervisor, ownership helper,
and preparation-kit digests match their preinstall values.

## Prestate and preservation

Bitwig was closed before installation. The manager service was active and
idle: DSP 0/6, maintenance 0, pending transactions 0, stale transports 0,
cleanup uncertainty false, and no active session. Six selected publications
were `Published` with valid artifacts: Pigments, Pure LoFi, Efx FRAGMENTS,
Serum 2, Blackhole Immersive, and Kontakt 8. A separate Serum 2 FX product
already needed attention.

The install replaced one manager/frontend generation. The six publication
status rows were exactly equal before and after installation. All seven
product identities, selected revisions, dispositions, environments, runners,
and module digests were unchanged. Serum 2 remained on selected revision 3
with runner `proton-11.0-2c-x11-touch-routing-v2`. The native catalogue kept
eight entries and the host catalogue kept four exact host/source pairs;
their only before/after field differences were immutable software paths.
The service restarted active with DSP 0/6 and no cleanup uncertainty.

## Operator-reported physical UI pass

The operator launched **Linux Audio Compatibility Manager** from the SteamOS
Applications menu, without a development launcher. Home opened against the
real snapshot and showed **Bridge ready**, DSP 0/6, and **Cleanup confirmed**.
The six navigation targets opened at ordinary size. Plug-ins showed the six
selected products, Workspaces was empty without an FL Studio claim, and
Activity showed no live session.

At narrow Deck width, the operator reported the deliberate three-by-two
navigation arrangement, reachable targets by touchscreen and trackpad,
visible selection, and keyboard Tab/Enter navigation. No target or content
was reported inaccessible. The first narrow Home attention item appeared
once; its **Review item** route opened **Serum 2 FX**. The request-status band
remained visible without scrolling. The operator did not submit a manager
action to create a test operation.

The operator then loaded one Serum 2 instance in Bitwig. Home showed
**Bridge in use** and DSP 1/6. The operator reported one running Serum 2 row
in Activity, **1 active** on the Serum 2 Plug-ins card, and no false claim on
Serum 2 FX. Read-only canonical snapshots showed exactly one active Serum 2
class session, `a418830e989f9c4a214cdf08d48f8312`, DSP 1/6, and no
pending transaction, stale transport, or cleanup uncertainty.

The operator played audio and closed only the manager. They reported that
Bitwig audio continued. A read-only snapshot while the frontend was closed
retained the same single session and DSP 1/6. After reopening the manager
from Applications, the operator again saw one running Serum 2 session and
the correct Plug-ins cards. Canonical readback confirmed the **same session
ID**, one DSP owner, and no new operation.

After the operator quit Bitwig normally, Home returned to **Bridge ready**,
DSP 0/6, **Cleanup confirmed**, and Activity showed no running session.
Final canonical readback showed service active, DSP 0/6, maintenance 0,
pending transactions 0, stale transports 0, cleanup uncertainty false, and
zero active sessions. Product identities and selected revisions were still
unchanged.

## Limits and disposition

This pass confirms the installed hierarchy, ordinary and narrow navigation,
the existing attention route, truthful real-session presentation, frontend
close/reopen behavior, and clean retirement. Input and audio continuity are
operator reports; the service and identity counts are canonical readbacks.
No physical action submission or polling-feedback lifecycle was exercised,
because there was no harmless operation needed for this pass. The one-click
and no-resubmission contracts remain covered by the source tests and hosted
validation on the accepted source head. No failure was fabricated and no
publication, candidate, installer, vendor, or workspace operation was run.

UI0 physical acceptance passed. Leave the installed generation in place.
