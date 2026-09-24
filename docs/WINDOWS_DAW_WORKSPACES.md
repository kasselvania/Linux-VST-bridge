# Managed Windows DAW workspaces

Decision date: 2026-09-24. Status: selected architecture extension for implementation; no Windows DAW is qualified by this document. The first outcome is [WD0 — FL Studio](WD0.md). Allocation and integration rules are in [WORKSTREAMS.md](WORKSTREAMS.md).

## Architectural ruling

The manager owns installation, environments, runtime selection, launch ownership, diagnostics and support claims. It does not need to own every audio call.

The four-plane path in [ARCHITECTURE.md](ARCHITECTURE.md) remains the native-Linux-DAW path:

```text
Bitwig or another qualified native Linux DAW
  -> Linux VST3 proxy
  -> project audio/state transport
  -> project Windows VST host
  -> Windows plug-in
```

The additional managed-Windows-DAW path is:

```text
same canonical management product
  -> workspace-scoped application launcher
  -> FL Studio under a pinned Wine/Proton runtime
       -> FL's own instruments and plug-in hosting
       -> Windows audio API / selected driver
       -> Linux audio service and device
```

Windows FL loads Windows plug-ins directly. Do not route it through the Linux proxy and back through the existing bridge. The native bridge's added-frame policy, six-DSP qualification, per-instance DSP leases, host pump and native terminal view are not automatically FL requirements or FL guarantees.

A workspace is not one giant universal prefix. FL and a later Ableton installation start in separate managed prefixes. Within an FL workspace, the DAW and its plug-ins use one coherent Windows filesystem, registry, runner and Wine-server cohort. They need not run in one process; FL owns its internal hosting decisions.

## What is reused and what is new

Reuse the Rust environment/artifact verification, installer supervision, exact process-generation ownership, private diagnostics, session-specific graphics/audio projection and existing installation/update primitives where their contracts fit. Existing ASC/Native Access code is a source of reusable owners, not a generic DAW implementation.

At the decision baseline, `bridge-manager/src/vendor_application.rs` has a closed ASC application identity; [NAUI2](NAUI2.md) deliberately limits the adjacent owner to Native Access. Do not weaken those selectors with an arbitrary `Other`, shell command, executable path or argument array. Add a small adjacent `DawWorkspace` owner and typed FL application action, factoring actual shared primitives rather than copying vendor code or designing a universal launcher first.

Minimum persistent facts, with private representation left to the implementer:

- workspace ID and revision; admitted DAW application identity and exact installed release;
- managed environment root/revision, pinned runtime closure and installation result;
- admitted executable/resources and known helper ownership;
- selected audio-backend identity and observed configuration;
- explicit writable project/user-data roots and later plug-in/content roots;
- application/session state, first useful failure and confirmed or uncertain retirement.

Executable and runtime generations are immutable. A Wine prefix, DAW preferences, user projects and authorization state are intentionally mutable under their owners. Do not describe the entire workspace as an immutable file tree or hash normal preference writes as corruption.

Workspace-specific installation and launch state must not rewrite the native bridge's `software.json`, profiles, publications, environment markers or service selection. The first WD0 package can expose workspace-only commands and a desktop entry from the canonical source, without replacing the shared native-bridge service. Its application state belongs to a distinct manager-owned workspace root. Later unified UI presentation reads these owners; it does not create another authority database.

## Runtime and plug-in compatibility

Different plug-ins currently need different exact runner policies. A direct-host workspace can select only a coherent runtime for its whole DAW/plug-in cohort. Do not assume the Blackhole DirectComposition runner, NI installer correction and Serum experimental touch runner are interchangeable or composable.

WD0 begins with stock FL instruments only and one admitted runtime. WD1 installs one exact third-party Windows VST3 through its lawful installer in that same workspace. It does not copy an activated prefix, registry/license blobs, or a `.vst3` file and assume resources and registration came with it.

Reuse installer caches and immutable runtime artifacts where permitted. Defer shared mutable libraries, deduplication, cross-prefix plug-in hosting and runtime patch aggregation until a concrete workload needs them. A later runtime change must preserve already-required fixes and qualify the DAW plus the installed plug-in set; a plug-in-specific runner swap cannot silently change the rest of the workspace.

Existing native-Linux publication claims do not become direct-FL claims. Conversely, a direct-FL pass does not qualify a Linux proxy. Use distinct execution-lane coverage in [SUPPORT_MATRIX.md](SUPPORT_MATRIX.md) when physical evidence exists.

## Containment and session ownership

