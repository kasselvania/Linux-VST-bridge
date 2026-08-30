# 6. Canonical product objects

The product needs an explicit model because ad hoc path-based logic is how compatibility systems become unrepairable.

## 6.1 Vendor

A human and machine identity for the publisher, with aliases for discovery. It does not imply one environment or license system.

## 6.2 Product family

Examples: Serum, Kontakt. A family can have multiple major versions, modules, editions, and license channels.

## 6.3 Product build

An exact vendor version and acquisition channel. It may be identified by vendor version plus module digest and installer provenance.

## 6.4 Installed module

An exact Windows VST3 or CLAP module at an exact environment revision. A VST3 module can export multiple classes.

## 6.5 Plug-in class

The format-defined class identity presented to the DAW. This is the key project-recall identity and must not be regenerated from a friendly name.

## 6.6 Managed environment

A versioned Wine-prefix-like state container with declared family policy, registry, Windows files, dependencies, machine identity, authorization posture, and snapshots.

The environment is not a plug-in and not a runner.

## 6.7 Runner

An immutable executable/runtime distribution with exact digest, Wine base, selected Proton/Wine patches, graphics components, synchronization posture, libraries, notices, and capability metadata.

## 6.8 Compatibility profile

Versioned reviewed data that binds exact match conditions to declared runner/environment/graphics/editor/install/content/process policies and a bounded support claim.

## 6.9 Native proxy

The Linux plug-in bundle loaded by the DAW. It has its own build, ABI, protocol compatibility, format metadata, and mapping to an exact plug-in class.

## 6.10 Host registration

A declaration that a proxy is published to a specific DAW installation mode and path/extension surface. A proxy can be registered to multiple hosts without duplicating its underlying Windows installation.

## 6.11 Content root

A user-owned or vendor-managed location for large sample libraries, wavetables, presets, databases, previews, or other content. Content roots have type, ownership, portability, required layout, digest/index posture, and access policy.

## 6.12 Authorization posture

A non-secret classification of the lawful authorization route and continuing requirements. It can say “authorized,” “requires refresh,” or “companion app required” without containing the credential or license payload.

## 6.13 Preset identity

A preset may be:

- vendor factory content;
- vendor user preset;
- DAW preset/device wrapper;
- bridge-managed state snapshot;
- external preset pack.

These are different namespaces and must remain distinguishable.

## 6.14 Project binding

The exact relationship observed when a DAW project instantiates a proxy and saves plug-in state. A project binding retains enough identity to explain why a future reopen succeeded, migrated, or failed.

## 6.15 Diagnostic receipt

A structured, sanitized fact from installation, scan, launch, audio, editor, state, update, or repair. Receipts are evidence inputs, not automatic support claims.

---

# 7. Presets, state, content, and organization

Presets are not one problem. A useful product must separate at least four layers.

## 7.1 Vendor-native preset and content truth

The vendor owns the format, directory structure, database, factory content, and browser semantics. The product may discover and index them when lawful, but must not rewrite or flatten them into a private format.

For Serum 2, the fixture must discover the actual current content and user-preset roots rather than assume historical Serum paths. Wavetables, samples, noises, presets, and configuration may have different portability and backup behavior.

For Kontakt, the instrument/library content is much larger and may be on external storage. Native Access may own application, download, and content path settings. A library's browser visibility and loadability can also depend on license class and Kontakt edition.

## 7.2 DAW-native presets

Bitwig may wrap plug-in state, mappings, device chains, remote controls, and metadata in its own preset form. The bridge should preserve this relationship rather than presenting its own preset system as superior truth.

A fixture must test both:

- the plug-in's own preset browser;
- a Bitwig-saved device preset or project state.

## 7.3 Bridge-managed snapshots

The product may offer named snapshots for support and migration:

```text
state blob
+ exact plug-in class
+ module build
+ environment revision
+ runner/proxy/protocol versions
+ parameter census digest
+ optional screenshot/notes owned by the user
```

A snapshot is opaque unless the format explicitly exposes safe structure. The product must not promise that a state blob from one module build will load in another.

## 7.4 Index and visual metadata

Favorites, tags, collections, notes, ratings, and search terms are user metadata. They can refer to vendor presets without moving or modifying them.

The index should support:

- exact source type;
- vendor/product/version;
- preset name and relative location;
- factory/user/external classification;
- content-root identity;
- last-seen digest/mtime where appropriate;
- whether the plug-in can currently resolve it;
- user tags and favorite state;
- missing/relocated status.

## 7.5 Relocation

