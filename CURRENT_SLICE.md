# AP12 — Everyday Pure LoFi → Efx FRAGMENTS use in Bitwig

## Outcome and current position

The operator loads **Pure LoFi** as an Arturia instrument and **Efx FRAGMENTS** as an Arturia audio effect from Bitwig's browser, chains them, uses their actual editors and automation, saves, restarts, and reopens the same sound without a checkout, SSH session or manually started preview owner. Leave the working installation, publications, automatic service and project available for human use.

Continue [PR #79](https://github.com/kasselvania/Linux-VST-bridge/pull/79) and [issue #78](https://github.com/kasselvania/Linux-VST-bridge/issues/78) on `codex/ap12-arturia-everyday-use`. Reviewed diagnostic source: `5375621dab0ac11a72c7d6b2bfb3866e233b36f3`; basis main: `1d5d693e2e2d12547dfd6b02e2b8492b27418592`. [Review 5146648137](https://github.com/kasselvania/Linux-VST-bridge/pull/79#pullrequestreview-5146648137) grounds this repair continuation in production code and the SDK. This replaces the initial installation work order, not the AP12 product outcome. Ordinary implementation, necessary builds, debugging, vendor-UI presentation and focused verification are included under [AGENTS.md](AGENTS.md). Do not create a new slice, permission receipt or duplicate diagnostic/acceptance campaign.

Normal installations now initialize both products. Preserve the Rust registration/publication service, installed supervision, exact metadata, independent session work and all valid observations. Do not reinstall everything. The operator-replaced LoFi module is a distinct recorded artifact, not evidence about the original installer or authorization. The user project is preserved; use a disposable copy for edits/removal tests. See [AP12 findings](docs/AP12.md) and the original [failure records](evidence/AP12/).

## Repair the actual state contract

### Opaque state and parameter readback are different results

`ap8_state.h::commercial_state` captures real component/controller bytes, then discards them when an extra parameter getter returns an invalid normalized value. Current LoFi evidence identifies id 1066, value -1, flags 1024. Its proprietary reason is unknown. In the pinned SDK, hidden is bit 4, not 1024. Do not infer unknown flag/sentinel semantics or special-case a product name, parameter title or ID.

Successful bounded opaque capture is saveable independently of supplemental parameter-readback completeness. Introduce a versioned, validity-tagged readback representation: preserve each known parameter's identity and distinguish an available finite [0,1] value from unavailable/invalid readback. Invalid values belong in bounded diagnostics, not normalized host/DSP messages. Do not clamp them, delete parameters, fabricate vendor state, or reconstruct state from a parameter list. The proxy may retain a prior valid value or validated SDK default only as an explicitly marked presentation fallback, never as newly observed vendor state or a write-back instruction.

Update the complete path together: Windows capture, Rust `state.rs` envelope/parser, native controller `apply`, descriptor/initial mirror handling, Windows editor refresh and native refresh completeness. A single omitted pair currently fails the native count contract, and an invalid getter also fails editor refresh. Preserve old saved-envelope reads, class/module integrity, real ParamIDs, duplicate/extent checks, UI thread ownership and AP10 payload lifetime. Keep rejecting invalid actual automation commands. Private layout is the engineer's choice; incompatible formats need explicit versioning.

### A declined save is not automatically a dead processing session

Apply this behavior through `MappedSession::state_call`, Rust `Session::component_state`, `queued.rs::worker`, the C ABI and native `Processor::getState`:

| Observed result | Required behavior |
| --- | --- |
| Component and supported controller capture succeed; supplemental getter is invalid | Return the genuine opaque state with explicit readback unavailability. Do not terminate audio or manufacture a parameter value. |
| GetState returns an ordinary refusal such as kResultFalse/kNotImplemented, with intact protocol and bounded/quiescent streams | Return a correlated operation failure containing stage and SDK result; consume its sequence once; tell Bitwig saving failed, while otherwise healthy audio/editor service continues. Discard partial unsuccessful capture bytes. |
| Prior confirmed recovery snapshot exists when a later save fails | Retain it as the prior snapshot only. Do not call the failed save successful, advance its revision, or silently save the older sound. |
| Crash, unanswered/deadline-expired call, malformed identity/correlation/payload, unsafe stream lifetime, or uncertain/partially failed restore | Retain explicit terminal/unsafe handling. Do not resume an existing project with default or partially restored sound. |

Decode a legitimate error response before applying success-response checks; do not obscure a vendor refusal as `state response correlation`. Preserve existing serialization and accepted-edit ordering. Removing the fatal policy is not permission to call vendor state concurrently with processing or to block an audio callback.

## Make the real vendor UI reachable before saving is available

FRAGMENTS currently returns kResultFalse before any stream writes, reads, seeks or unsupported-interface queries. That is a genuine state refusal, not LoFi's successful-capture/readback failure. Bigger buffers or guessed stream interfaces do not address this observation.

Separate metadata/discovery, fresh-instance/editor readiness, and persistence availability. `inspect_module.cpp` currently requires getState before binding the editor; the native controller connection/`Processor::readback` also bootstraps through full state. Remove both universal prerequisites while preserving successful component/controller synchronization when state is available. A failed restore of an EXISTING project is not a fresh-instance request and must never be replaced with defaults.

Add the smallest managed **open vendor editor / vendor-access session** for an exact installed class and environment, including an unpublished class, by reusing the existing Windows host, controller and view lifecycle. No second synth pretending to control an active DSP, vendor code modification, screenshot editor or new GUI framework. Allow explicit save-unavailable audition where the vendor permits it, with visible status; do not advertise project recall.

Use that route to observe FRAGMENTS' own restriction and normal vendor access. Arturia documents [FRAGMENTS demo](https://www.arturia.com/demo/efx-fragments) with saving/loading/import/export disabled and a time limit. This is a hypothesis until the actual environment's vendor UI confirms it. Present any needed normal editor/installed companion handoff; account/activation actions belong to the operator. Do not log credentials, copy authorization files, replace vendor binaries, bypass demo limits or synthesize saved state. A confirmed restriction can still block the full AP12 recall claim, but must not block ordinary shared-code repair or strand its own authorization window.

## Keep reporting outside cleanup authority

The new `bridge-manager/runtime/session.py::run` writes its rich report unguarded after physical cleanup but before native-release/transport-retirement work. A report-write exception can skip that work and strand admissions. Repair this in the existing owner: cleanup and peer-release ordering must finish independently of diagnostic persistence; expose reporting failures separately and give the parent a reliable minimal ownership outcome. Retain leases/refusal when physical cleanup or ownership really is unproved. Do not fabricate clean readback or restart/kill healthy siblings. Add a focused injected report-write-failure test; no new supervisor framework.

## Implementation and verification

First repair the shared contracts with production SDK/transport/native-consumer tests, not another round of failing commercial trials. Cover successful opaque save with unavailable readback through native apply and genuine refresh; completed save refusal followed by correctly correlated audio/control; no fresh snapshot on refusal; safe restore/protocol failure; editor access before saving; and reporting-failure cleanup. Preserve the existing valid state, focus, gesture, stopped-save and shallow-payload regressions. Reuse unaffected evidence and binaries.

Then verify the actual installed products. Resolve the FRAGMENTS vendor-access question through its real UI. Recheck the LoFi two-control edit → state save → editor/device removal sequence and edited-sound recall. Once both products permit the necessary operations, complete the same Bitwig LoFi → FRAGMENTS project: correct browser roles, independent sessions/editors, one recorded and hands-off-replayed parameter per device, sound agreement, save/quit/reopen, and a Deck or user-session restart without development tooling. Removing FRAGMENTS must leave LoFi working. Do not label a demo-only or externally blocked pair project-safe.

Keep 512 added frames per proxy and report the actual chain's vendor latency and gap counts. Two bridge delays alone total 1,024 frames / 21.33 ms at 48 kHz. Earlier delivery timeouts remain separate from the later state exception. If a timeout reproduces, trace its first outstanding request and owning threads; do not declare these state repairs its cause without evidence or replay a broad latency matrix.

Retain metadata-derived names/vendor/subcategories and stable native class IDs, independent instance state, atomic/idempotent publication and installed startup without `.git` or fixture paths. Temporary tests and unrelated settings are restored; intended product artifacts stay installed. Stop repetitive trials when a vendor action is genuinely required, retain completed sub-results, and identify the exact user action rather than claiming success.

## Runtime boundary and sources

Keep the current pinned runner. `giang17/wine` at previously reviewed `0077f1c63098d65a4d2554cd31d07903773cf992` remains a potential runtime source for a matching rendering/window fault, not a demonstrated fix for these getters or saving restrictions. Do not mix DXVK/builtin graphics DLLs, change global Wine detection or assume every runner supports AP11's X11 window-ID capability. The independent bridge remains selected; no yabridge migration.

Use the existing pinned SDK, especially `ivsteditcontroller.h`, `ivstcomponent.h`, `gui/iplugview.h`, official `public.sdk/source/vst/hosting/plugprovider.cpp`, and [VST3 persistence](https://steinbergmedia.github.io/vst3_dev_portal/pages/FAQ/Persistence.html). Dossier basis: [architecture](docs/ARCHITECTURE.md) state/host boundaries and [activation/recovery](docs/design-dossier/03-activation-flatpak-runtime-and-recovery.md). Respect SDK thread affinity and normal vendor restrictions; a healthy process and a saveable project are distinct facts.

Finish this same PR with repairs, actual product results and remaining limits, left unmerged for review. Serum #77, full manager polish, universal routing, a new runtime, and a lower-latency default are not AP12 prerequisites. All historical failures remain unchanged.
