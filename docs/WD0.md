# WD0 — Managed FL Studio workspace: first usable stock project

Selected by the operator on 2026-09-24. Implementation work order, not a completed qualification. Architecture: [WINDOWS_DAW_WORKSPACES.md](WINDOWS_DAW_WORKSPACES.md). Work allocation: [WORKSTREAMS.md](WORKSTREAMS.md).

## Outcome and fixed scope

From the normal manager/desktop entry, open the operator's exact Windows FL Studio in a dedicated managed workspace, make a short stock-instrument pattern, hear it through Linux audio, save and export it, close cleanly, and reopen the same project after a fresh launch when licensed.

Deliver one canonical implementation PR with that connected result, not separate framework, installer-proof, audio-proof and closure PRs. A CLI plus an ordinary desktop launcher and useful status is sufficient for WD0. The engineer may repair in-scope faults and continue without a new review at each step. Review is before merge; existing tool, spending, user-data and security permissions still apply.

First fixture: x86-64 Steam Deck Desktop Mode, mouse/trackpad, one FL workspace, stock installed content. No third-party VST installation, Ableton, Max for Live, ARM, Gaming Mode, new distro campaign, full manager redesign or touch-repair dependency in this cut.

## Repository and installed-state starting point

Canonical documentation base: `05cb957e0174c3437aac8934b89294e57b53308f`, tree `ce720e73fdcbe7b80434ba62d207d6364b5c8654`. Start implementation after this design/work-order change is reviewed and merged, on `codex/wd0-fl-studio-workspace` from then-current canonical main. Reconcile intervening changes; never reset another agent's branch or worktree.

Read [AGENTS.md](../AGENTS.md), [ARCHITECTURE.md](ARCHITECTURE.md), this work order and its architecture extension. Reuse relevant source owners, particularly `bridge-manager/src/vendor_application.rs`, `renderer_application.rs`, `installer_import.rs`, `onboarding.rs`, `setup_install.rs`, `operator_model.rs`, and runtime supervision. Inspect their actual contracts before reuse: ASC and Native Access selectors are deliberately closed.

Read the existing failure classes for graphics/session authority, positive retirement, exact host/source continuity and audio configuration. The native bridge's FC-AUDIO-002 block restriction does not mechanically apply to a directly hosted Windows DAW.

Before physical work, read the actual Deck installation and coordinate with the current custodian. Serum PR #151 is unmerged but candidate C is installed. WD0 must not overwrite the working bridge service, its default host/source selection, installed `x11_touch_release_v1` policy, six publications, vendor prefixes or Bitwig project. Add workspace-only product commands and a separate private FL workspace/application service; do not turn this into a forked manager implementation.

## 1. Admit the installer and create one workspace

The operator is obtaining the current official Windows installer. The official download page showed **26.1.6.5639** on 2026-09-24. This is a discovery hint, not an admitted hash or permission to substitute bytes.

When the local file is available:

- identify its original filename, actual size, SHA-256, PE/version facts and publisher/signature result where available;
- distinguish the x64 application target from the installer bootstrapper's own PE architecture;
- confirm the operator's official-source handoff; preserve the original download;
- import one exact private copy using the existing installer custody pattern;
- bind the observed version and hashes in the private installation record; do not put local paths or license state in a public compatibility profile.

No installer is attached or its hash known at planning time. Never invent a digest. If the downloaded official build is newer, record and pin that exact operator-selected build rather than silently fetching a second installer.

Create a fresh manager-owned FL workspace and prefix. Reuse the verified standard Proton 11.0-2c/SLR4 runtime as the first runner candidate; do not inherit the experimental Serum touch or Blackhole DComp override policy. Reference the exact available immutable runtime closure, not a moving Proton/GE-Proton label. If the first concrete FL failure requires a runner change, bind one coherent successor in the FL workspace only.

Do not adopt or clone any existing licensed Arturia, Serum, NI or Blackhole prefix. Installation resources, registry, data roots and authorization belong to the new workspace. No shotgun winetricks bundle or unselected updater. A demonstrated missing dependency may be supplied through a narrow official/verified installation within this workspace.

