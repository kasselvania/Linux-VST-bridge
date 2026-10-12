# Product flow walkthrough

Read the diagrams in order, starting with ordinary use and adding a plug-in.
The [interactive viewer](index.html) provides readable-size and fit controls.
The [full product model](../01-product-and-user-experience.md) contains the task
inventory and detailed boundaries. These are design flows, not installed claims.

The [replacement clickable prototype](../prototype/index.html) follows one musician
adding eFX FRAGMENTS to Bitwig through Arturia Software Center. The operator
rejected the earlier layout comparison because specification terms became confusing
actions with no concrete user reason. The new question is whether each step and
button makes sense without narration. It includes downloaded-installer choice,
suggested storage with an optional drive change, a separate vendor-window stand-in,
automatic readiness and one unfinished download/resume. The user accepted this
first-install interaction, including the suggested device-neutral storage.
The next [returning journey](../prototype/index.html?journey=returning) adds Analog
Lab V through the already installed ASC. It retains the installation and location,
names FRAGMENTS before shared installation work, and offers postponement while its
Bitwig project is open. The project-close event is outside the product; stopping
playback is not enough. Completed first installation naturally reaches the same
ASC entry. The user also accepted this returning interaction. The
[working Kontakt mock](../prototype/index.html?journey=kontakt) is illustrative but
insufficient: it assumes a valid connection rather than explaining the original
conflation of experimental publication, testing, installer history and availability.
The user's Bitwig play/save/reopen report remains attributed. Missing history alone
is not the explicit partial-installation condition behind the special old warning;
the exact installed cause remains unverified. The [responsibility table and sequence](../01-product-and-user-experience.md#responsibilities-and-normal-sequence)
now separate the work and its observable results. Manager implementation stays paused.
The broader flows and five selected policies still apply, including update recovery.
All activity is in-memory simulation. **Save offline copy** downloads the complete
prototype with the chosen starting journey; reload starts that fixture over. The
outside-product selector and `?journey=first/returning/kontakt` choose the starting situation.
The optional server command below serves it at
`/prototype/`; direct file viewing needs no server.

## The five agreed choices

- **First installation:** chosen plug-ins become available automatically after
  vendor steps and required checks. Changing a working setup gets its own review.
- **Existing collections:** beta manages this product’s installations and coexists
  with external Wine/yabridge setups. Migration is later work.
- **Storage:** beta includes drive choice, missing-library reconnection and
  supported same-machine moves. Full collection backup and computer transfer come
  later. Recovery from product updates remains required.
- **Completion:** normal status uses observed facts. User feedback is optional in
  help/testing; ordinary use never requires “mark installed”.
- **Support:** testers use their existing private contact with the maintainer.
  Diagnostics are exported and shared only by deliberate user choice. An in-app
  support submission service is not required for beta.

The policies are selected; the flow details still need walkthrough and usability
validation. Boxes describe work, not a list of screens. Diamonds include automatic
product decisions and do not imply prompts for the user.

Green marks an outcome; yellow marks a decision or condition. Labels carry the
same meaning without color. Each Mermaid file is editable; the matching diagram
below renders on GitHub.

## Start with three concrete situations

These are proposed interactions for discussion, not observed test results or
final screen copy. The diagrams include automatic product work as well as actions
the person takes.

| Situation | What the person does | What the product handles | When the task is finished |
| --- | --- | --- | --- |
| A new REAPER user wants a plug-in | Install the manager, choose the installer or vendor application, make needed drive/DAW choices, and complete vendor steps | Acquire its runtime, supervise installation, discover and prepare the selected plug-ins, and make them available; explain a real failure if one occurs | The user can find the plug-in in REAPER and make music. Rescan guidance appears only when needed. No separate prepare/publish lesson or completion checkbox. |
| An ASC user wants another Arturia product | Open the existing managed ASC context, choose the additional product and complete vendor choices | Show affected plug-ins before maintenance; preserve the existing identity; finish the safe handoff from vendor maintenance; automatically prepare the selected new products | The new product is available and existing products have a clear result. If affected plug-ins are in use, the user can defer maintenance and keep making music. |
| Kontakt already plays and recalls in Bitwig, as reported by the user | Continue in Bitwig; inspect optional information or request a concrete change only when wanted | Establish current availability independently of installer history and compatibility evidence; retain the attributed experience and unknown history | A valid native connection has no completion chore or mandatory test in another DAW. Actual use failures and a requested change receive their own concrete actions. |

The next design exercise is to walk these situations through proposed navigation
and actions. A flow is successful when the person can explain their situation,
choose the next action and recognize completion without the designer narrating it.

## An ordinary visit

Start with the collection and the user’s goal. A working user can go straight back to the DAW. This does not prescribe a screen layout.

[Editable Mermaid source](start.mmd)

```mermaid
flowchart TD
    Arrive["Open the manager"] --> Collection["See your collection<br/>and any current work"]
    Collection --> Intent{"What do you want to do?"}
    Intent -->|Make music| Music["Continue in your DAW<br/>The manager can be closed"]
    Intent -->|Add a plug-in| Add["Choose an installer<br/>or vendor application"]
    Intent -->|Manage something| Manage["Choose the affected plug-in<br/>or current task"]
    Manage --> Action["Update, adjust, fix,<br/>move or remove"]
    Add --> Result["See the result<br/>and any useful next action"]
    Action --> Result
    Result --> Collection
    classDef outcome fill:#d9f2e5,stroke:#26734d,color:#153b29
    classDef choice fill:#fff2cc,stroke:#ad7900,color:#4b3900
    class Music,Result outcome
    class Intent choice
```

## Add a plug-in

The user handles vendor choices; the product automatically discovers, inspects and
connects selected plug-ins to the chosen DAW. Installation mutations must end safely;
a closed window is insufficient and needed licensing services may remain alive.
Dashed boxes describe cross-cutting handoffs and optional evidence, not extra gates.
Authorization may occur before, during, at first use or later; an actual request
returns to the same step and environment. Unknown licence state is not failure.
Publication does not prove DAW indexing, audio correctness or recall.

[Editable Mermaid source](install.mmd)

```mermaid
flowchart TD
    Choose["User chooses software, lawful source and DAW<br/>Accepts suggested storage or changes it"] --> Context["Product prepares or reuses the environment<br/>Keeps its identity, runtime and supported services"]
    Context --> Shared{"Could vendor changes conflict with<br/>plug-in instances already in use?"}
    Shared -->|Yes| Pause["Name the plug-ins that must stop<br/>Close their instances or defer installation"]
    Shared -->|No| Vendor["Vendor installs the chosen software<br/>User answers its actual choices and requests"]
    Pause -->|Instances closed| Vendor
    Pause -->|Defer| Later["Keep the current working setup<br/>Return later"]
    Vendor --> Handoff{"Have installation mutations<br/>ended safely?"}
    Handoff -->|No or interrupted| Recover["Explain the unfinished step<br/>Resume, finish or safely stop that work"]
    Recover --> Handoff
    Recover -->|No supported continuation| Help["Preserve partial work and useful results<br/>Explain the limitation and offer help or exit"]
    Handoff -->|Yes| Discover["Automatically discover selected classes<br/>Inspect basic interfaces and dependencies"]
    Discover -->|Inspection succeeds| Prepare["Automatically prepare the native DAW connection<br/>Exact plug-in, environment and supported access"]
    Discover -->|Inspection fails| Problem["Name the failed capability or step<br/>Do not guess that a load error is licensing<br/>Offer the matching retry, vendor route or help"]
    Prepare -->|Preparation succeeds| Publish["Publish the native connection<br/>Through supported access for the chosen DAW"]
    Prepare -->|Preparation fails| Problem
    Publish --> Result{"Did publication succeed?"}
    Result -->|Yes| Available["Show availability for the chosen DAW<br/>No extra approval or unknown-licence gate"]
    Result -->|Publication fails| Problem
    Available --> Music["User finds it in their own DAW and makes music<br/>Indexing, audio and recall are separate results"]
    Vendor -.-> Auth["If a real vendor request appears at any step:<br/>use its supported route, then resume the same step<br/>in the same environment; keep needed services alive"]
    Discover -.-> Auth
    Music -.-> Auth
    Available -.-> Evidence["Optional compatibility detail or report<br/>Not yet tested by us does not block music"]
    classDef outcome fill:#d9f2e5,stroke:#26734d,color:#153b29
    classDef choice fill:#fff2cc,stroke:#ad7900,color:#4b3900
    class Available,Music,Later outcome
    class Shared,Handoff,Result choice
```

## Kontakt already works

Current availability, the user's Bitwig experience, compatibility evidence and
history are different facts. “Not yet tested by us” can be a quiet optional detail;
it does not require a REAPER test or success report. Missing history matters only
when a requested operation needs it; a report cannot manufacture current authority.
An actual DAW failure does not erase the installation and needs its specific fix.
The diagram specifies desired behavior, not the exact installed manager's cause.

[Editable Mermaid source](working.mmd)

```mermaid
flowchart TD
    Return["Return to Kontakt after reported Bitwig use"] --> Read["Read current native connection and availability facts<br/>Keep your reported play and recall attributed"]
    Read --> Route{"Is the product-owned connection<br/>valid for Bitwig now?"}
    Route -->|Yes| Available["Show availability for Bitwig<br/>No installer-history or testing completion task"]
    Available --> Choice{"What do you want next?"}
    Choice -->|Keep making music| Music["Continue in your existing Bitwig session<br/>The manager can be closed"]
    Available -.-> Evidence["Quiet optional compatibility information<br/>Not yet tested by us is separate from availability<br/>A report is optional, not formal support"]
    Choice -->|User requests a change| Check["Check only what that particular change requires"]
    Check --> Gap{"Is information or authority needed<br/>for this change missing?"}
    Gap -->|No| Change["Explain consequences before mutation<br/>Preserve identity and review shared changes"]
    Gap -->|Yes| Explain["Explain this change's concrete limitation<br/>Keep current use when its connection remains valid"]
    Route -->|Specific invalid or unverified connection| Specific["Name the current access or capability problem<br/>Installed files and earlier experience are retained"]
    Music -->|An actual DAW problem occurs| Specific
    Specific --> Recovery["Offer help for the actual failed step<br/>Indexing, load, sounds, editor, audio or recall"]
    Recovery -.-> Auth["Only an actual vendor request uses its supported authorization route<br/>Then resume the same step in the same environment<br/>Unknown licence or a generic load error is not that request"]
    classDef outcome fill:#d9f2e5,stroke:#26734d,color:#153b29
    classDef choice fill:#fff2cc,stroke:#ad7900,color:#4b3900
    class Music,Available outcome
    class Route,Choice,Gap choice
```

## Update or change settings

Explain shared effects and the actual restoration limits before a change. Retain a route back when preparation passes but actual DAW use fails. A trial can be kept or restored; ordinary use needs no completion checkbox.

[Editable Mermaid source](change.mmd)

```mermaid
flowchart TD
    Request["User asks to update software or try settings"] --> Scope["Product identifies the component and affected plug-ins<br/>Shows the change and actual restore limits"]
    Scope --> Consent{"User wants to proceed?"}
    Consent -->|No| Keep["Keep the selected working setup"]
    Consent -->|Yes| Active{"Must affected plug-in use stop?"}
    Active -->|Yes| Wait["Name the affected use<br/>Wait for it to stop or let the user defer"]
    Wait -->|Defer| Keep
    Wait -->|Use stopped| Apply["Perform the chosen product or vendor operation"]
    Active -->|No| Apply
    Apply --> Check["Recheck changed facts<br/>Prepare the agreed configuration"]
    Check --> Result{"Required checks pass?"}
    Result -->|Yes| Available["Show the selected configuration and result<br/>User tries it in the DAW"]
    Available -->|Continue using it or keep the settings trial| Music["Continue making music<br/>No mandatory completion checkbox"]
    Available -->|Problem in actual use or user chooses Restore| Restore
    Result -->|No or interrupted| Restore{"Can the previous configuration<br/>actually be restored?"}
    Restore -->|Yes| Return["Offer the available restoration<br/>Check it before returning to use"]
    Restore -->|No| Problem["Explain the specific failure and next remedy<br/>Do not promise a vendor or licence rollback"]
    classDef outcome fill:#d9f2e5,stroke:#26734d,color:#153b29
    classDef choice fill:#fff2cc,stroke:#ad7900,color:#4b3900
    class Keep,Available,Music outcome
    class Consent,Active,Result,Restore choice
```

## Recover a problem

Preserve the original task and independently valid work. A specific failure gets a relevant retry, external handoff, supported restoration or help/exit route.

[Editable Mermaid source](recover.mmd)

```mermaid
flowchart TD
    Trouble["An operation fails or the user returns after interruption"] --> Explain["Show the original task, what finished<br/>and the specific remaining problem"]
    Explain --> Scope["Separate the affected capability<br/>from independently valid work"]
    Scope --> Next{"What can actually resolve it?"}
    Next -->|Product can retry or resume| Retry["Run the relevant retry or resume<br/>Check that this operation can run safely"]
    Next -->|Vendor action needed| Vendor["Open the correct vendor window<br/>Return to this task afterward"]
    Next -->|DAW action needed| DAW["Give instructions for the user's DAW<br/>Example: rescan when the plug-in is missing"]
    Next -->|Valid predecessor exists| Restore["Explain what can be restored<br/>User chooses the scoped restoration"]
    Next -->|No supported remedy| Support["Explain the limitation<br/>Offer deliberate support sharing or exit"]
    Retry --> Result{"Original task can continue?"}
    Vendor --> Result
    DAW --> Result
    Restore --> Result
    Result -->|Yes| Continue["Return to the original task<br/>No restart of unrelated setup"]
    Result -->|No| Support
    classDef outcome fill:#d9f2e5,stroke:#26734d,color:#153b29
    classDef choice fill:#fff2cc,stroke:#ad7900,color:#4b3900
    class Continue outcome
    class Next,Result choice
```

## Move, reconnect or remove

Reconnecting the same unchanged drive is distinct from moving or remapping content. Every mutation follows ownership, capability, destination and consequence checks. Unsupported relocation or licensed migration remains a named gap, not a promised operation.

[Editable Mermaid source](storage.mmd)

```mermaid
flowchart TD
    Intent["User wants storage help or removal"] --> Kind{"What is the goal?"}
    Kind -->|Reconnect the same unchanged drive| Reconnect["Reconnect the existing drive<br/>No file movement or mapping change"]
    Kind -->|Relocate or remap content, move files or free space| Inspect["Show owned files, sizes and shared dependencies<br/>Separate caches, installations and user content"]
    Kind -->|Remove a plug-in or manager| Remove["Separate DAW visibility, uninstall and deactivation<br/>Explain effects on projects and sibling products"]
    Inspect --> Capable{"Is this a supported operation<br/>on files the product may manage?"}
    Remove --> Capable
    Capable -->|No| Limit["Explain the exact limit<br/>Preserve uncertain files and existing use"]
    Capable -->|Yes| Choose["User chooses the exact operation<br/>and sees anything that will be lost"]
    Choose --> Execute["Run the scoped operation<br/>Do not write through an absent mount"]
    Execute --> Verify["Verify the actual result and remaining access<br/>Retain recoverable data until safe cleanup"]
    Reconnect --> Verify
    Verify --> Done["Show what moved, remains or was removed<br/>Explain any remaining project action"]
    classDef outcome fill:#d9f2e5,stroke:#26734d,color:#153b29
    classDef choice fill:#fff2cc,stroke:#ad7900,color:#4b3900
    class Done,Limit outcome
    class Kind,Capable choice
```

## Deliver and support the product

These are our responsibilities, not extra work for the musician. A release, support promise or restoration offer needs a corresponding operation behind it.

[Editable Mermaid source](product-lifetime.mmd)

```mermaid
flowchart LR
    Define["Team defines scope<br/>and truthful compatibility claims"] --> Qualify["Test whole install, use,<br/>recall, update and recovery flows"]
    Qualify --> Release["Deliver a coherent release<br/>with clear requirements and limitations"]
    Release --> Use["Users install and make music<br/>Receive help when needed"]
    Use --> Learn["Learn from deliberate reports<br/>Improve general mechanisms"]
    Learn --> Qualify
    Use --> Fault{"Bad release discovered?"}
    Fault -->|Yes| Stop["Stop recommending it<br/>Explain affected versions and remedy"]
    Stop --> Remedy["Deliver repair or a valid predecessor<br/>Preserve projects and licensed state"]
    Remedy --> Use
    Fault -->|No| Maintain["Maintain releases and support<br/>Communicate changes before retirement"]
    classDef outcome fill:#d9f2e5,stroke:#26734d,color:#153b29
    classDef choice fill:#fff2cc,stroke:#ad7900,color:#4b3900
    class Use outcome
    class Fault choice
```

## Open the standalone viewer

Open [index.html](index.html) directly in a browser, including from the file
browser. It embeds all seven SVG diagrams and their Mermaid text. Viewing and
switching flows need no server, renderer download or internet connection.
**Save offline copy** downloads one HTML file with the current flow and viewing
preferences. **Editable Mermaid source** downloads the selected flow's `.mmd`
text from the embedded copy.

An optional local HTTP address is also available. From the repository root, run:

```sh
python3 -m http.server 8769 --bind 127.0.0.1 --directory docs/design-dossier
```

Then open <http://127.0.0.1:8769/flows/>. Save an offline copy before closing the
server if you want to keep using the downloaded file later.

## Regenerate after editing a diagram

The `.mmd` files are the source of truth. Keep their matching Mermaid blocks in
this walkthrough aligned when editing them, then run:

```sh
python3 docs/design-dossier/flows/regenerate.py
```

Open the temporary localhost URL printed by the command in a browser. This
renders all seven sources with Mermaid **11.12.0** and the viewer's existing theme,
then replaces only the embedded source/SVG block in `index.html`. The command
stops its temporary server when done. Regeneration needs an internet connection
for the pinned renderer; the resulting viewer does not. Python and a browser are
enough, with no installed Node packages or permanent service. A failed render
leaves the existing viewer intact; Ctrl-C cancels the temporary server.