Use one coherent application runtime view for FL and its children. Reuse pressure-vessel/Steam Runtime when required by the selected runner. Do not nest the Ubuntu qualification sandbox around each in-process plug-in, and do not treat Bubblewrap removal as the product outcome.

Define only the workspace's actual surfaces: runtime dependencies, graphical session/Xauthority, audio-service socket and driver libraries, selected input devices, workspace data and explicitly chosen project/export roots. Keep other prefixes, credentials and unrelated home data outside that view. Where online installation/unlock is selected, scope network access to that operation/workspace; no host firewall or security disabling.

A graphical-session transition, desktop-service restart or leftover process must be handled by the existing positive-ownership principles. Do not create a fake bridge DSP registration or load a dummy VST merely to keep the workspace alive.

The application session, not the launcher parent's exit code alone, owns FL and its exact helpers. A second launch should focus the same session or refuse clearly, not start an untracked second writer. Normal sessions have an interactive lifetime, not an arbitrary diagnostic expiry.

Graceful stop requests preserve the user's save opportunity. A stop timeout remains incomplete/failed, not success. Forced termination requires the operator's explicit confirmation of possible unsaved-work loss and targets only the exact workspace cohort. Never use a global `wineserver -k`, `killall wine`, or process-name-only cleanup.

The manager can report workspace/application ownership and available driver statistics. Without an explicit FL interface it cannot truthfully report each FL-hosted plug-in as an existing bridge-owned DSP session or promise per-plug-in recovery.

## Audio decision

First establish audible stock-project playback through the selected Wine runtime's existing Windows audio route. Name the actual backend; do not infer it from a successful launch or a dropdown label.

The first candidate for a managed ASIO route is WineASIO -> PipeWire's JACK client library -> the existing PipeWire server. This is a selected implementation candidate, not a certified Proton combination. WineASIO documents both the ASIO-to-JACK design and per-prefix registration. PipeWire documents that `pw-jack` redirects client-library loading; a host-side wrapper alone does not prove the required library survived a Proton/runtime boundary.

Bind the exact WineASIO binary pair, runtime ABI, loaded `libjack` implementation, audio endpoint and prefix registration. Use an immutable workspace-specific driver/runtime closure if placement in Wine's library tree is required. Do not hot-copy files into a shared runner or register into the default `~/.wine`.

Start at the existing 48-kHz/512-sample operating conditions to reduce variables, but measure FL's actual driver block and Linux graph quantum separately. Here 512 is an initial driver setting, not the native bridge's mandatory 512 added frames. No bridge transport is in this audio path. Device latency, scheduling and vendor processing still exist; do not claim zero or measured round-trip latency from buffer arithmetic.

Do not start a second JACK server or let FL change shared graph settings silently. Pin workspace driver settings to use the existing server; leave machine-wide PipeWire configuration and scheduling untouched. If the WineASIO ABI/path combination is concretely incompatible, retain that failure and choose one evidence-led maintained alternative. Do not run a driver tournament or write a new ASIO driver in WD0. A working built-in backend is a legitimate partial result, but not an ASIO qualification.

## Persistence and commercial boundaries

Projects, exports, preferences and content need stable user-visible locations independent of volatile runtime cleanup. Runtime rollback is not permission to roll back the user's newer project or licensing state. Installer mutation, package replacement, project persistence and launch retirement are separate operations.

The official Windows installer is supplied by the operator and kept private. Record exact bytes, release and publisher/signature result; never commit an installer, account credential, activation file, raw vendor trace or proprietary project content. Authentication and licensing stay in the vendor UI or official offline flow with the user; no credential automation, license cloning or bypass.

FL's trial can save/export but cannot reopen saved projects. A trial-limited result must distinguish working playback/export/relaunch from untested licensed project recall. Use stock instruments included in the user's edition for recall checks, avoiding trial-only device substitution.

The same management platform can later host Ableton in another workspace. Ableton core DAW qualification and Max for Live are separate scopes. Neither is admitted by FL results. The ARM appliance remains a third execution backend with separate CPU/runtime and support evidence.

## Source references

Checked 2026-09-24; exact installed bytes outrank mutable web version labels.

- [Image-Line official Windows download and trial limits](https://www.image-line.com/fl-studio/download)
- [FL Studio edition and trial-project behavior](https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/app_feature.htm)
- [FL Studio audio settings](https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/envsettings_audio.htm)
- [WineASIO implementation and registration](https://github.com/wineasio/wineasio)
- [PipeWire JACK client redirection](https://docs.pipewire.org/page_man_pw-jack_1.html)
