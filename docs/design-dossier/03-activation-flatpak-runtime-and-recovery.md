# 9. Installation, phone-home, and activation realities

Vendor authorization is one of the decisive differences between a hobby bridge and a product.

## 9.1 Capability declaration

Every product/profile should declare, where known:

```text
network required for installer
network required for first launch
network required for every launch
periodic verification interval/class
companion application required
background service required
external browser required
custom URL callback required
localhost callback required
offline challenge/response available
license file import available
hardware token/driver required
number-of-machines behavior known/unknown
machine identity sensitivity
clock sensitivity known/unknown
```

Unknown remains unknown until observed or documented.

## 9.2 Browser and webhook shape

A plug-in may open a system browser, embed a WebView, register a custom URL scheme, launch a local callback listener, or expect a vendor manager to receive the result. The product needs an **authorization broker** that can:

- present the vendor's real window;
- open an allowed external browser URL;
- route a declared callback to the correct environment process;
- preserve the process/session until the vendor confirms success;
- report timeout or unsupported callback type;
- avoid logging query parameters or tokens;
- support offline file selection without copying the file into evidence.

“Webhook” should not be assumed to mean an internet webhook; many desktop OAuth flows use localhost or custom-scheme callbacks. The exact mechanism is fixture evidence.

## 9.3 Recurring verification

A product that periodically checks a license server needs a runtime policy distinct from installation. The manager may keep a companion app available, launch it before the DAW, or warn that the plug-in is approaching its offline window. It must not fake a successful check.

A recurring-check fixture must test:

- fresh online launch;
- expected offline interval;
- expired/offline behavior;
- account logged out;
- companion app closed;
- clock/time-zone changes only where lawful and safe to observe;
- network denied;
- recovery after reconnect.

## 9.4 Machine identity

Wine environments expose Windows-like machine identifiers, registry state, host names, hardware descriptions, and filesystem paths that vendors may bind to authorization. The product must define what remains stable when:

- the runner changes;
- the environment moves to another disk;
- the SteamOS image updates;
- the Linux username changes;
- the application is reinstalled;
- a snapshot is restored;
- the physical machine changes.

The safe default is to preserve environment identity and never promise cross-machine cloning. Any migration assistant must distinguish copying technical state from obtaining vendor authorization.

## 9.5 Background services and drivers

Wine user-space services may be manageable. Windows kernel drivers, USB dongle drivers, low-level anti-tamper, and unsupported device stacks may be blockers. The manager should detect and classify these rather than repeatedly retry an impossible install.

## 9.6 Security boundary

The manager can observe process names, window classes, exit states, declared file roots, and safe registry metadata. It should not inspect encrypted vendor protocols, scrape passwords, inject into licensing processes, intercept TLS, or patch authorization code.

---

# 10. Flatpak and host publication

Bitwig's present Flatpak packaging is unusually promising because it declares Linux audio plug-in extension paths and broad media-oriented permissions. That is an observed current implementation fact, not permanent authority.

## 10.1 Development publication mode

The first proof may publish a native proxy under a user VST3 path, provided the exact Bitwig Flatpak build can see and load it. This minimizes packaging work while proving the DAW-facing ABI.

Required proof:

- exact host path and sandbox view;
- discovery after rescan/restart;
- proxy dependencies visible in sandbox;
- broker/socket/shared-memory availability;
- Windows host launch route;
- no reliance on hidden host escape.

## 10.2 Flatpak extension mode

A production-quality Bitwig integration may package native proxy and supporting runtime pieces as an `org.freedesktop.LinuxAudio.Plugins` extension matching Bitwig's Freedesktop runtime branch.

Potential benefits:

- known Linux ABI/runtime;
- same sandbox namespace as the DAW plug-in host;
- explicit extension lifecycle;
- no system-package mutation;
- SteamOS-friendly distribution.

Costs:

- runtime-branch coupling;
- extension build/release work;
- per-DAW manifest differences;
- restrictions on mutable runner/environment data;
- need to place user-owned Windows state in persistent writable locations;
- upgrades when Bitwig changes its runtime branch.

