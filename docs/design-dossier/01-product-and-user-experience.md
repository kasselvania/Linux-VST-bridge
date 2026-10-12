# Linux Plugin Bridge product model

**Design draft for operator review — 11 October 2026.**

The product lets someone bring Windows audio plug-ins into their native Linux
DAWs, use them in music, and maintain them without administering compatibility
infrastructure. Its design must cover the lifetime of the product and the user's
collection, including work performed in vendor applications and DAWs.

The operator has paused manager implementation to establish this model. This
replaces the earlier interaction-design seed in this file. Existing screens,
labels and implementation do not determine the proposed flows. The draft does
not approve new scope, change beta acceptance, or claim these behaviors exist.
Established constraints below remain binding; proposed behavior and the decisions
at the end require resolution before dependent implementation.

Read the model by subject:

- [Product outcome and scope](#product-outcome-and-scope)
- [People and the things they manage](#people-and-the-things-they-manage)
- [Task inventory](#task-inventory)
- [What is common and what varies](#what-is-common-and-what-varies)
- [Control, observation and authority](#control-observation-and-authority)
- [Facts and user-visible status](#facts-and-user-visible-status)
- [Flow structure](#flow-structure)
- [Operating the product over its lifetime](#operating-the-product-over-its-lifetime)
- [Interaction requirements](#interaction-requirements-derived-from-the-model)
- [Decisions for the operator](#decisions-that-remain-with-the-operator)
- [Scenarios for testing the model](#scenarios-for-testing-the-model)
- [From agreed model to implementation](#from-agreed-model-to-implementation)

## Product outcome and scope

A user can answer four questions at any point: What do I have? What can I do
with it? Does anything require my attention? What happens if I make this change?
Ordinary playback continues with the manager window closed. Returning to the
manager resumes from the current situation, including already working plug-ins.

The current product lane is Windows VST3 plug-ins used through native Linux
DAWs. Instruments and effects are equal requirements. The previously selected
[Windows DAW lane](../WINDOWS_DAW_WORKSPACES.md) and ARM work remain later work;
this draft does not cancel them. Other formats, migration from other bridges,
cloud services, product accounts and billing need explicit scope decisions.
There is no new requirement for our own account or subscription in this design.

The [working rules](../../AGENTS.md), [architecture](../ARCHITECTURE.md), and
[integrated beta contract](../INTEGRATED_BETA_DELIVERY.md) retain these constraints:

- The DAW loads a native proxy. The product supplies its own runtime and manages
  the Windows host; a customer compiler, Steam or separately installed Wine is
  not a prerequisite.
- Unfamiliar plug-ins are attemptable within implemented capabilities. Tested
  support, a local success, and permission to attempt are distinct.
- Exact identities, user projects, content, presets and licensed environment
  identity survive supported changes. A matching name never authorizes substitution.
- Vendor authorization belongs to the vendor and user. Credentials and licensing
  material stay out of diagnostics and repository records.
- Saved state, automation, reboot, updates and recovery are part of usable-product
  acceptance. Discovery or installation alone cannot establish that acceptance.
- The manager and editor are absent from the real-time audio dependency path.
  Failed editors, installers and audio processing retain distinct causes.

## People and the things they manage

| Person or organization | Goal and responsibility |
| --- | --- |
| Musician | Acquire software they are entitled to use, make music, preserve projects, and make consequential choices. Experience and accessibility needs vary. |
| Advanced user | Inspect effective settings, try supported alternatives and restore previous choices without a separate product or terminal-only workflow. |
| Maintainer and support | Deliver compatible releases, explain failures, improve general mechanisms and help a user resume their task. They are not a required step in ordinary installation. |
| Plug-in vendor | Supply software, content, licensing and vendor-controlled installation or account services. |
| DAW vendor | Own DAW discovery, projects, routing, automation and host behavior. |
| Platform provider | Own operating-system policy, device drivers, sandbox facilities and distribution infrastructure. |

A person may have several roles. The ordinary product must work without assuming
that the musician is also its maintainer.

| Concept | Meaning and relationship |
| --- | --- |
| Vendor product | What someone obtains from a vendor. One product may contain several instruments, effects, editions or formats. |
| Plug-in | One actual instrument or effect class available to a DAW, bound to an exact installed build. This is not interchangeable with a vendor product or installer. |
| Installation | The managed files and dependencies resulting from vendor operations. A past installation attempt and the current installed software are separate facts. |
| Vendor application | A launcher, installer, updater or authorization application serving one or more products. It remains reachable after first installation. |
| Managed environment | The product-owned Windows compatibility context, potentially shared by related products and vendor services. It is normally an implementation detail; shared consequences are visible. |
| DAW destination | A particular Linux DAW installation and its access to plug-ins. Several DAWs may share a publication path; per-DAW visibility cannot be promised without an implemented mechanism. |
| Content location | Samples, libraries and presets with explicit ownership. These can be large, external, missing or shared independently of installation files. |
| Current configuration | The exact runtime, plug-in build, bridge components and applicable settings selected for use. Proposed changes remain separate until applied. |
| Operation | A bounded activity with a target, progress, result and recovery route. Closing its window does not erase ownership or invent completion. |
| User project | DAW-owned music and references to plug-ins/content. The manager does not assume it knows every project or may rewrite one. |

## Task inventory

**Required** means necessary to the ordinary outcome, not necessarily a user
click. **Conditional** means required when the stated condition occurs.
**Optional** means a user-selected preference or investigation. All rows are
needs the design must account for; listing one is not a delivery commitment.
Existing beta obligations remain in the beta contract. Unselected scope is
identified in the decision section rather than silently removed.

### Evaluate and acquire the product

| Task | Kind and trigger | Actor and completion |
| --- | --- | --- |
| Understand the product and its limits | Required before adoption | Product explains the native Linux DAW lane, vendor ownership, requirements and current limitations; user can decide whether to proceed. |
| Check a machine, DAW and desired plug-in | Required for a useful attempt | Product reports known support, untested combinations and specific missing capabilities separately. Unknown names do not create a refusal. |
| Obtain the appropriate release | Required on a new machine | Product supplies the correct package and required notices; integrity and platform requirements are checked. |
| Install and launch the manager | Required on a new machine | User starts installation; product completes its own setup or identifies the failed step with retry/removal available. |
| Choose product and content storage | Conditional on capacity, external drives or preference | Product proposes locations, shows download/installed space and temporary-space requirements where known, checks access, and lets the user choose before large writes. |
| Obtain compatibility components | Required when absent | Product acquires and verifies its own runtime; user intervenes only for a meaningful choice, permission or failure. |
| Work without a current network connection | Conditional | Product distinguishes cached executable capability from vendor/download connectivity needs; available local work stays accessible. |

### Bring software and content into use

| Task | Kind and trigger | Actor and completion |
| --- | --- | --- |
| Add through a standalone installer | Conditional acquisition path | User selects a lawful installer; product supervises it and reconciles actual results. |
| Add through a vendor application | Conditional acquisition path | Product opens the correct managed application; user selects products there; product recognizes resulting installations. |
| Add another product through an existing vendor application | Conditional returning user | Product reuses the correct environment and explains effects on existing siblings; no unnecessary new identity or login is created. |
| Encounter software installed outside this product | Conditional existing collection | Product identifies the boundary without taking ownership. Adoption, copying and coexistence need the migration decision below. |
| Choose editions, components or formats | Conditional vendor choice | User selects meaningful options; product explains which installed formats can be used and retains unsupported findings as such. |
| Sign in, accept terms or authorize | Conditional vendor requirement | User completes the vendor flow; product presents/focuses it and handles lawful return. Only an authoritative result supports an authorization claim. |
| Handle trials, subscriptions and activation limits | Conditional vendor restriction | Product explains the observed restriction and offers the vendor route. A save-disabled demo cannot qualify state recall. |
| Install a companion application or dependency | Conditional demonstrated requirement | Product identifies why it is needed and which products are affected; supervised installation returns to the interrupted task. |
| Download or locate sample libraries | Conditional content requirement | Vendor software and user choose lawful content; product tracks declared locations and availability without claiming that an existing directory is valid content. |
| Import user presets or additional sound content | Conditional user collection | User chooses legitimate files; vendor-supported import/location rules determine installation. Product preserves provenance and ownership without rewriting proprietary formats. |
| Discover instruments and effects | Required after relevant changes | Product identifies actual modules/classes and separately reports failures; unrelated successful discoveries remain useful. |
| Prepare the chosen plug-ins for DAW use | Required before first use or changed configuration | Product chooses a plausible supported configuration and checks required capabilities without asking the user to administer a prefix. |
| Make plug-ins accessible to intended DAWs | Required before use | Product verifies its own publication and filesystem access; DAW indexing remains a separate fact. |
| Rescan, enable or locate a plug-in in a DAW | Conditional DAW state | DAW-specific guidance or a supported integration reaches the browser entry; the user does not receive REAPER instructions when using Bitwig. |

### Use and organize a collection

| Task | Kind and trigger | Actor and completion |
| --- | --- | --- |
| Add an instrument or effect to a project | Required for musical use | DAW owns the track and routing; product launches the correct build and contains host failure. |
| Open, resize and close the editor | Conditional editing need | Product coordinates the vendor editor; closing it leaves safe processing intact. Accessibility limitations are stated where relevant. |
| Browse, select and save user presets | Conditional sound-design task | Vendor editor and DAW retain their preset functions; product preserves declared locations and project recall without making a new preset browser a prerequisite. |
| Play, record, automate and render | Required supported musical operations | DAW owns the operation; bridge preserves supported audio/event/state behavior, including offline processing within declared capabilities. |
| Use multiple instances or DAWs | Conditional workload | Product enforces actual capacity and ownership boundaries and preserves independent instance state. |
| Save, close, reopen and reboot | Required usable-product outcome | DAW saves the project; exact sound, state and content recall are verified in qualification. Local observations and user reports retain their scope. |
| Find and organize the collection | Optional organization | Names, vendor, instrument/effect, search and later organizational choices help retrieval without changing identity or moving files. |
| Inspect advanced compatibility settings | Optional investigation | Product shows current effective values, source, scope and differences from defaults. |
| Try and retain an alternative configuration | Conditional problem or optional preference | Product offers a bounded comparison with affected products and a truthful keep/restore path. |

### Maintain, recover and leave

| Task | Kind and trigger | Actor and completion |
| --- | --- | --- |
| Update the manager and its bridge components | Conditional release choice | Product applies a coherent generation and preserves exact working relationships and a supported predecessor route. |
| Update a runtime or compatibility recommendation | Conditional release/problem | Product shows scope, respects local overrides and checks affected users of the environment. |
| Update a plug-in, vendor application or library | Conditional vendor release | Vendor owns its mutation; product explains consequences, rescans and states actual restoration limits before proceeding. |
| Recover after an interrupted operation or reboot | Conditional interruption | Product reconciles owned work and offers resume, retry, stop or retained use according to facts; it does not restart every workflow from scratch. |
| Recover a missing or changed plug-in | Conditional missing file, external change or old project | Product identifies the exact gap and offers managed recovery without substituting a similarly named build. |
| Resolve editor, audio, launch or content failure | Conditional observed problem | Product identifies the affected capability and offers a relevant next action; successful unrelated work remains available. |
| Change DAWs, OS setup or hardware | Conditional environment change | Product refreshes capabilities and DAW access without silently recreating licensed identity. |
| Move product files or content to another drive | Conditional storage change | Product checks ownership, space, mount/access and vendor relocation constraints; movement must preserve references or explicitly explain the supported recovery. |
| Back up, restore or move to another machine | Conditional user need | Product explains what is recoverable and which vendor steps may recur; it never assumes activation is portable. Exact support scope remains a decision. |
| Stop offering a plug-in to DAWs | Optional collection change | Product explains the visibility scope and project consequences; this is distinct from uninstalling vendor software. |
| Uninstall one vendor product | Optional removal | User sees affected siblings, projects whose use is known, content ownership and vendor deactivation options before the vendor operation. |
| Remove the manager or reclaim storage | Optional exit/cleanup | Product explains effects on bridged projects and separates caches, retained software, user data and licensed installations; uncertain ownership is preserved. |
| Ask for help and share diagnostics | Conditional unresolved issue | Product explains the failed task and lets the user inspect an allow-listed export; sharing is deliberate and support returns an actionable response. |

## What is common and what varies

The common structure is intent, current facts, automatic product work, an external
handoff if needed, an observable result, and a route back after interruption.
The following dimensions change steps within that structure.

| Dimension | Flow consequence |
| --- | --- |
| New installation, existing managed collection, or external collection | Different entry points and ownership. A working collection must not be sent through onboarding again. |
| DAW and native/sandboxed installation | Different access and scan instructions. DAW identity is selected or confirmed when ambiguous, not inferred from the first reference fixture. |
| Distribution, architecture, graphics, audio hardware and input methods | Different required capabilities and accessibility needs. Unsupported capabilities must be named. |
| Vendor hub, standalone installer, offline media or companion service | Different acquisition, process lifetime and authorization handoffs. A vendor window may remain open after installing several products. |
| Ownership and license channel | Activation, recurring checks, trial limits and offline options. Vendor name alone cannot determine these. |
| Instrument/effect, classes, editions and supported formats | Different track use and discovery choices; one installer is not one plug-in. |
| Small installation versus large or external content | Different capacity, progress, location and reconnect behavior. |
| Shared environment and dependencies | One change may affect several products; the scope must be visible before mutation. |
| Existing projects and prior builds | Compatibility and retention needs; the manager may not know every project affected. |
| Online/offline, interrupted sessions and limited resources | Different available operations and recovery; no fabricated progress or successful completion. |
| Accessibility and experience | Keyboard/focus, scaling, assistive technology and help depth; advanced capability never becomes a mandatory lesson. |

The model uses these conditions rather than a separate wizard for each vendor.
Support research must include combinations that cross them, not only the simplest
happy path on the maintainer's machine.

## Control, observation and authority

| Area | What our product controls | What it can observe or assist | External boundary |
| --- | --- | --- | --- |
| Runtime and bridge | Acquisition, exact selection, supervision, publication and its supported recovery | Capability checks and bounded failures | OS/kernel policy and actual hardware capability |
| Vendor installation | Launch context, owned operation lifetime and reconciliation | Discovered files/classes and supported installer results | Vendor choices, mutations and install semantics |
| Authorization | Presentation and lawful handoff; stable managed environment identity | Vendor-reported outcomes when available; user-reported experience | Entitlement, terms, activation limits and vendor service availability |
| DAW access and use | Native proxy placement and supported access integration | Proxy load and processing observations; DAW scan/use through supported integration | DAW indexing, track decisions, project writes and host-specific behavior |
| Content and projects | Declared mappings and product-owned metadata | Missing roots, observed use and explicit user reports | Vendor content validity and user/DAW-owned project data |
| Performance | Bridge behavior and supported compatibility settings | Whole-call timing and observed audio/editor behavior | Arbitrary workloads, other applications, drivers and external hardware |
| Updates and support | Product releases, notices, recommendations and support response | Changed components and resulting failures | Vendor release availability, downgrade rights and third-party policies |

A missing integration is a capability gap, not a permanent law of nature. Vendor
licensing authority is a permanent external boundary. User consent does not make
an unimplemented recovery operation possible. Product responsibility includes a
clear handoff and useful return path even when the action belongs elsewhere.

## Facts and user-visible status

The product needs independent facts rather than one overloaded Complete flag.

| Fact | What it establishes | What it does not establish |
| --- | --- | --- |
| Installer attempt completed, failed or unknown | What was observed of that operation | Whether every intended plug-in or library is installed |
| Exact plug-in discovered | A module/class exists in the observed installation | Authorization, processing or recall |
| Configuration prepared and publication current | Product-owned preparation and access are valid to the stated depth | That a DAW indexed it or all launch checks have passed |
| Loaded or processed in a DAW | That exact observed use | Audible correctness or a saved/recalled project |
| Editor opened or failed | That editor result | A universal audio or installation result |
| Vendor authorization result | The vendor's result for that context and time | Future entitlement or portability to another machine |
| User reports successful use and recall | The user's experience with the identified context | An automated observation or published qualification |
| Release qualification | The declared tested combination and operations | Every build, DAW, machine or future configuration |
| History or ownership gap | A specific fact or authority is absent | Permission to invent history or assume all current functions are broken |

Facts have a source, relevant configuration and time. Changes can make old facts
historical; they remain readable but cannot authorize new execution. Displaying
status must not launch expensive scans or mutate the installation. Launch and
mutation owners still perform their required verification.

Proposed presentation rules:

- Show the useful conclusion and next meaningful action. Technical detail is
  available when wanted, rather than being the ordinary task vocabulary.
- Separate an action requiring attention now from optional validation, an update
  opportunity and historical detail. Each interruption explains the consequence.
- A disabled action says why, what would enable it and whether it matters to the
  user's goal. A satisfied prerequisite can simply appear satisfied.
- A failure in one installed class does not erase successful sibling findings.
  Shared-state uncertainty still blocks the particular unsafe operation.
- User confirmation can add a clearly attributed report. It cannot invent a
  successful installer exit, authorization, ownership or safe update path.
- Missing history becomes a user task only when there is a concrete consequence
  and a meaningful action. An independently valid current route can remain usable.
- Local success does not automatically change published support. Conversely,
  absent published support does not forbid an otherwise valid attempt.

## Flow structure

These are proposed product flows. They specify behavior without choosing screens,
navigation or a frontend toolkit. First-install automation is subject to the
explicit decision below; existing execution safeguards remain intact.

Shared-environment maintenance and music use need an explicit handoff. Observation
of changed files does not authorize concurrent scanning, preparation or playback.
Where architecture requires exclusion, vendor maintenance must complete or its
owned activity must be safely retired before the conflicting operation begins,
with fresh ownership checks. An idle or closed vendor window proves neither.
Returning to music uses that supported handoff; it does not arbitrarily kill
licensing services needed during playback. Reopening maintenance while affected
plug-ins are active names the instances that must stop, or defers maintenance.
Concurrent vendor mutation and sibling DSP are not a proposed new capability.

~~~mermaid
flowchart TD
    Intent[User arrives with a goal] --> Entry{Current situation}
    Entry -->|New product installation| Product[Install manager and acquire prerequisites]
    Entry -->|Add software| Add[Select installer or existing vendor application]
    Entry -->|Working collection| Use[Use in the DAW]
    Entry -->|Problem or interrupted work| Recover[Explain and recover the affected operation]
    Entry -->|Change or remove| Change[Show scope and consequences]
    Product --> Add
    Add --> Vendor[Vendor install and authorization when required]
    Vendor --> Reconcile[Discover and assess actual results]
    Reconcile -->|Required capabilities available| Available[Make selected plug-ins available]
    Reconcile -->|Specific gap| Recover
    Available --> Use
    Use -->|Return for maintenance| Change
    Change -->|Supported operation chosen| Reconcile
    Change -->|Removal chosen| Remove[Scoped removal and remaining-use explanation]
    Recover -->|Working route retained| Use
    Recover -->|Retry appropriate| Reconcile
    Recover -->|Vendor action required| Vendor
    Recover -->|No supported remedy| Support[Explain limitation and offer help or exit]
~~~

This is a map of destinations, not a mandatory linear wizard. Cancellation is
available wherever the underlying operation permits it. After any restart, the
entry point is derived from actual current work and ownership.

### Evaluate, install and first launch

**Entry:** A new user with a Linux machine and a desired DAW/plug-in combination.

**Sequence:** Explain the product and current requirements; select the correct
package; install and launch; identify DAWs and storage; clarify ambiguous choices;
acquire missing product components. Show network, capacity or permission work
only when it requires attention. No DAW installed is a valid waiting situation.

**Completion:** The manager can begin a supported installation attempt and clearly
shows any specific unmet prerequisite. It does not claim plug-ins work yet.

**Interruption or failure:** Preserve completed acquisition and owned work. Retry
only the affected step; offer a storage change, network retry, appropriate package
or removal as applicable. Cancelling a download does not remove existing products.

### First plug-in through a standalone installer

**Entry:** User chooses to add software and selects a lawful installer.

**Sequence:** Identify relevant capabilities and intended scope; present storage
and any consequential choices; create/reuse the appropriate managed environment;
run the vendor window; handle authorization if needed; discover actual installed
classes; assess and prepare selected supported classes; make them available under
the selected automation policy; hand off to the user's DAW.

**Completion:** Product-owned access is established, the installed classes are
identifiable, and the user knows where to find them. Successful musical use and
recall remain separate outcomes in the next flow.

**Interruption or failure:** A closed installer, nonzero exit, ambiguous result or
partial installation triggers reconciliation, not an invented success or blanket
failure. Preserve discovered successes. Explain missing requirements and offer
return to the vendor application or a scoped retry. If ownership remains unsafe,
stop only operations that depend on it and name the blocked consequence.

### Several products through a vendor application

**Entry:** User opens a new or existing managed vendor application such as ASC or
Native Access, intending to install or maintain one or more products.

**Sequence:** Bind the application to the correct existing environment; identify
shared effects before vendor mutation; allow vendor product selection, login and
downloads; reconcile resulting changes at safe observation points; show results
for individual instruments/effects; prepare selected classes and hand off to DAW
use. Adding a sibling must not silently create a new activation identity.

**Completion:** Each requested product has an independent useful result. The
vendor app can remain installed and reachable for future work. A hub remaining
open is not proof that installation is incomplete or safe to interrupt.
Availability for music follows the maintenance-to-use handoff above, not merely
discovery of a new file while the hub is still running.

**Interruption or failure:** Retain owned process state, partial downloads where
supported, and unaffected siblings. Distinguish a running vendor task from an
unresponsive process. Never imply generic Stop reverses vendor changes.

### Vendor authorization and return

**Entry:** Installation, first launch or a later use requires vendor action.

**Sequence:** Explain which vendor application or browser is involved; open/focus
that context; user supplies credentials and choices there; support the declared
return path; reconcile the resulting capability without collecting secrets.
Authorization may occur before installation, within it or on first plug-in use.

**Completion:** An authoritative vendor result is shown when available. Otherwise
state that the user returned or reported success and allow the legitimate next
capability check; do not fabricate an activated state.

**Interruption or failure:** Preserve the environment. Offer the vendor's retry,
offline alternative where supported, or account-management route. Activation
limits require an explicit user choice about existing devices. Trial restrictions
remain visible, including saving being unavailable.

### DAW discovery and ordinary music work

**Entry:** A plug-in has a valid product-owned route, or the user already uses it.

**Sequence:** Present the correct DAW and plug-in name; explain a rescan only if
needed; user adds it to a track, selects or saves presets, edits, plays/processes,
automates and saves within the DAW. Vendor-supported preset import and content
locations remain intact. The bridge launches and supervises its host automatically. Reopening the
project and rebooting reuse the selected exact configuration. The manager can be
closed; its UI is not an audio dependency.

**Completion:** The user's musical task succeeds. Product observations and any
user report are recorded only to their actual depth; ordinary use is not gated on
a compulsory manual qualification checklist.

**Interruption or failure:** Distinguish not found in DAW, host launch, authorization,
content, editor, processing and recall failures. A DAW crash or lost host must not
cause an endless wait. Preserve recoverable state and explain what the user must
retry; do not claim an unsaved project can be restored by the manager.

### Return to a working installation

**Entry:** User returns to inspect Kontakt after using it successfully in Bitwig,
or opens the manager after a reboot, upgrade or long absence.

**Sequence:** Read bounded current facts; show the collection and relevant current
actions; expose vendor applications, settings and maintenance in context. A valid
working installation needs no new completion ceremony. Optional first-use guidance
can be dismissed without rewriting factual status.

**Completion:** The user can understand what is available and whether a real
current issue requires attention, then leave and continue working.

**Interruption or failure:** Historical gaps remain inspectable. If one blocks a
requested update, explain that update-specific gap at the point it matters and
offer a valid recovery path. Do not treat a user report as authority to repair
missing lineage. Independently valid present-use facts remain separate.

### Try an unfamiliar plug-in or alternative settings

**Entry:** A supported installation attempt encounters an untested combination,
a specific capability failure, or a user asks to improve behavior.

**Sequence:** Offer a plausible initial setup from machine capabilities, actual
plug-in requirements and applicable advice; explain a failed stage; show relevant
supported alternatives and scope; retain the working predecessor where possible;
change one relevant variable or a declared dependent group; compare; keep or
restore through product controls. Respect explicit local overrides.

**Completion:** A usable local result or a specific unresolved capability is
understood. A local success is not silently promoted to universal support.

**Interruption or failure:** Stop affected processing before unsafe changes. Keep
unrelated work intact. Returning to a previous runtime does not undo vendor-state
mutations; state that limit before the experiment. Missing adapters and unsupported
operations are named gaps, not invitations to edit arbitrary prefix internals.

### Update software

**Entry:** User chooses an available manager, runtime, plug-in, vendor application
or content update, or an external change is detected.

**Sequence:** Identify the component and every known affected sibling; explain
what is retained, what can be restored and what must stop; secure the supported
predecessor route; perform the appropriate product or vendor operation; recheck
changed facts; present the selected configuration and any remaining DAW action.
A newer available version does not replace the current selection automatically.

**Completion:** A coherent selected configuration is available and the user can
continue existing work. Release qualification separately verifies retained-project
recall through update and restoration; a successful update job alone is insufficient.

**Interruption or failure:** Resume or restore only what the product actually
retains. A bridge rollback is not a vendor downgrade or an authorization rollback.
If vendor mutation cannot be reversed, explain that before launch. Never silently
substitute a different build for an old project's reference.

### Recover interrupted work or a broken capability

**Entry:** Product startup finds unfinished owned work, or the user encounters a
failure during installation, scanning, editor use, audio or maintenance.

**Sequence:** Establish the current operation, target and ownership; show what
completed and what remains; choose the narrowest supported retry, continuation,
restoration or vendor handoff; verify its actual result and return to the original
task. Resume user intent as well as technical ownership.

**Completion:** The affected task can continue, the retained working route remains
available, or a specific unresolved limitation has a support/exit route.

**Interruption or failure:** Unknown ownership is not erased to manufacture a
clean slate. Repeated unsuccessful retries produce a different recommendation or
support route. Stale buttons cannot authorize a mutation after the state changed.

### Storage, missing content and moving machines

**Entry:** Storage is low, a removable drive is absent, content moved, or a user
wants another drive or machine.

**Sequence:** Distinguish caches, product components, vendor installations and
user content; show sizes and ownership; reconnect an existing root when possible;
use only supported vendor-aware relocation; verify mounts, access and references;
retain the source until the supported validation permits cleanup. A machine move
must explicitly address vendor activation and exact software availability.

**Completion:** Required data is accessible at its intended location and the
relevant existing project recalls, or the product explains which relocation or
migration step is not supported. Capacity recovery reports the space actually freed.

**Interruption or failure:** No writes fall through an absent mount to another
drive. Do not remove an uncertain source or overwrite a destination. Backup and
cross-machine migration scope is unresolved below; this flow does not promise
portable activations, whole-prefix cloning or arbitrary project rewriting.

### Change DAWs or encounter an external collection

**Entry:** A second DAW is installed, access permissions change, or an installation
managed elsewhere is found.

**Sequence:** For DAWs, refresh destinations/access and explain actual shared-path
visibility; offer host-specific scanning guidance. For external software, show its
source and avoid ownership claims or duplicate-name substitution. Preserve the
existing installation while the user chooses a supported path.

**Completion:** The user knows the scope of DAW access and which installation the
product controls. An external collection is unchanged unless an explicitly
supported adoption operation is selected.

**Interruption or failure:** Missing permissions and unsupported adoption are
concrete capability gaps. Do not rebuild an external prefix, seize another
bridge's publications or silently repoint existing projects. The adoption policy
must be resolved before implementing that branch.

### Remove software or leave the product

**Entry:** User wants a cleaner DAW list, less disk usage, one product removed or
the manager uninstalled.

**Sequence:** Distinguish visibility, vendor uninstall, vendor deactivation,
content removal and product removal; preview exact affected ownership and shared
dependencies; explain known project consequences and unknown project use; require
the appropriate choice before destructive action; run the scoped operation;
reconcile what remains. Deactivation, if chosen, belongs to the vendor.

**Completion:** The intended material or visibility is removed, unrelated products
and retained user data survive, and the user understands how old projects are
affected. Removing a UI shortcut is not falsely reported as complete removal.

**Interruption or failure:** Preserve uncertain ownership and remaining data;
report partial removal and a scoped continuation. Never promise recovery of an
uninstalled proprietary build without a legitimate retained or obtainable source.

### Get help and resume work

**Entry:** A failure cannot be resolved through supported self-service actions.

**Sequence:** Explain the original goal, failed stage, affected product and
relevant non-sensitive configuration; offer an inspectable allow-listed export;
user deliberately shares it through the selected support channel; support supplies
a specific explanation or operation; user returns to the original task.

**Completion:** Resolution is checked against that task, or the limitation and
available alternatives are clear. Closing a support ticket is not a repaired
plug-in. Contact channel and response expectations remain an operator decision.

**Interruption or failure:** No automatic upload of project audio, credentials,
licences, proprietary binaries or whole environments. Offline export remains
possible where the implementation supports it; no backend is assumed.

## Operating the product over its lifetime

These are responsibilities of the product team, not extra setup tasks for the
musician. Distribution and support mechanisms must exist behind the corresponding
user promises; their implementation is not selected by this draft.

| Team responsibility | Trigger and sequence | Useful outcome |
| --- | --- | --- |
| Define a release | Select scope and exact supported combinations, prepare coherent packages, satisfy distribution obligations, exercise complete use/update/recovery flows | A user can obtain the release represented by its claims. |
| Distribute and onboard | Explain requirements and known limits before acquisition; provide install/removal guidance and accessible first-use help | A new user can enter without a maintainer preparing the machine. |
| Maintain compatibility knowledge | Classify observed failures and local successes; verify proposed general fixes; publish bounded advice with version scope | Future attempts improve without a per-product whitelist or secret maintainer procedure. |
| Roll out an update | Publish accurate changes and affected scope; retain supported predecessor paths; avoid surprising mutations during musical work | Users understand the choice and can recover within declared limits. |
| Handle a bad release | Identify affected versions, stop recommending the faulty version, communicate an action, deliver repair or a valid predecessor | Existing users have a specific recovery route; withdrawing a recommendation does not itself delete their software. |
| Support a user | Receive a deliberate report, identify the failed task, offer a safe next action, verify return to use | Support resolves the musical/product task rather than merely collecting logs. |
| Maintain privacy and retention | Define what local/remote information exists, why it is needed, consent and deletion behavior | Users can understand and control diagnostic sharing; no unselected telemetry service is introduced. |
| End a version or feature | Explain changes and transition options before withdrawing support; preserve access to user-owned work within declared retention obligations | Users are not silently stranded by changed policy. |

The product lifecycle is define, deliver, onboard, operate, maintain, recover and
eventually retire. The plug-in lifecycle runs inside it and also crosses vendor
and DAW boundaries. Updating or removing our product therefore needs an explicit
answer for every surviving plug-in, environment and project dependency.

## Interaction requirements derived from the model

Navigation is still undecided. These requirements constrain whatever interface
is chosen:

- A user can start from add, current collection, ongoing work, a specific problem
  or maintenance. One entry point must not force unrelated setup steps.
- Plug-ins, vendor applications and shared effects are understandable together.
  The user can return to ASC or Native Access without pretending to add it anew.
- The ordinary view answers the four product questions at the start of this
  model. History and advanced configuration remain available in context.
- Each offered action has a clear target, purpose and consequence. Automatic work
  has a result; external work has a return path; an unavailable action has a reason.
- Long operations acknowledge the request promptly and show their actual phase,
  affected software and whether stopping is possible. Unknown download size or
  progress is presented as unknown, not a manufactured percentage.
- Closing a window, cancelling a request, stopping an owned process, undoing a
  vendor change and restoring product software are different actions. Their
  effects must be stated and supported individually.
- Keyboard navigation, focus return, readable scaling and assistive access are
  part of ordinary flows, including vendor handoffs and recovery. A compatibility
  option that disables Windows accessibility must disclose that consequence.
- Troubleshooting stays attached to the user's original task. Technical details
  explain a problem when useful; they do not become the primary collection model.

## Decisions that remain with the operator

The approved request is to establish this model before implementation. The
following recommendations make the unresolved choices concrete; they do not
silently settle terminology, scope or authority.

| Decision | Recommendation and alternative | Consequence |
| --- | --- | --- |
| First-install automation | Recommend one initial intent to install and make chosen plug-ins available; automate discovery and safe preparation, asking only for meaningful choices or external actions. Alternative: a separate final review before first publication. | Architecture 18 currently describes explicit preparation/publication steps. Combining the interaction requires an agreed authority model and an architecture amendment before code. Replacing a working configuration still requires its own consequence review. |
| External installations | Recommend first establishing reliable coexistence and an explanation of supported vendor reinstall paths; assess adoption separately. Alternative: make adoption a required beta journey. | The old design seed listed module import without a complete ownership/licensing flow. Beta inclusion or exclusion must be decided explicitly; no existing installation is taken over now. |
| Storage and portability scope | Recommend specifying supported content/product relocation and its limits, then deciding whether full backup and cross-machine transfer are beta obligations. | A clean relocation and a licensed machine migration are different capabilities. No blanket snapshot or activation-portability promise is made. Existing preservation requirements remain. |
| Recording successful use | Recommend optional, clearly attributed user feedback plus available automatic observations. Alternative: provide guidance without storing user confirmation. | Neither creates a compulsory completion checklist or lets a user report authorize unsafe operations. Formal support qualification remains separate. |
| Support and release operations | Recommend using the existing release/support infrastructure with a stated contact route and response expectations; add account services only for a demonstrated need. | The operator must choose the actual support channel, diagnostic retention and release-notification policy before those are promised to testers. |

DAW selection and per-DAW visibility also need a capability check during design:
selection can personalize guidance even when several DAWs share one publication
location. The UI must not promise selective hiding that the product cannot enforce.

## Scenarios for testing the model

These are planned design walkthroughs and later usability cases, not recorded
passes. The starting facts are deliberately varied. A prototype must allow the
person to act and explain the outcome without the designer narrating it.

| Situation | Flows exercised | What the person must be able to understand or do |
| --- | --- | --- |
| Kontakt already plays and recalls in Bitwig, as reported by the operator | Return, ordinary use, facts/status | Recognize the working local installation and any concrete maintenance limitation without being sent through installation again. |
| First beta user starts with REAPER and a lawful installer | Evaluate, first install, DAW use | Install, find the plug-in, make music and return later without maintainer setup or Bitwig-specific assumptions. |
| ASC already contains FRAGMENTS; another Arturia product is added | Vendor application, authorization, discovery | Reuse the existing context, see sibling effects and obtain separate outcomes for installed products. |
| One installer yields an instrument, an effect and an unsupported format | Install, preparation, DAW access | Understand which classes are available and why one format cannot be used; successful siblings stay visible. |
| Unknown plug-in meets all implemented requirements | Unfamiliar attempt, ordinary use | Attempt it without a catalogue entry and understand that local success does not equal a broad support claim. |
| A trial plays but refuses saving, or activation slots are exhausted | Authorization, ordinary use | Recognize the vendor limitation and the legitimate action; avoid a false recall pass or automatic deactivation elsewhere. |
| Vendor hub remains open after installing two products | Vendor application, interruption | Distinguish ongoing application lifetime from individual installation results and know how to return to music. |
| Network fails or a reboot occurs during acquisition/installation | First launch, vendor application, recovery | Resume from actual partial work and understand whether a vendor process must still finish. |
| DAW access is blocked by sandbox permissions | DAW access, changed DAW, recovery | Understand the specific access gap and supported fix without Wine administration. |
| An editor fails while safe audio processing continues | Ordinary use, recovery, alternatives | Understand the failed capability and recover the editor without an unnecessary installation reset. |
| New compatibility advice conflicts with a local setting | Alternative configuration, update | See current versus suggested values and keep the user's choice unless they change it. |
| Runtime or dependency change affects several products | Alternatives, vendor application, update | See shared scope before mutation and the actual limits of restoration. |
| Update occurs with an existing saved project or active instances | Update, ordinary use, recovery | Understand what must stop, what remains retained and how to test recall or restore the supported predecessor. |
| External library drive disappears or space runs out | Storage, recovery | Reconnect/change location without writing through an absent mount or deleting uncertain data. |
| Another bridge's installed collection is found | External collection | Understand ownership and available paths without silent adoption or duplicate substitution. |
| User removes one product or the entire manager | Remove, product retirement | Distinguish visibility, uninstall, activation and content, and understand effects on projects and shared siblings. |
| An inaccessible vendor window or offline unresolved failure needs help | Authorization, accessibility, support | Find a supported way forward, disclose relevant limits and deliberately share only appropriate information. |

For every scenario, check comprehension of the current situation, the intended
next action, its consequence, and the point at which the task is finished.
Record wrong turns and requests for explanation. Rewording alone is insufficient
when the person cannot locate the task or the product lacks a truthful outcome.

## From agreed model to implementation

The next design pass resolves the decisions above and walks the scenarios against
this model. Then derive information architecture and a disposable interaction
prototype, including failures and return visits. Test comprehension with the
operator's Bitwig case and a REAPER user before assigning an implementation slice.

After the flows hold together, compare existing behavior against them: retain a
mechanism that serves the flow, replace a conflicting one, or identify a missing
capability. Do not preserve screens merely because they exist. Sol implementation
and Astra review resume against agreed outcomes and necessary architecture changes.

Design completion means the task inventory, variations, ownership, completion
conditions and recovery paths are coherent and consequential scope choices are
resolved. It does not mean the manager defect is fixed or beta readiness changed.
Working software and ordinary installed use remain the delivery standard.

## Basis

The current operator instruction establishes the product-design reset. Governing
constraints are linked above; [support](../SUPPORT_MATRIX.md) and
[open failures](../FAILURE_CLASSES.md) remain the locations for implemented claims.
The operator's Kontakt account is user-reported experience, not a new automated
acceptance result. The previous design seed remains in Git history.

The discovery method follows [mapping the user's whole problem](https://www.gov.uk/service-manual/design/map-a-users-whole-problem)
and [researching users during discovery](https://www.gov.uk/service-manual/user-research/user-research-in-discovery):
include related services and user differences before selecting interface structure.