Relocation is an explicit transaction:

1. identify the content root and owner;
2. determine whether the vendor supports relocation or requires its own manager;
3. copy/move with verification or invoke the vendor-supported locate flow;
4. update only the owner-authorized path declarations;
5. reopen representative presets/projects;
6. retain rollback or original location until accepted.

The product must never bulk-edit opaque state blobs to replace paths.

## 7.6 Backup and restore

Backup classes differ:

- reproducible runner: redownloadable, digest-pinned;
- product manifests/profiles: product-owned;
- environment state: sensitive and potentially license-bound;
- vendor installers: user/vendor-owned, redistribution restricted;
- plug-in binaries: vendor-owned;
- factory content: vendor-owned;
- user presets: user-owned and important;
- large libraries: user/vendor-owned, often impractical to duplicate;
- authorization state: vendor-owned and machine-bound.

The UI should tell the user what is actually protected. “Environment backed up” must not imply that a vendor will accept a cloned activation on another machine.

---

# 8. Visual representation and interaction design

The manager's visual system exists to make the compatibility chain understandable. It is a projection over canonical state, never a second state machine.

## 8.1 Home view

The home view answers:

```text
Can I make music now?
What needs attention?
What changed recently?
Can I safely update or roll back?
```

Suggested sections:

- **Ready** — published plug-ins with passing last-known health;
- **Needs attention** — authorization, missing content, host publication, runner mismatch, or known regression;
- **Installing/scanning** — explicit foreground transactions;
- **Quarantined** — crash/hang with reason and safe next action;
- **Updates** — plug-in/profile/runner updates separated;
- **Recent recovery** — rollbacks, repaired paths, or recovered projects.

## 8.2 Plug-in card

A card shows human identity first and technical truth on expansion:

```text
Serum 2
Instrument · Xfer Records
Ready in Bitwig
Editor: detached, verified
Authorization: Xfer lifetime, offline-capable
Content: available
Runtime: Audio Runner 0.1
Profile: serum2-xfer-owned / exact revision
```

Expanded view:

- exact module and class IDs;
- installed and scanned versions;
- environment and runner digests;
- host registrations;
- tested sample rates and buffers;
- editor/graphics mode;
- state/preset/project recall status;
- known limitations;
- last diagnostics and evidence claim level.

## 8.3 Environment view

The environment view shows why products share or do not share a prefix-like state container. It includes vendor family, installed products, dependencies, companion apps, machine identity posture, authorization class, snapshots, disk usage, and external modifications.

It must not encourage casual registry editing as the normal workflow.

## 8.4 Chain view

For advanced diagnostics, show the actual route:

```text
Bitwig Flatpak
  -> native Serum 2 proxy
  -> local broker / protocol vN
  -> Windows VST3 host
  -> Serum 2 class
  -> Xfer environment revision
  -> Audio Runner revision
  -> content roots
  -> authorization posture
```

Every node is selectable. The chain view is readback, not editable wiring.

## 8.5 Installation and authorization theater

Installers and activation windows should appear inside a bounded session shell with:

- transaction title;
- current process/application identity;
- network posture;
- environment target;
- cancel/terminate behavior;
- “open browser” or “select offline license file” actions;
- non-sensitive event timeline;
- help text for popups hidden behind other windows;
- explicit completion versus “installer exited but module not found.”

The manager must handle child windows, modal dialogs, browser returns, custom URL schemes, localhost callbacks, WebView content, and vendor-manager relaunches as first-class events. It should never ask the user to hunt through invisible Wine windows.

## 8.6 Editor experience

The first trustworthy editor mode is a supervised detached top-level window:

- opens adjacent to the DAW;
- follows the correct plug-in instance;
- restores size/scale where appropriate;
- receives keyboard and mouse focus correctly;
- supports plugin-created popups and menus;
- does not disappear behind the DAW;
- closes without killing audio;
- can be force-closed without corrupting the environment;
- reports unsupported embedding honestly.

Later modes may include X11/XWayland embedding or captured/streamed native presentation. These are separate capabilities, not automatic upgrades.

## 8.7 Generic control fallback

Where the editor is unavailable but parameters are valid, the product may permit the DAW's generic parameter view. This is degraded functionality, not “fully working.” The card must say so.

## 8.8 Accessibility and small screens

The Steam Deck fixture makes small-screen and touch usability visible, but the manager must also support ordinary desktops. Critical actions should not depend on hover, tiny controls, or one fixed resolution. Advanced technical detail can use progressive disclosure.

---