## 10.3 Broker placement

There are three candidate shapes:

1. proxy and Windows host both execute inside the DAW sandbox;
2. proxy communicates with a narrowly permitted host broker;
3. DAW is installed natively and both run in the user session.

The architecture must not assume these are identical. Cross-sandbox shared memory, sockets, process creation, graphics, and file visibility require exact proofs.

## 10.4 No general host escape

`flatpak-spawn --host` or broad D-Bus access may be useful for experiments, but an unrestricted host escape is not a mature product boundary. A host broker, if selected, exposes only declared operations such as start exact environment host, open exact editor, and report health.

---

# 11. Runtime and bridge experience from the user's perspective

The user should not normally choose between Wine, Wine Staging, Proton Experimental, GE-Proton, DXVK variants, or synchronization patches. The compatibility profile and environment know a tested runner.

Advanced users can see and override with warnings.

## 11.1 Runner lifecycle

```text
built from exact source/submodules
-> license and notices recorded
-> signed/digested
-> capability tested
-> distributed immutable
-> selected by profile/environment
-> superseded without deleting old revision
-> withdrawn if known unsafe
```

## 11.2 Environment lifecycle

```text
created from exact runner family policy
-> installer transaction
-> authorization transaction
-> scanner census
-> host publication
-> working revision retained
-> explicit update or repair transaction
-> rollback or retirement
```

## 11.3 Plug-in instance lifecycle

```text
proxy loaded
-> broker handshake
-> Windows host selected/started
-> module loaded
-> exact class created
-> component/controller connected
-> buses/parameters/state established
-> process activated
-> audio/events processed
-> editor optionally created
-> state saved
-> deactivated and destroyed
-> host cleaned or retained by process-group policy
```

The manager visualizes health outside this callback lifecycle. It never pauses audio to refresh a card.

---

# 12. Failure and recovery constitution

A commercial compatibility product is judged most clearly when something breaks.

## 12.1 Installer failure

Result includes process stage, exit result, discovered partial changes, rollback availability, and safe retry. It does not mark the plug-in installed.

## 12.2 Authorization failure

Result distinguishes login failure, callback failure, unsupported browser/WebView, account/license rejection, network denial, expired recurring authorization, and unknown vendor result. Sensitive payloads are excluded.

## 12.3 Scanner failure

A scanner can crash, hang, exceed resource limits, spawn prohibited children, or return malformed metadata. The module is quarantined with stage evidence; Bitwig is not exposed to it.

## 12.4 Proxy load failure

The proxy may be missing a dependency, incompatible with the DAW ABI, unable to find its mapping, or unable to reach the broker. It returns a bounded error and diagnostic identifier without crashing the host.

## 12.5 Windows host failure

The broker reports runner launch, environment, module-load, class-create, protocol, or process crash separately. Orphan cleanup is mandatory.

## 12.6 Real-time deadline failure

The system records deadline misses outside the callback and follows a declared audio posture: silence, previous safe output where valid, host-requested bypass, or instance failure. It does not wait indefinitely.

## 12.7 Editor failure

The editor may fail while audio continues. The system can close/restart the editor, switch to generic parameters, or mark the editor unsupported without necessarily unloading the processor.

## 12.8 State failure

If get/set state fails, the plug-in is not project-safe. A project reopen must never silently instantiate defaults and claim success.

## 12.9 Content failure

Missing files, moved libraries, permission denial, unsupported external filesystem, or vendor database mismatch are content failures. The product offers locate/repair only through known-safe mechanisms.

## 12.10 Runner regression

The environment retains its last known runner. A new runner is staged and compared; rollback is one action. A global runtime update may not break every plug-in at once.

## 12.11 Environment corruption

External Wine tools or manual edits may change an environment. The manager can detect manifest drift, snapshot before repair, and either adopt observed changes through an explicit transaction or restore/clone. It does not pretend the environment is unchanged.

---