Run the real installer through existing exact launch/root ownership. Preserve distinct installer completion, application identification and successful application execution. Let the operator handle vendor consent and account entry. Do not install optional cloud products, unrelated drivers/content or paid add-ons just because the installer offers them.

## 2. Implement the minimum canonical DAW owner

Add an adjacent typed FL workspace owner rather than using `ApplicationId::Other` or abusing a VST registration.

Expose the following capabilities using existing CLI/operator patterns; final command spelling is the implementer's choice:

```text
workspace import/install
workspace launch
workspace focus
workspace status
workspace stop (graceful)
```

The normal launch selects a known workspace ID, exact installed application and admitted runtime. It does not accept an arbitrary shell command, user-supplied renderer flags, process name or PID as authority. Project selection is restricted to the explicit approved project roots through normal file selection, not interpolated shell arguments.

Retain exact application/helper generation ownership, launch outcome and first useful failure. Distinguish installing, ready, running, needs-user-action, failed and cleanup-unconfirmed. Repeated launch focuses or truthfully refuses an existing instance. Startup/readiness must not require a bridge keeper that only a VST admission would create. FL is not a fake bridge DSP instance.

Give the workspace an ordinary desktop entry using the same product launch route. It must work from a normal graphical session without a development shell, ad-hoc environment exports or a Mac SSH session remaining alive. The GUI frontend may follow; do not gate usable launching on a full UI rewrite.

Use graceful close and exact cohort retirement, preserving the user's save opportunity. Do not kill a healthy FL session at an arbitrary test timeout. A forced stop needs explicit operator confirmation and must leave project bytes intact; unknown ownership blocks destructive cleanup. Do not stop another prefix's Wine server or the native-bridge service.

## 3. Keep runtime projection coherent

FL and its scan/helper processes share the same workspace filesystem, registry, Wine server and runner. Reuse the runner's required Steam Runtime/pressure-vessel layer; do not add one independent sandbox for each plug-in.

Provide only the actual required graphics, audio-service/driver, chosen input and project/content surfaces. Preserve exact Xauthority and session changes using existing ownership rules. A browser-based unlock or official offline unlock remains user-operated; network/browser access is explicit and scoped, never supplied by disabling host protections.

The runtime dependencies must resolve inside the launched workspace. A file or library visible only to the host shell is not evidence it is visible to FL. Keep stable prefix/library paths when Wine creates absolute builtin references. Keep durable projects and preferences out of `/run` and temporary capture roots.

## 4. Establish real audio, then the selected ASIO route

First launch FL with stock content and obtain audible playback through an actually available built-in Windows audio route. Record the backend selected and observed Linux audio endpoint. A visible meter alone is insufficient.

Then attempt the selected managed route:

```text
FL Studio ASIO client
  -> exact 64-bit WineASIO driver
  -> PipeWire JACK client implementation inside the runtime
  -> existing PipeWire server and selected output
```

Build/admit the driver against the exact selected runtime. Register it only in the new FL prefix, never default `~/.wine`. Preserve complete matching PE/Unix artifacts and dependency identity. If the runner needs new library placement, use a private immutable workspace-specific runtime/driver closure, not a mutation of the shared runner.

Verify the loaded JACK library is PipeWire's implementation from within the runtime; `pw-jack` outside Proton alone is not proof. Do not autostart a separate JACK server. Keep WineASIO server autostart disabled and shared-graph buffer changes disabled; use the actual existing audio-server policy rather than rewriting system audio configuration.

Use 48 kHz and an initial 512-sample driver/graph target where supported by the actual endpoints. Retain FL's selected and effective driver buffer, graph rate/quantum, underrun observations and output identity separately. Here there is no added Linux-proxy transport delay and no inherited six-DSP limit. Do not call buffer arithmetic measured round-trip latency.

