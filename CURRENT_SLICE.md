# AP12 — Everyday Pure LoFi → Efx FRAGMENTS use in Bitwig

## Outcome and authority

After one setup transaction, the operator can restart the Deck, launch Bitwig normally from Applications, find **Pure LoFi** as an Arturia instrument and **Efx FRAGMENTS** as an Arturia audio effect, drag Pure LoFi onto an instrument track, place FRAGMENTS after it, open both real vendor editors, change and automate one exposed control on each, save the project, and reopen the same sound and automation without manually starting a preview owner, SSH session, or development checkout.

The working publications remain installed at handoff. Setup may initially use a typed CLI; routine music use may not depend on that CLI remaining open.

[Issue #78](https://github.com/kasselvania/Linux-VST-bridge/issues/78); branch `codex/ap12-arturia-everyday-use`; basis main `1d5d693e2e2d12547dfd6b02e2b8492b27418592`. AP11/#76 is merged. Serum-specific lawful authorization/editor qualification remains separately tracked in #77. Issues #72/#73/#74 remain ordinary backlog, not gates to this work. Implementation, necessary builds, installer execution, focused device verification, and ordinary in-scope repairs are authorized under [AGENTS.md](AGENTS.md). No additional selection, receipt, or permission-document loop is required.

## Establish a normal Arturia installation first

Use the existing user-owned Pure LoFi 1.0.0 and Efx FRAGMENTS installer artifacts on the Deck. Verify their exact paths, hashes, versions, and regular-file ownership before execution. Preserve the existing working FRAGMENTS environment, the historical Pure LoFi failure environment, installed vendor files, projects, and license/account state as rollback evidence.

Create a persistent bridge-owned Arturia environment and run the normal vendor installers in their real foreground UI. A shared Arturia family environment is appropriate when the installers and resulting modules support it; split environments only for a demonstrated compatibility or ownership reason. The installer UI itself is sufficient for this slice—do not build a complete graphical manager.

Record non-sensitive installation facts needed for later support: installer identity, process outcome, installed module/class paths and hashes, resource/content roots, runner identity, and authorization posture. Do not commit proprietary binaries, presets, credentials, cookies, license files, serials, or account data. Do not bypass licensing. If a vendor/operator action is required, present the lawful window and retain the exact result.

The old Pure LoFi result is not a compatibility verdict: AP8 copied an existing module and selected resources into a prepared environment, then observed `IComponent::initialize()` return `kResultFalse`. Re-test the normally installed module using the current host. If it still fails, trace the exact call, dependency, path, or runtime boundary and repair the demonstrated cause. Do not switch plug-ins or runners merely to obtain a pass. A genuine vendor/authorization blocker may leave the Pure LoFi portion incomplete, but it must not become an unrelated installer hunt or fabricated success.

A normal FRAGMENTS installation may also clarify its current resource-integrity warning. Investigate actual installed layout and vendor requests; do not create guessed files or erase the warning without evidence.

## Preserve exact identity and publish familiar devices

Carry the Windows factory and class metadata required for publication: vendor, class name, class ID, version where available, and declared VST3 subcategories. Browser role is determined from the format metadata—not inferred from whether the class has audio inputs. An instrument may accept audio, and an effect may accept events. Use `IPluginFactory2`/`PClassInfo2` or the applicable richer factory interface when available, with an explicit fallback when it is not.

Publish two independently selectable native devices whose human-facing metadata matches the inspected classes:

- **Arturia · Pure LoFi · Instrument**
- **Arturia · Efx FRAGMENTS · Audio FX**

Preserve the established native processor/controller ID derivation from the exact Windows class identity, the real parameter IDs, and saved-project bindings. Friendly names, paths, builds, and environment revisions do not regenerate class identity. Bridge build/provenance remains inspectable in technical details; product presentation must not imply Arturia authored or officially supports this bridge.

Publication must be atomic and idempotent. Repeating setup must not create duplicate Bitwig entries, generic “Commercial Instrument Bridge” entries, or new class IDs. Removing a bridge registration may remove only bridge-owned publication and product state—not vendor files, content, authorization, or user projects. Whether the implementation uses separate bundles or multiple classes in one module is private engineering latitude as long as Bitwig presents the two exact devices correctly.

No product-name switch may determine behavior. Module/class metadata and registered compatibility state drive the generic path.

## Replace preview preparation with persistent registered startup

Implement the smallest real management core needed for this vertical: register/import the installed classes, publish/status/unpublish them for Bitwig, bind each mapping to an exact environment and runner revision, and start the supervised Windows host automatically when Bitwig instantiates a published proxy. Rust owns the canonical registration/publication state and command surface. Existing Python supervision may be reused where it remains sound, but installed playback must not extract helpers with `git show` or depend on a repository checkout.

A systemd user service, socket activation, or an equivalently bounded user-session mechanism is appropriate. The native proxy requests an exact registered mapping; it must not launch arbitrary Wine commands or scan the filesystem from the audio callback. Normal operation must not depend on `.git`, a build directory, `AP9-Performance/serum`, `AP8-Commercial-Test/preview`, a manually started `ap8_preview.py`, or a running agent.

Support at least one Pure LoFi and one FRAGMENTS instance concurrently. Each DAW instance owns distinct transport, audio/event buffers, state, editor session, automation, failure, and cleanup identity. Sharing one Arturia environment does not permit sharing mutable processor/controller state or one session directory. Removing or closing either instance must leave its healthy sibling running. The existing commercial `capacity = 1` and single-session-directory assumptions are not acceptable product behavior.

Registration and status operations stay outside real-time audio. Hashing, scanning, service startup, environment verification, and UI refresh may not enter the callback path or trigger fabricated parameter invalidation/state capture. Preserve AP11’s repair: opening or focusing an editor is not a reason to request a full parameter refresh.

## Compatibility settings belong to the product mapping

Retain the currently verified pinned Proton runner first. Store exact runner/environment/profile identity rather than following whichever `wine` or Proton build is newest. FRAGMENTS’ explicit per-process Windows-accessibility override must be represented as a visible compatibility setting for that product/profile only; do not mutate the entire Arturia environment or every plug-in process.

`giang17/wine` branch `d2d1-dcomp-11.0`, last reviewed at `0077f1c63098d65a4d2554cd31d07903773cf992`, is a serious candidate runner source beneath our independent bridge. It is not a replacement for our proxy, Windows host, transport, state, editor, or management work and does not reopen the no-yabridge decision. Do not build or migrate to it merely because it exists. For a matching rendering/window/runtime failure, identify the loaded graphics path and compare one coherent pinned runner in an isolated reversible environment. Do not mix individual DXVK/WineD3D/DXGI DLLs, apply global Wine-detection overrides, or assume its 64-sample FL Studio result measures our bridge. Reverify the Wine X11 window-ID capability used by AP11 on any alternate runner.

## Musical behavior and latency

Bitwig owns the serial device chain. The bridge publishes normal independent devices; it does not create a private Pure LoFi-to-FRAGMENTS connection.

Pure LoFi must accept the class’s actual note/event and audio configuration and produce note-driven sound. FRAGMENTS must accept Pure LoFi’s audio output through Bitwig and return input-dependent processed audio. Auxiliary/sidechain buses may remain inactive when not yet supported, but must be represented and refused honestly rather than silently misrouted.

Each vendor editor controls the same processor instance producing that device’s audio. Choose one useful vendor-exposed automatable parameter per plug-in. Verify vendor UI → Bitwig automation writing and hands-off Bitwig replay → Windows controller/DSP/UI, with sound agreement. Not every vendor button or preset-browser action is required to be automatable.

Retain **512 added frames per proxy** for this slice. Two serial bridge instances therefore contribute 1,024 frames—21.33 ms at 48 kHz—before vendor and hardware latency. Report the actual whole-chain delay and existing gap counters. Do not increase buffering to conceal a regression or run an undirected latency matrix. When a new gap appears, follow the first originating request and repair a demonstrated cause where practical.

## Completion

AP12 is complete when all of the following are true on the actual Deck/Bitwig fixture:

1. The normal user-owned installers establish an exact persistent Arturia environment, and Pure LoFi initializes and produces note-driven audio through the current bridge.
2. After setup exits and the Deck or user session restarts, normally launched Bitwig shows Pure LoFi under instruments and Efx FRAGMENTS under audio effects, with no generic or duplicate bridge entries.
3. The operator creates a Pure LoFi → Efx FRAGMENTS chain from Bitwig’s browser; the real sound passes through both independent Windows instances.
4. Both real vendor editors open/focus/reopen without replacing DSP. One automatable parameter per plug-in records and replays through Bitwig with the corresponding vendor control and sound following.
5. The project saves, Bitwig quits, the system restarts, and the project reopens with both exact classes, settings, chain order, automation, and usable sound without setup tooling running.
6. Removing FRAGMENTS leaves Pure LoFi playing; closing either editor or instance leaves its sibling healthy. Owned sessions retire cleanly.
7. Focused tests cover exact metadata/classification, idempotent publication, changed/missing module or runner refusal, interrupted publication, duplicate startup, and independent instance/session ownership.
8. Temporary diagnostics and unrelated settings are restored, while the intended Arturia environment, registrations, services, proxy publications, and user project remain installed for human use.

Publish one implementation PR against main with actual results, exact versions, remaining limitations, and cleanup. Leave it unmerged for technical review.

## Outside this slice

A polished graphical manager, universal installer catalog, automatic vendor updates, arbitrary multichannel/sidechain completion, broad Arturia support, automatic runner migration, CLAP, Serum authorization/qualification (#77), and a new lower-latency default are not required. Do not convert these exclusions into reasons to stop ordinary in-scope repair.