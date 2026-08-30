# Fixture Cards

Fixture cards define pressure and required evidence. They do not declare compatibility.

## F0 — Open deterministic VST3 reference fixtures

### Role

Test instrumentation for CI, protocol coverage, deliberate failures, and exact assertions. These fixtures are not the first real product target.

### Required fixture family

The repository should eventually own or lawfully consume redistributable modules covering:

1. **Minimal effect** — deterministic gain with exact parameter/state/audio hashes.
2. **Minimal instrument** — MIDI/note input to deterministic audio output.
3. **Multi-class module** — at least two classes from one VST3 factory.
4. **Split component/controller** — independent lifecycle and connection messaging.
5. **Reentrant callback fixture** — controlled host callback during an outstanding call.
6. **Resizable editor fixture** — open, resize, popup, keyboard focus, close/reopen.
7. **Crash fixture** — controlled crash at selected scan/lifecycle/process stages.
8. **Hang fixture** — bounded scanner/host timeout proof.
9. **Deadline fixture** — controlled slow process call.
10. **State fixture** — large and small deterministic state blobs, rejection of malformed state.

### Evidence

- source revision and license;
- VST3 validator results where applicable;
- exact factory/class/interface census;
- expected audio/state/event hashes;
- intentional failure controls;
- no test fixture is confused with commercial support.

---

## F1 — Serum 2 VST3

### Fixture status

**Selected as first real commercial plug-in. Not yet observed or verified in this repository.**

### Why it matters

Serum 2 provides a useful early vertical without immediately requiring a massive companion ecosystem:

- current 64-bit Windows VST3;
- high-value instrument behavior;
- graphical, resizable editor;
- rich presets, wavetables, and sample content;
- MIDI/note, automation, state, and project recall pressure;
- real authorization with online and documented offline forms;
- distinct lifetime and Splice Rent-to-Own channels.

### Required exact fixture declaration

Before any result:

```text
Serum 2 exact version/build
installer SHA-256 and acquisition class (not committed)
license channel:
  Xfer direct/owned
  Splice paid-off lifetime/Xfer
  Splice active Rent-to-Own
  demo (limited claim only)
Bitwig exact version and installation mode
SteamOS/distro/kernel/session
runner exact revision/digest
managed environment exact revision
native proxy/protocol exact revision
content roots and filesystem
network posture
```

### Installation questions

- Is installation direct `.exe`, Splice-managed, or another route?
- Which VST3 module path is produced?
- Which redistributable runtimes/fonts/components are installed?
- What child processes and windows appear?
- Are URL handlers or browser callbacks registered?
- Where do factory content, user presets, wavetables, samples, noises, and configuration actually live?
- Which roots are vendor-selected versus user-selectable?
- Can content be externalized without breaking authorization or project recall?

### Authorization classes

#### Xfer owned/lifetime

Expected documented pressure:

- authorization UI inside first launch;
- external Xfer account/browser flow;
- machine identity;
- documented offline challenge/license-file alternative;
- finite machine activations.

Required observations:

- exact window/process/browser handoff;
- callback mechanism, if any;
- whether the DAW plug-in instance must remain alive;
- which non-secret environment facts must remain stable;
- offline launch after successful authorization;
- behavior after runner update with same environment;
- behavior after environment move/restore without claiming cross-machine rights.

#### Splice active Rent-to-Own

Additional documented pressure:

- Splice Desktop installation/login;
- recurring authorization checks;
- limited offline window;
- update ownership through Splice;
- companion process/service lifecycle.

This is a distinct profile and compatibility claim. Never inherit an Xfer-owned result.

### VST3 census requirements

- factory info;
- exported classes and IDs;
- component/controller relationship;
- full supported-interface census at each proxy depth;
- input/output audio and event buses;
- parameter count, IDs, units, flags, programs;
- latency/tail/silence behavior where exposed;
- process setup and sample format support;
- editor platform, initial size, resize constraints, scale behavior;
- context menu, popup, drag/drop, file-dialog behavior;
- state sizes and repeatability characteristics.

### Musical acceptance ladder

1. Bitwig discovers exact proxy/class.
2. Instance creates and destroys repeatedly.
3. MIDI/note events produce audio.
4. Sample rates and buffer sizes under the declared matrix work.
5. Core parameter automation is sample-offset-correct where observable.
6. Preset changes remain stable.
7. State get/set restores sound and visible settings.
8. Bitwig project save/close/all-process-stop/reopen restores the instance.
9. Steam Deck reboot/reopen restores the project.
10. Multiple instances work under declared process grouping.
11. Editor open/resize/popups/focus/close/reopen are usable.
12. Forced editor and Windows-host failure follow declared containment.
13. Runner and plug-in update/rollback preserve the last known-good route.

### Required negative tests

- module absent after installer;
- authorization cancelled;
- browser callback timeout;
- network denied at first authorization;
- content root missing;
- user preset root read-only;
- scanner crash/timeout simulation around the commercial fixture without modifying it;
- Windows host killed during audio;
- editor killed/closed unexpectedly;
- protocol major mismatch;
- runner missing/tampered;
- changed module digest under same friendly version;
- project reopen with old environment unavailable;
- Splice companion closed/offline only when that license channel is tested.

### Claim language

Acceptable:

> Serum 2 build X under Xfer-owned authorization was exercised on Bitwig Y Flatpak/runtime Z using runner R, environment E, proxy P, and profile C for the listed capabilities and limitations.