No global priority change, full-home mount, host firewall change, or driver tournament. A concrete incompatibility can justify one maintained alternative within the same slice. Preserve working built-in audio as a partial result if ASIO remains blocked; do not mislabel it a low-latency ASIO pass or reopen the native-bridge performance campaign.

Physical MIDI input is tested only with an available operator-selected device. A piano-roll note clip establishes FL event playback, not hardware MIDI or round-trip input latency. Input recording, multi-interface I/O and controller-script coverage remain separately named follow-up work unless a simple already-available input is exercised.

## 5. One small musical workflow

Use a protected new project and a stock instrument included in the chosen edition; no internet sound-pack download or trial-only premium device is required.

1. Launch FL from the new normal desktop/manager entry.
2. Create a short note pattern, arrange it and play it audibly.
3. Adjust one stock control and one mixer level; confirm the sound follows.
4. Save the FLP in the stable approved project root.
5. Export a short WAV through FL's own export operation; verify a finite, nonempty output and retain it privately for the operator.
6. Quit normally and verify the exact FL/helper cohort retires; the shared native-bridge service and publications remain intact.
7. Relaunch normally. When licensed, reopen that same FLP and confirm its notes, stock setting, routing and audible playback. Close normally again.

No repeated matrix or forced reboot is required for this first user-launched workspace. If startup/login integration is changed beyond its desktop entry, test that affected boundary rather than infer it.

The trial can save and export but cannot reopen saved FLPs. Detect/record that limit as `trial_limited`; do not debug it as Wine corruption, buy a license automatically, or automate credentials. Ask the operator only for the normal unlock when needed. If no license is available, retain playback/export/clean-relaunch success and report project recall as blocked by licensing. Do not claim full WD0 completion. Also avoid trial-only stock devices that could disappear on recall even in an otherwise licensed edition.

Touchscreen coverage is not required for WD0. Use mouse/trackpad while FC-UI-002/003 remains open. A future shared Wine fix may be evaluated for FL after it is merged and preserves this workspace's complete runtime requirements; the native bridge's own editor-pump fix is not executed inside FL's vendor-owned host.

## Validation and result

Use focused production-helper tests for workspace identity, immutable application/runtime binding, repeated launch/focus, real child lifetime versus parent exit, exact stop/refusal, stable project roots and missing audio endpoint. Exercise the existing installer/supervisor regressions where changed. Do not build a new diagnostic framework or rerun AP17/Ubuntu/RPI campaigns.

Before review, run the touched manager/CLI tests, strict Clippy where Rust changed, relevant Python/shell checks and documentation checks. Record unavailable hosted execution honestly; zero-step CI is not success. Do not raise account spending limits.

Completion requires implemented manager-owned install/launch/status/stop and normal desktop launch, actual stock-project audio through a named backend, export, clean relaunch, licensed same-project recall, and no mutation of the existing working fleet. Report ASIO and physical MIDI independently. Full DAW support, touchscreen, all plug-ins, low-latency round-trip guarantees and commercial release readiness are not implied.

Return one implementation PR with exact source/tree, admitted installer/release, runtime and driver identities, chosen paths and process boundary, observations and first useful failures, physical workflow result, cleanup and rollback posture. Public evidence is sanitized. Private projects, audio, account state and vendor binaries stay private.

Update the relevant existing failure-class cards and support rows for actual changed understanding; add a new class only for a demonstrated reusable failure. Until physical completion, FL appears as planned/in progress, not supported. An empty launcher or schema-only PR is not the outcome.

## After WD0

WD1 installs one exact third-party Windows VST3 through its lawful installer in the same FL workspace, discovers it using FL's own scanner, and exercises play/editor/save/reopen/retirement. No Linux publication or bridge transport is created for that direct route. Select the product after checking runtime and license compatibility; do not import the entire current fleet at once.

Then expand the supported FL plug-in set and everyday manager UI. Ableton follows the proven workspace owner in a separate prefix, initially without Max for Live. The native-Linux bridge remains the first release-driving product throughout.
