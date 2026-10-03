# Research Basis

**Prepared:** 2026-08-29 America/Los_Angeles  
**Purpose:** Preserve the exact internal method and external technical/vendor facts used to seed the initial dossier. This file is evidence and provenance, not a compatibility claim.

## 1. Internal repository bases

The internal design method was read directly at these bases:

| Repository | Commit | Tree | Role in this design |
|---|---|---|---|
| `kasselvania/generalized_execution_platform` | `8f64d371cd88505d38656e5323506d0b9af1c115` | `9cc4a3caaa15324db607cecc45ec1d857bff03a4` | Current clean implementation repository posture and small-slice discipline. No implementation from this repository is imported. |
| `kasselvania/trade_replicant_library` | `10afa08ee95b6640a0c9e3f8195e5b38a2482268` | `79f9671670eb054239c2697e63c6ef0c8bf66c39` | Current library/recovery posture. No audio-bridge authority is imported. |
| `kasselvania/science_platform_and_replicant_dossier` | `d85532dbbe36476284ad0cd92b1ed93d153389b3` | `db1a1dcb0bbf55806e5fa79e8599b42c6ec05b68` | Exact `UP_TO_SPEED.md`, governance, dossier-consumption method, and human-north-star versus active-slice separation. It is not a runtime or build dependency. |
| `kasselvania/standalone-BitWig-push` | `2a694a728c0b6bdff3bd64ceb5850dbc2ad27ffe` | `2b2e606e963ec94eef4efcd434541c3f49f0b8a2` | The neighboring Bitwig/Steam Deck fixture, evidence posture, component-boundary style, and rule that Steam Deck is a reference fixture rather than universal product definition. |

### Exact internal documents read

- `science_platform_and_replicant_dossier/UP_TO_SPEED.md`
- `science_platform_and_replicant_dossier/GOVERNANCE.md`
- `science_platform_and_replicant_dossier/AGENTS.md`
- `science_platform_and_replicant_dossier/docs/dossiers/GEP_At_a_Glance.md`
- relevant beginning and constitutional sections of `Scientific_Definition_Activation_and_Visual_Workbench_Governing_Design_v2_2026-08-23_Reconciled.md`
- `standalone-BitWig-push/AGENTS.md`
- `standalone-BitWig-push/CURRENT_SLICE.md`
- `standalone-BitWig-push/docs/ARCHITECTURE.md`

### Internal method retained

```text
large human design corpus
    -> exact current-source inspection
    -> one bounded decision
    -> one active repository-owned slice
    -> focused implementation and evidence
    -> exact review
    -> post-merge status without auto-selecting a successor
```

This repository adopts that method. It does not copy the GEP product architecture.

## 2. External source bases

## 2.1 VST3 format and SDK

Primary sources:

