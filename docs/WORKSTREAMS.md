# Product workstreams

Selection date: 2026-09-26. This allocates work; it is not a compatibility
certificate or live machine report. [Support](SUPPORT_MATRIX.md) and
[failure classes](FAILURE_CLASSES.md) state the accepted physical claims.

## One management product, separate execution lanes

| Lane | Product role | Current next outcome | Boundary |
|---|---|---|---|
| x86 native Linux DAW bridge | Primary revenue and private-beta lane | [PB0](../CURRENT_SLICE.md): supported-system readiness, a guided setup plan and a local sanitized support export; then PB1–PB4 in [PRIVATE_BETA.md](PRIVATE_BETA.md) | A Linux DAW loads the native VST3 proxy; the supervised Windows host loads the plug-in. An exact Deck/Bitwig result does not qualify every distro, DAW or plug-in. |
| Managed Windows DAWs | Expansion lane | Continue exact FL workspace and directly hosted Windows VST3 work under separate WD ownership | No native proxy, bridge transport or inherited six-instance lease in FL's own audio path. Do not consume unmerged WD implementation in PB0. |
| ARM/OEM hardware demonstration | Separate appliance and manufacturer lane | Preserve accepted Pi findings and pursue separately selected Pi work | ARM first-note and hardware results do not define completion of the x86 private beta. Do not consume unmerged ARM work in PB0. |

The proposed $30 beta value is managed, exact compatibility on a supported
machine: lawful setup guidance, an exact tested runtime/profile, native DAW
publication, truthful readiness, diagnosis and safe rollback without routine
manual Wine administration. The beta contract and its limits are in
[PRIVATE_BETA.md](PRIVATE_BETA.md). This direction is not a claim that a beta
release has been selected or that every step is implemented today.

## Current allocation and custody

PB0 owns the public x86 manager/bridge product source and one read-only
Steam Deck Desktop Mode assessment of an existing ordinary Arturia publication.
The operator can keep using the six selected native products. Other source
work may proceed in isolated branches, but physical Deck mutations are
serialized; no concurrent installer, runner transition, service replacement,
publication change, or audio-setting experiment belongs to the PB0 gate.

The manager is the sole product authority. Exact profiles, immutable installed
catalogue, physical publication, transaction state and bounded platform facts
feed readiness. `SUPPORT_MATRIX.md` summarizes accepted results; it is never
parsed to decide runtime eligibility. Unknown observations are named unknown,
and a source build alone is not support.

The current public source baseline for PB0 is canonical `main` commit
`65f113b463ed1dbba7d142f1b833720575932a0d`, tree
`bf364f138849f8744ca18319380858545c0a1cf4`. The Steam Deck's installed
generation is separate physical authority and remains unchanged by the PB0
source work. Every later package transition must preserve Serum candidate D,
`x11_touch_routing_v2`, all six native publications, exact host/source pairs,
their runners, and rollback generations.

## Integration rule

One branch and one PR own the selected PB0 outcome. Normally merge reviewed
canonical main forward if it changes. Do not base PB0 on the private beta
frontend stack, unmerged ARM, WD1 or reassessment branches. Follow-on PB1–PB4
need their own selection and evidence; none expands PB0 automatically.
