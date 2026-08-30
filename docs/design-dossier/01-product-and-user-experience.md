# Linux Audio Compatibility Bridge

## Managed Windows Plug-ins for Native Linux DAWs

**Document identity:** `linux-audio-compatibility-bridge.soft-design-dossier.v1`  
**Document class:** `HUMAN_NORTH_STAR__REPOSITORY_OWNED__NON_IMPLEMENTATION_CLAIM`  
**Status:** Initial operator-requested design seed. It describes desired functionality, user experience, fixture pressure, technical realities, and proof ordering. It is not a release plan, schedule, compatibility promise, or implementation task card.  
**Prepared:** 2026-08-29 America/Los_Angeles.  
**First reference host:** Maintainer Steam Deck, SteamOS-derived environment, Bitwig Studio Flatpak.  
**First real commercial plug-in fixture:** Serum 2 VST3.  
**Later hostile-system fixture:** Kontakt plus Native Access and representative library/content cases.  
**Implementation consumption posture:** The operator and technical lead read this dossier. Coding agents receive only one distilled `CURRENT_SLICE.md` claim plus applicable architecture sections and exact fixture requirements.

> **Central ruling:** The product is not “Proton loads Windows plug-ins in Bitwig.” The product is a managed compatibility appliance. A native Linux proxy represents an exact Windows plug-in class to the DAW; a supervised Windows host loads the proprietary module under a pinned audio-oriented runner; a versioned real-time-safe transport connects them; and a manager owns installation, authorization handoff, scanning, organization, host publication, diagnostics, update, repair, and rollback. The user should not have to understand Wine prefixes, bridge synchronization, Flatpak mount paths, or runtime archaeology.

---

# 1. Product declaration

Linux audio users currently pay a compatibility tax in several unrelated currencies at once:

- choosing and maintaining a Wine build;
- deciding where installers and plug-ins live;
- constructing prefixes;
- adding runtime dependencies and fonts;
- generating native bridge proxies;
- making those proxies visible to a sandboxed DAW;
- handling browser login, vendor managers, machine activation, and recurring checks;
- recovering from a plug-in or Wine update;
- debugging editor focus, resizing, graphics, drag-and-drop, and popup windows;
- preserving presets, content paths, automation, and project state;
- distinguishing a plug-in problem from a bridge, runtime, sandbox, authorization, or content problem.

The intended product collapses those separate burdens into one explicit system.

The desired user-level statement is:

> Install and use supported Windows audio plug-ins in native Linux DAWs without manually managing compatibility infrastructure.

“Supported” is deliberately bounded. It means the product can state exactly which plug-in build, license channel, runner, environment policy, DAW build, sandbox posture, editor mode, content configuration, and capability set were exercised. It does not mean every Windows plug-in is promised.

## 1.1 What the product owns

The product owns:

- versioned audio runners;
- managed Windows environments;
- installer and companion-app supervision;
- authorization presentation and result classification;
- safe discovery and quarantine;
- exact plug-in/module/class identity;
- native proxy publication;
- process topology and supervision;
- IPC compatibility;
- real-time transport;
- editor-window coordination;
- host registration;
- content-root declarations;
- preset/state indexing without rewriting vendor truth;
- compatibility profiles;
- diagnostics, support bundles, repair, and rollback;
- exact compatibility claims and nonclaims.

## 1.2 What the product does not own

The product does not own:

- the DAW's project model, audio engine, transport, controller mappings, or native plug-in sandbox policy;
- the vendor's plug-in code, installer, updater, account, license, activation server, preset intellectual property, or sample library;
- the user's right to run or redistribute commercial software;
- Windows itself;
- a promise that Wine can reproduce every Windows driver, service, browser, anti-tamper system, or graphics path;
- scientific or artistic meaning of a preset;
- the user's music files;
- a hidden substitute for unsupported vendor licensing.

## 1.3 Why Proton matters without becoming the product

Proton demonstrates a powerful operational shape:

```text
known runtime
+ title-specific compatibility policy
+ isolated mutable state
+ deterministic launch environment
+ logs and override controls
+ rollback to another known runtime
```