- [Steinberg VST 3 SDK repository](https://github.com/steinbergmedia/vst3sdk), inspected at current 3.8.1-era head `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`.
- [VST3 plug-in format documentation](https://steinbergmedia.github.io/vst3_dev_portal/pages/Technical+Documentation/Locations+Format/Plugin+Format.html).
- [VST3 SDK license and usage material](https://github.com/steinbergmedia/vst3sdk#license--usage-guidelines).

Source facts retained:

- Windows and Linux VST3 bundles contain platform-specific binaries; a Windows DLL is not a Linux in-process module.
- VST3 uses a factory that can expose multiple classes.
- VST3 separates processing and editing/control concepts and exposes many dynamically queried interfaces.
- The current official SDK is C++ and includes examples, test host, and validator support.
- The SDK moved to an MIT license in the 3.8 generation; exact notices and trademark usage still require review for distribution and naming.

Design consequence:

- Use a native Linux proxy and out-of-process Windows host.
- Preserve module and class identity.
- Keep the first official VST3 boundary in narrow C++20 shells while Rust owns the product.

Unknown:

- Exact Bitwig-required interface subset for Serum 2.
- Exact permitted consumer-facing name under current trademark guidance. The repository name is not accepted as a product brand.

## 2.2 Existing Rust VST3 bindings

Primary source:

- [RustAudio `vst3-sys`](https://github.com/RustAudio/vst3-sys), current repository head observed as `f3e8f01c3de6d5df2f503c920c9f2bf8166a771b`; README blob `fb5da8b46aa3d45e5a1ca341b53c6549ecee967d`.

Source facts retained:

- It is a pure-Rust port of the COM-compatible VST3 API, not a high-level complete SDK wrapper.
- Its README explicitly reports missing constants/coverage and intentional omission of non-COM SDK material.
- The inspected repository remains GPLv3 and has not advanced since 2023, despite the newer MIT SDK era.

Design consequence:

- Do not make the first commercial product boundary depend on this crate's completeness or licensing.
- Re-evaluate newer permissive Rust bindings before implementation, but keep the official C++ shell as the provisional low-risk choice.

Unknown:

- Whether a newer complete permissive Rust host binding will mature before the VST3 boundary slice begins.

## 2.3 CLAP

Primary source:

- [free-audio/clap](https://github.com/free-audio/clap), current observed head `a47f6badb49d948fd009998f28309cdab78979c9`.

Source facts retained:

- CLAP defines a stable C ABI with 1.x backward-compatibility intent and explicit thread requirements.
- Officially listed Rust bindings exist.

Design consequence:

- Share manager/runner/environment/transport architecture across formats.
- A future CLAP proxy/host may be predominantly Rust.
- Do not claim CLAP implementation from format-neutral core design.

## 2.4 Yabridge prior art

Primary sources:

- [robbert-vdh/yabridge](https://github.com/robbert-vdh/yabridge), inspected near head `b580a9f7fc46509767ca156d4f92872552b9e571`.
- [`docs/architecture.md`](https://github.com/robbert-vdh/yabridge/blob/master/docs/architecture.md), blob `257323e63986dcb2af8bfd9f68212b4eb3d51742`.

Source facts retained:

- Yabridge uses native Linux plug-in libraries/chainloaders and Wine-hosted Windows processes.
- Control calls are serialized over Unix-domain sockets.
- Additional sockets/threads are needed for mutually recursive calls.
- GUI/message-loop operations require thread-affinity handling.
- VST3 modules can expose multiple classes and require extensive object/interface proxying.
- Audio buffers use shared memory while sockets carry synchronization and other data.
- Editor embedding requires substantial X11/Wine window management, focus, resizing, and drag-and-drop work.

Match:

- This dossier independently reaches the same unavoidable process boundary and several similar technical pressures.

Gap/opportunity:

- The intended product adds managed runners, installation, authorization, Flatpak publication, profiles, content/preset organization, diagnostics, rollback, and support-state UX.

Clean-room rule:

- No yabridge source is copied. Public architecture and observed constraints are prior-art research. Original implementation is based on public format specifications and repository-owned interfaces.

Licensing fact:

- Yabridge is GPLv3. Any reuse/fork strategy would require an explicit product and license decision; none is made here.

### 2026-10-03 audio-path comparison

The operator requested a renewed comparison while investigating the config4
failure. This is source research, not a matched performance run. LVB source
`19c5888bd3332c827c317387365b177587d9d805` is the frozen comparison; its native
and Windows audio trees remain unchanged at `bb3cfad3`.

| Boundary | LVB config4 | Yabridge at `b580a9f7` |
| --- | --- | --- |
| Delivery | Positioned queue with epochs and stale-result rejection; added delay requires `D >= M`. | Returns the current request's output in that callback through shared buffers and a dedicated processor channel. [Processor](https://github.com/robbert-vdh/yabridge/blob/b580a9f7fc46509767ca156d4f92872552b9e571/src/plugin/bridges/vst3-impls/plugin-proxy.cpp#L213-L260) |
| Wakeup | Native request, Windows mailbox and native reply paths poll with approximately 50-microsecond requested sleeps; only completion toward the DAW callback has a futex notification. | Socket arrival wakes a blocked receiver. The examined reply read has no per-block deadline. That is not LVB's required bounded real-time failure contract. [Communication](https://github.com/robbert-vdh/yabridge/blob/b580a9f7fc46509767ca156d4f92872552b9e571/src/common/communication/common.h#L1029-L1036) |
| Scheduling | Best-effort exact-owned RTKit request and effective readback; the failed fixture remained ordinary, with Windows selection failing before RTKit. | Attempts FIFO priority 5 and forwards the DAW thread's real-time priority periodically. Different policy is not evidence that adopting it repairs LVB. [Host scheduling](https://github.com/robbert-vdh/yabridge/blob/b580a9f7fc46509767ca156d4f92872552b9e571/src/wine-host/bridges/vst3.cpp#L1722-L1848) |
| Offline work | Uses the same local `N/Fs` completion allowance; timeout can substitute silence. Whole-callback and serial-chain budgets remain unqualified. | Awaits completion and dispatches offline work to the main context, introducing its own owner coupling. [Offline dispatch](https://github.com/robbert-vdh/yabridge/blob/b580a9f7fc46509767ca156d4f92872552b9e571/src/wine-host/bridges/vst3.cpp#L1858-L1888) |

This supports the already accepted handoff/completion repair direction in the
[platform assessment](PLATFORM_ARCHITECTURE_REVIEW.md#prior-art-comparison).
It does not identify the cause of the retained 1024-frame gap. No implementation
code was copied. Compare measured delivery stages and actual render-thread CPU/runqueue
time before choosing a timing repair; separately test unpaced bursts and slow valid
offline processing through the final supplied block.

Proton 11's [runtime options](https://github.com/ValveSoftware/Proton/blob/5b89db940e0ebe3a137a6009a3589232fe084c09/README.md#runtime-config-options)
document fsync and ntsync. These affect Windows
synchronization; they do not automatically notify LVB's mailbox. The inspected
[Valve Wine delay implementation](https://github.com/ValveSoftware/wine/blob/dc26e61847081a1b5cb0733dc30feba6ee575482/dlls/ntdll/unix/sync.c#L2451-L2502)
handles non-alertable `NtDelayExecution` through timed waiting/yielding. This review
does not establish the effective synchronization backend of the installed GE-Proton11-7.

Capacity is a separate measured dimension. CPU quotas cap bandwidth rather than
reserve scheduling time; guest policy cannot guarantee prompt VM execution.
Reclaim may stall work without an OOM. Record interval quota/pressure/fault deltas
and both guest and outer memory limits for the authorized memory comparison.
[Kernel resource controls](https://docs.kernel.org/admin-guide/cgroup-v2.html),
[pressure measurements](https://docs.kernel.org/accounting/psi.html).
A clean larger-memory repetition cannot replace the failed lifetime or establish
its cause.

The [installed comparison](../evidence/preparation/2026-10-03-controlled-capacity-observation.json)
at source `c9117d6a` retained the same engine/Windows host and completed one short
reference lifetime at each memory limit. Both were clean. The larger guest avoided
the smaller run's measured swapping; fixed order and cache carryover remain
confounds. This is neither a yabridge benchmark nor dependable-audio qualification.

The exact Windows render target was unavailable at both sizes. Source review at
`c9117d6a` traces custody to the authenticated pre-exec bootstrap in
`bridge-manager/runtime/session.py` and descendant tracking in `ownership.py`.
The render-start event has no authenticated final Linux-host generation; refreshing
the tracker already occurs before scheduling. Unseen reparented or shared-service
children cannot be acquired solely by that traversal. The observed cohort had no
exact session-status mapping; missing final-host custody versus mapping projection
is unassigned. The repair must bind the final host through existing launch custody,
including namespaces and status generation, without adopting guessed processes or
the keeper's entire cohort. Zero targets refused before RealtimeKit, so this result
is not a permissions denial.

Control timing also measured deep repeated verification during ordinary readback,
request admission and the worker. `operator_cli/current.rs`, `operator_model.rs`,
`preparation_cli.rs` and registration/runner validation share those paths. Bounded
display/offer records and fresh execution/mutation admission have different jobs:
use the existing control records for the former and retain the latter's exact
verification at its owner. No new persisted cache authority or metadata-based claim
of freshly verified executable bytes is justified. These measured costs and source
ownership gaps are independent findings, not an attribution of the retained gap.

## 2.5 Proton

### Final-host custody investigation, 2026-10-03

The proposed shared-runtime adapter reuses the existing per-launch stdout channel.
[Linux v6.16 socket UAPI](https://github.com/torvalds/linux/blob/v6.16/include/uapi/asm-generic/socket.h)
and [Unix socket implementation](https://github.com/torvalds/linux/blob/v6.16/net/unix/af_unix.c)
provide per-message credentials and a kernel-delivered process handle through
`SO_PASSCRED` and `SO_PASSPIDFD`. A received numeric PID alone is insufficient:
queued output may outlive its sender. Pin the generation, then validate the exact
host readiness identity and mapped session object. Diagnostic output grants no
custody. This is an original adapter design, with no third-party implementation
copied into the repository.

At Wine source `dc26e61847081a1b5cb0733dc30feba6ee575482`, the
[Unix file implementation](https://github.com/ValveSoftware/wine/blob/dc26e61847081a1b5cb0733dc30feba6ee575482/dlls/ntdll/unix/file.c)
contains caller-side writes after descriptor resolution. Conversely, the
[socket server](https://github.com/ValveSoftware/wine/blob/dc26e61847081a1b5cb0733dc30feba6ee575482/server/sock.c)
performs connection operations; a newly connected socket's peer is not sufficient
proof of the final plug-in host. These source facts motivate testing the inherited
channel. They do not prove the pinned installed Proton stdout route, kernel
availability on all targets, custody, cleanup, scheduling or audio continuity.

Primary source:

- [ValveSoftware/Proton](https://github.com/ValveSoftware/Proton), current observed development head `b7a763327f0cb76e9e0bd53dc4de95dee0d4c3b7` and inspected `proton_10.0` README blob `37096af2ced6f71003f54b646e15246deae8ec51`.

Source facts retained:

- Proton is a Steam tool using Wine to run Windows games.
- Source is provided for modification.
- Proton builds in a supplied SDK/container workflow and can produce redistributable builds.

Design consequence:

- Treat Proton as a runtime-shape and patch/component source.
- The production runner should be audio-governed, immutable, side-by-side versioned, and independent from the Steam client.
- Existing Proton/UMU may be used for experiments without becoming product authority.

Unknown:

- Exact synchronization, graphics, container, and Wine patch set best for audio plug-ins.
- Whether the first runner should begin from Proton, Wine Staging, a specialized build, or another maintained base.

## 2.6 Bitwig Flatpak

Primary source:

- [Flathub Bitwig manifest](https://github.com/flathub/com.bitwig.BitwigStudio/blob/master/com.bitwig.BitwigStudio.yaml), current observed repository commit `ec70c742b9196b63b038f92685f028ecec494310`, manifest blob `355a30626acc5d14ec6a69a785cb4b32718b25ca`.
- [Linux Audio base extension](https://github.com/flathub/org.freedesktop.LinuxAudio.BaseExtension), README blob `7a8d9bbbdfc5451cbd0f52468b22a74d5ae9ee7e`.

Current manifest facts retained:

- The inspected Bitwig package is 6.0.11 on Freedesktop runtime 25.08.
- It declares device access, IPC, PipeWire/PulseAudio, X11, network, host filesystem access, and multiarch.
- It declares `VST_PATH`, `VST3_PATH`, and `CLAP_PATH` under `/app/extensions/Plugins`.
- It declares the `org.freedesktop.LinuxAudio.Plugins` extension point for VST/VST3/CLAP.
- The Linux Audio extension documentation notes runtime-branch matching and that local `~/.vst3` visibility varies by application.

Design consequence:

- First prove a native proxy in the exact sandbox.
- Keep user-path publication and a matching Linux Audio extension as separate candidate modes.
- Treat current broad permissions as version-specific evidence, not permanent architecture.

Unknown:

- Exact broker/process/shared-memory/editor path on the maintainer's current fixture.
- Whether Bitwig's internal plug-in host adds additional namespaces or restrictions beyond the app manifest.

## 2.7 Serum 2

Primary/vendor sources:

- [Xfer Records Serum 2 product page](https://xferrecords.com/products/serum-2).
- [Splice: lifetime Serum 2 activation](https://support.splice.com/en/articles/10547355-how-to-activate-your-lifetime-copy-of-serum-2).
- [Splice: Serum 2 license troubleshooting](https://support.splice.com/en/articles/10055412-solving-serum-2-license-and-authorization-issues).
- [Splice: Rent-to-Own authorization behavior](https://support.splice.com/en/articles/8652687-about-rent-to-own).

Current documented facts retained:

- Serum 2 is delivered as 64-bit VST3/AU/AAX and requires Windows 10+ for the Windows build.
- Xfer advertises more than 626 presets and 288 wavetables, creating meaningful content/preset pressure.
- The demo is time-limited, so it is not a durable long-session compatibility fixture.
- An owned/lifetime copy can authorize through an Xfer browser login; documented offline machine-ID/license-file authorization exists.
- Active Splice Rent-to-Own uses the Splice Desktop app for recurring verification and documents a limited offline interval.
- Owned and active-rental channels therefore have materially different runtime requirements.

Design consequence:

- Serum 2 is the first real commercial VST3 fixture.
- Every result names exact build and license channel.
- Prefer the maintainer's lawful Xfer-owned/lifetime route for first bridge isolation when available; otherwise treat Splice companion behavior as part of the fixture.
- Discover actual Serum 2 content/config paths during fixture capture rather than importing historical Serum assumptions.

Unknown:

- The maintainer's exact Serum 2 build/license channel.
- Exact current Windows install/content/config paths.
- Exact editor graphics backend and callback behavior under the chosen runner.

## 2.8 Kontakt and Native Access

Primary/vendor sources:

- [Native Access FAQ](https://support.native-instruments.com/support/solutions/articles/69000879276-native-access-frequently-asked-questions).
- [Installing NI products on a new computer](https://support.native-instruments.com/support/solutions/articles/69000879371-installing-native-instruments-products-on-a-new-computer).
- [Kontakt “Not activated” / “Full Kontakt required”](https://support.native-instruments.com/support/solutions/articles/69000879750-kontakt-error-message-not-activated-or-full-kontakt-required-).

Current documented facts retained:

- Native Access is NI's administration tool for activation, installation, and updates.
- Internet is required for normal product installation/update and serial registration flows.
- Native Access exposes configurable download, application, and content paths.
- Existing Kontakt libraries can be located/relinked rather than necessarily redownloaded.
- Kontakt Player versus full Kontakt and licensed versus unlicensed third-party libraries affect activation and browser/load behavior.
- Legacy product support is not uniform.

Design consequence:

- Kontakt is a later hostile-system test for companion managers, large content, relocation, authorization, edition/license semantics, and legacy behavior.
- Kontakt does not block the first VST3 protocol proof.

Unknown:

- Exact Kontakt and Native Access versions available to the maintainer.
- Which lawful representative libraries should form the first narrow matrix.

## 3. Evidence table used for the initial bounded decision

| Question | Source | Current implementation | Match | Gap | Drift | Unknown |
|---|---|---|---|---|---|---|
| Can native Bitwig directly load a Windows VST3 DLL? | VST3 platform bundle docs | No bridge implementation exists | None | Native proxy + Windows host + IPC required | Any “Proton directly loads it in Bitwig” wording would be false | Exact Bitwig interface use |
| Is Rust appropriate? | Repository experience; current Rust/C++ ecosystems | Empty repo | Rust strongly fits manager/broker/state | VST3 SDK boundary and current Rust binding maturity | Pure-language preference must not override boundary risk | New permissive binding maturity |
| Does Proton remove the need for a bridge? | Proton and VST3 sources | None | Proton can provide Windows runtime | Native ABI/proxy still required | Treating Proton as the bridge would be architectural drift | Exact runner composition |
| Can Bitwig Flatpak host the product? | Current Flathub manifests | Not tested on maintainer fixture | Declared VST3/CLAP extension surface is favorable | Proxy/broker/runtime/editor path unproved | Current broad permissions cannot be permanent law | Exact sandbox behavior |
| Is Serum 2 a good first commercial target? | Xfer/Splice docs | User-selected target; not yet observed | Modern VST3, editor, presets, content, real authorization pressure | Exact local installer/build/channel unknown | Calling it “works” now would be false | Graphics, paths, callbacks |
| Is Kontakt a good first target? | NI docs | None | Excellent broad-system pressure | Too many independent uncertainties for first protocol seam | Letting it define early architecture would be overreach | Exact first library matrix |

## 4. Research nonclaims

- Public documentation is not a substitute for running the maintainer's exact installers and licenses.
- Vendor documentation can change; every implementation slice must re-check relevant current facts.
- No license or trademark analysis in this repository is legal advice.
- No external repository's architecture is accepted wholesale.
- No source establishes real-time performance on the Steam Deck; that requires measurement.