Prohibited:

> Serum works on Linux.

---

## F2 — Second ordinary commercial family

### Role

Prevent overfitting before Kontakt.

### Selection pressure

Choose a legally owned 64-bit Windows VST3 from another vendor with at least two of:

- different framework/editor graphics path;
- effect rather than instrument;
- different authorization model;
- installer without companion manager;
- unusual bus/sidechain layout;
- larger state;
- drag-and-drop/file browser use;
- multiple classes in one module.

### Required result

The same manager, broker, proxy, transport, profile schema, and diagnostics core can express the fixture without hard-coded Serum logic.

No product is selected by this initial dossier.

---

## F3 — Kontakt hostile-system fixture

### Fixture status

**Selected as a later hostile systems test. Not an early implementation target and not yet scoped to an exact library matrix.**

### Why it matters

Kontakt pressures the entire product rather than only the bridge:

- Native Access companion manager;
- login, activation, install, and update;
- separate application/download/content paths;
- very large libraries;
- internal/external/removable storage;
- locate/relink and missing-content repair;
- Kontakt Player versus full Kontakt;
- licensed versus unlicensed third-party libraries;
- library browser rules;
- product/version compatibility;
- legacy products and paths;
- project recall when content or engine versions change.

### Narrow first Kontakt matrix

The technical lead should later select:

```text
one exact Kontakt build
one exact Native Access build
one exact license/edition
one small NI-owned licensed library
one content root on a declared filesystem
one Bitwig/runner/environment/profile matrix
```

Optional later additions:

- one licensed third-party Player library;
- one lawfully owned unlicensed/full-Kontakt library;
- one external-drive relocation;
- one older project/library version case.

Do not begin with the user's whole Kontakt library collection.

### Native Access installation questions

- Can Native Access install and update under the runner?
- Which Windows services, WebView/browser components, background agents, URL handlers, and elevated operations are required?
- Can application/download/content paths be set to product-managed locations?
- Which operations require internet?
- How are interrupted downloads resumed?
- Can the manager observe status without scraping credentials or proprietary protocols?
- How does Native Access report activation versus installation versus repair?
- Which filesystem semantics do large libraries require?

### Content questions

- What identifies a library independent from path?
- Which metadata files/databases are vendor truth?
- How is a moved library located?
- What changes between internal, external, and removable storage?
- How are missing samples reported inside Kontakt versus Native Access?
- Does project state contain absolute paths or library identifiers?
- Which library updates break older Kontakt versions?
- Which user presets/snapshots need separate backup?

### Authorization and edition questions

- How does Native Access authenticate and persist account state?
- Which non-secret machine identity must remain stable?
- Does Kontakt itself launch Native Access for missing activation?
- How do full Kontakt and Kontakt Player differ for the selected library?
- How are third-party serials added?
- Which unlicensed libraries are loadable only in full Kontakt and absent from the library browser?

### Runtime/bridge questions

- Does Kontakt's module expose multiple classes?
- What are its bus/event/parameter/state characteristics?
- Does it create worker/helper processes?
- How large and frequent are state transfers?
- How does sample streaming interact with Wine, filesystems, page cache, and real-time deadlines?
- Are file dialogs, drag-and-drop, database scanning, and background analysis required?
- Can editor failure remain isolated from sample playback?

### Required acceptance classes

Kontakt must not receive one binary “works” state. Separate:

```text
Native Access launches
account login succeeds
product installs
product authorizes
module scans
Kontakt instance processes audio
selected library installs/locates
selected instrument loads
content streams without deadline failures
state/project recalls
content relocation repairs
update/rollback works
```

### Required negative tests

- Native Access offline;
- invalid/expired session without capturing credentials;
- download path unavailable;
- content root missing or read-only;
- library requires full Kontakt in Player;
- unactivated library;
- old Kontakt cannot load updated library;
- external drive absent at project reopen;
- library relocation interrupted;
- large scan/download cancelled;
- Windows host crash during sample streaming;
- state/project reopen with content missing.

### Claim ceiling

A successful first matrix proves only that exact Kontakt/Native Access/library combination under exact conditions. It does not prove the entire Komplete catalog, every third-party library, legacy Service Center products, or hardware integration.

---

## F4 — Host and deployment fixtures

### H1 — Maintainer Steam Deck + Bitwig Flatpak

First real host fixture. Capture:

- SteamOS/distro and kernel;
- CPU architecture and memory;
- graphical session/X11/XWayland posture;
- Flatpak Bitwig version/runtime/permissions;
- Bitwig plug-in sandbox mode;
- user-path and extension-path visibility;
- local socket/shared-memory/process-launch behavior;
- audio device/PipeWire posture;
- display scale and editor-window behavior;
- power/performance observations where relevant.

The Steam Deck is not the minimum product specification.

### H2 — Ordinary desktop Linux + native Bitwig or another DAW

Later portability fixture separating SteamOS and Flatpak effects from general Linux behavior.

### H3 — Another Flatpak DAW

Later proof that publication and broker abstractions are not Bitwig-manifest hard-coding.

---

## Fixture evidence hygiene

Never retain:

- commercial plug-in or installer binaries;
- license files;
- serial numbers;
- account names/emails;
- cookies/tokens;
- full Wine prefixes;
- paid presets/samples;
- personal project data.

Retain exact digests, versions, redacted manifests, capability censuses, structured outcomes, timing aggregates, and screenshots only where legally and technically appropriate.