Audio plug-ins need the same operational discipline, but not the same game-oriented assumptions. The project should selectively reuse or learn from Proton's Wine patches, synchronization work, redistributable build process, graphics components, and runtime pinning. It should not require Steam, add plug-ins as games, or blindly inherit fullscreen/game-container behavior.

The product concept is therefore **Proton-shaped and audio-governed**, not “stock Proton with a new launcher.”

---

# 2. Intended users and success experience

## 2.1 Musician or producer

The musician wants to install a familiar plug-in, find it in Bitwig, open it, play it, save the project, and have the same sound return tomorrow. The system should not turn them into a Wine administrator.

## 2.2 Linux audio power user

The power user wants exact versions, environment inspection, advanced overrides, process grouping, graphics choices, and exportable diagnostics. Advanced control should be available without making it mandatory for ordinary use.

## 2.3 Maintainer or compatibility engineer

The maintainer needs deterministic fixtures, structured capability censuses, process traces, crash isolation, state hashes, timing evidence, and compatibility-profile review. Workarounds must become reviewable data rather than folklore.

## 2.4 Support engineer

The support engineer needs to answer:

```text
Which exact layer failed?
What changed?
Can it be reproduced?
Can the environment be rolled back?
Can a sanitized support bundle show enough without exposing the user's account or licenses?
```

## 2.5 Plug-in or DAW vendor

A cooperative vendor may want a tested Linux compatibility profile, a supportable install recipe, or a future officially supported bridge target. The system should be technically transparent enough that a vendor can understand the boundary without surrendering its Windows build.

---

# 3. Irreducible product laws

1. **Native means native at the DAW boundary.** The DAW loads an ELF VST3 or CLAP proxy appropriate to Linux. The Windows module remains out of process.
2. **Exact identity precedes convenience.** Vendor, product, module, class, build, format, environment, runner, proxy, content, license channel, and host registration are not interchangeable.
3. **Names and paths are projections.** They help a person navigate; they do not establish identity.
4. **Install is a transaction.** The manager records base environment, installer identity, declared inputs, observed changes, discovered modules, scan results, and resulting environment revision.
5. **Authorization is vendor-owned.** The product presents and supervises lawful flows; it never fabricates or bypasses them.
6. **Secrets are not evidence.** Credentials, cookies, serials, activation files, tokens, and proprietary account data are excluded from logs and repository evidence.
7. **Compatibility is versioned.** A profile applies to exact declared conditions and can be superseded, withdrawn, or known-regressed.
8. **Working environments are immutable by default.** Updates create explicit revisions and rollback points rather than silently mutating a known-good runtime.
9. **Audio cannot depend on management.** The manager, installer, scanner, profile service, and UI are absent from the real-time callback path.
10. **Failure ownership is visible.** Installer, authorization, scanner, VST3 lifecycle, audio, editor, content, sandbox, and runtime failures retain distinct identities.
11. **A plug-in is not usable until it recalls.** Save/reopen, automation identity, parameter state, content resolution, and editor reopening are first-class acceptance.
12. **Detached correctness before embedded cleverness.** A reliable top-level editor is preferable to a beautiful but fragile embedded window.
13. **No universal claim from a flagship plug-in.** Serum 2 is a powerful first fixture, not a proof of every JUCE, VST3, synthesizer, or vendor.
14. **Content is not the prefix.** Large libraries and user presets need independent, relocatable, backed-up roots.
15. **The user can always see the actual chain.** The visual manager may simplify it, but it must not lie about which runner, environment, module, proxy, and host are active.
16. **Repair is reversible.** Destructive repair must be previewed and backed by a rollback or explicit abandonment path.
17. **The Steam Deck is evidence, not destiny.** The initial fixture must not hard-code one screen, distro image, home path, or Steam installation.
18. **The system can abstain.** Unsupported activation, missing content, ambiguous class identity, incompatible state, or unsafe update should stop with a clear result rather than guess.
19. **Commercial ambition does not excuse license confusion.** Every distributed dependency, patch, SDK, bridge component, and runtime has an explicit license and source-compliance posture.
20. **The system must be easier than yabridge administration.** If routine use still requires terminal-driven prefix archaeology and manual proxy synchronization, the product has failed even if its audio transport is excellent.

---

# 4. Product nonclaims

This dossier does not claim that:

- the final consumer brand is selected;
- the repository name is legally suitable as a product name;
- a repository-wide software license is selected;
- every component will be closed source or open source;
- the exact Wine or Proton fork is selected;
- UMU is a production dependency;
- the exact IPC serialization library is selected;
- a native UI toolkit, Tauri, Electron, web UI, or terminal UI is selected;
- editor embedding is required for first usability;
- a streamed/captured editor is proven;
- Bitwig's current Flatpak permissions will remain unchanged;
- `~/.vst3` is the final Flatpak packaging strategy;
- VST2 or 32-bit support belongs in the first product;
- CLAP support is implemented;
- Serum 2 is presently compatible;
- Kontakt is presently compatible;
- Splice Desktop, Native Access, iLok, CodeMeter, hardware dongles, kernel drivers, or background services are supported;
- project-state blobs are portable between plug-in builds;
- one vendor's license allows environment cloning or machine-identity migration;
- low-latency behavior is achieved merely because shared memory is used;
- this project will bypass anti-tamper, licensing, or unsupported drivers.

---

# 5. The end-user experience

The intended experience is a sequence of explicit transactions with progressive disclosure. The ordinary user sees a small number of meaningful states. The advanced user can inspect every technical fact beneath them.

## 5.1 First launch

The application performs a host census:

```text
Linux distribution and architecture
session type and graphics capabilities
available DAWs and installation modes
Flatpak applications and runtime branches
user plug-in paths visible to each DAW
shared-memory and Unix-socket capabilities
relevant sandbox permissions
available disk space and content locations
installed runners, if any
```

The user chooses a target such as **Bitwig Studio (Flatpak)**. The application runs a harmless native proxy probe and reports whether Bitwig can discover and instantiate it. This verifies the publication path before any commercial installer is involved.

The UI should say:

```text
Bitwig Studio 6.0.11 — Flatpak
Native proxy path: verified
Runner: not installed
Windows environments: none
```

It should not say “ready” merely because Bitwig was found.

## 5.2 Add a plug-in

The user chooses one of three acquisition forms:

- **Run a vendor installer** (`.exe` or `.msi`);
- **Run a vendor manager** such as Native Access or Splice Desktop;
- **Import an already installed module** into a managed environment.

For Serum 2, the likely first path is a lawful user-provided Windows installer. The manager identifies or asks for the product family, proposes an isolated Xfer environment, and shows the actions it intends to perform.

Example:

```text
Product: Serum 2
Environment family: Xfer Records
Runner: Audio Runner 0.1 / exact digest
Network during install: allowed
Network during scan: ask / profile-driven
Content root: ~/Music/Plug-in Content/Xfer
Rollback snapshot: enabled
```

The user can accept the default or inspect advanced details.

## 5.3 Installer session

The manager creates an environment revision and starts the installer in a supervised foreground session.

It records non-sensitive facts:

- installer digest and source classification;
- process tree;
- exit code;
- files created or changed within declared roots;
- registry-key names and value classes where safe, not secret values;
- installed runtimes and fonts;
- child services, scheduled tasks, URL handlers, and background launch requests;
- discovered VST3/CLAP modules;
- content and preset roots proposed by the installer;
- machine-identity-sensitive areas;
- cleanup outcome.

The user sees the real installer. The product does not pretend every vendor can be reduced to a silent install recipe.

## 5.4 Authorization session

Authorization is represented as a capability flow, not an accidental popup.

The manager can support declared patterns:

```text
in-plug-in credential form
external browser login
custom URL/deep-link return
localhost callback
downloaded offline license file
machine-ID challenge/response
companion desktop application
periodic phone-home verification
always-online requirement
hardware dongle or driver requirement
```

The manager opens the necessary foreground window or browser and tells the user which boundary is active. It records only outcomes such as:

```text
authorization class: Xfer lifetime / online browser
result: authorized
machine identity: stable internal identifier (not secret value)
offline alternative: vendor-documented
continuing phone-home: not observed / not required by this license channel
```

For Splice Rent-to-Own, the profile must separately declare the companion-app and recurring-check requirement. “Serum 2” alone is not enough to describe authorization behavior.

## 5.5 Isolated scan

The manager never asks Bitwig to discover an unknown Windows plug-in first.

A disposable scanner launches the module under the selected runner and performs a bounded census:

- module entry and exit;
- factory retrieval;
- factory metadata;
- every exported class and class identifier;
- component/controller relationship;
- supported interfaces;
- audio/event bus arrangements;
- parameter identities and flags;
- program lists and units;
- state round-trip probes;
- editor creation, platform support, initial size, resize behavior, and popup behavior;
- bounded process/CPU/memory behavior;
- crash, hang, child-process, and network observations allowed by the profile.

A module that crashes or hangs is quarantined. The application remains healthy and says exactly which stage failed.

## 5.6 Publish to the DAW

The manager generates or configures a native Linux proxy bundle whose identity maps to the exact scanned class. It publishes that proxy to the selected host path or Flatpak extension surface.

The publication transaction includes:

```text
host identity and version
native proxy build and digest
proxy ABI version
bridge protocol version
Windows module identity and digest
VST3 class identifier
managed environment revision
runner revision
compatibility profile revision
```

The user sees “Published to Bitwig,” not “symlink created.” Advanced details remain inspectable.

## 5.7 First host launch

When Bitwig instantiates the proxy:

1. the proxy validates its configuration and protocol compatibility;
2. the broker selects the exact environment and runner;
3. the Windows host starts or joins the permitted process group;
4. the host loads the module and instantiates the exact class;
5. component/controller/state interfaces are paired;
6. audio/event shared memory is prepared before activation;
7. the proxy reports buses, parameters, latency, and capabilities to Bitwig;
8. the editor remains closed unless the user opens it;
9. health and crash state are reported outside the audio callback.

The user should be able to play Serum 2 from a MIDI track, hear correct audio, automate a parameter, change a preset, save, close, reboot, and reopen with the same result.

## 5.8 Everyday organization

The application presents plug-ins by useful human concepts without erasing technical identity:

```text
Installed
Needs attention
Updates available
Quarantined
Unpublished
Content missing
Authorization needed
Known regression
```

The user can group or filter by:

- instrument/effect;
- vendor;
- product;
- native host format;
- authorization class;
- environment family;
- support state;
- editor mode;
- content location;
- favorite/tag;
- last used or recently repaired.

These groupings are metadata projections. Moving a card does not move a binary or rewrite project identity.

## 5.9 Update

A plug-in update is not an in-place surprise.

The application:

1. identifies the current installed module and environment revision;
2. creates a rollback point where legally and technically permitted;
3. runs the vendor updater or installer;
4. rescans the new module in isolation;
5. compares classes, parameters, buses, interfaces, state behavior, editor behavior, and dependencies;
6. tests state migration with non-sensitive fixtures;
7. stages proxy publication changes;
8. asks the user to activate the new revision;
9. retains the old compatible route until the transaction is accepted or explicitly discarded.

A runner update follows a different transaction. Plug-in and runner updates must never be conflated.

## 5.10 Repair

The repair UI begins with ownership:

```text
Host cannot see native proxy
Proxy/protocol mismatch
Windows host failed to start
Module failed to load
Class missing or changed
Authorization required
Content root missing
Editor unsupported
Audio deadline failures
Known runner regression
Environment corrupt or externally modified
```

Repairs are narrow. “Reset everything” is the last resort and must preview what will be lost.

## 5.11 Diagnostics export

The user can create a sanitized support bundle containing allow-listed metadata, version manifests, state-machine transitions, crash summaries, timing aggregates, and redacted logs.

It must not include:

- entire prefixes;
- account databases;
- browser cookies;
- license files;
- serials;
- proprietary plug-ins or installers;
- paid presets or sample content;
- project audio;
- personal home-directory listings unrelated to the product.

---
