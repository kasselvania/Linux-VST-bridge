# Product flow walkthrough

Read the diagrams in order, starting with ordinary use and adding a plug-in.
The [interactive viewer](index.html) provides readable-size and fit controls.
The [full product model](../01-product-and-user-experience.md) contains the task
inventory and detailed boundaries. These are design flows, not installed claims.

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
| Kontakt already works and recalls in Bitwig | Return to the manager, inspect Kontakt if wanted, then continue in Bitwig | For this scenario, the current route is valid and only an old installer result is missing. Show current availability and the attributed use report; retain history without turning it into unrelated setup work | Nothing needs confirming to continue use. If the user later requests a change that needs missing history, explain that particular limitation and a valid recovery path. |

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

The user handles vendor choices. The product supplies compatibility work and automatically makes selected plug-ins available after vendor steps and required checks. A closed vendor window alone is not proof that maintenance has safely ended.

[Editable Mermaid source](install.mmd)

```mermaid
flowchart TD
    Choose["User chooses software to add<br/>Installer or managed vendor application"] --> Context["Product reuses or prepares the right environment<br/>Obtains its own runtime and checks storage"]
    Context --> Shared{"Could maintenance affect<br/>plug-ins already in use?"}
    Shared -->|Yes| Pause["Show affected plug-ins<br/>User stops their use or defers this task"]
    Shared -->|No| Vendor["User completes vendor installation<br/>and authorization when required"]
    Pause -->|User continues after use stops| Vendor
    Pause -->|Defer| Later["Keep the current working setup<br/>Return later"]
    Vendor --> Handoff{"Has vendor installation<br/>finished safely?"}
    Handoff -->|No or interrupted| Recover["Explain the unfinished step<br/>Resume, finish or safely stop that work"]
    Recover --> Handoff
    Recover -->|No supported continuation| Help["Preserve partial work and useful results<br/>Explain the limitation and offer help or exit"]
    Handoff -->|Yes| Automatic["Product discovers selected plug-ins<br/>Checks capabilities and prepares them automatically"]
    Automatic --> Result{"Can this selected plug-in<br/>be made available?"}
    Result -->|Yes| Available["Product makes it available to the DAW<br/>No extra publication approval"]
    Result -->|Specific gap| Problem["Name the missing capability or failed step<br/>Offer a relevant retry or help"]
    Available --> Music["User finds it in their own DAW<br/>Rescans only if needed, then makes music"]
    classDef outcome fill:#d9f2e5,stroke:#26734d,color:#153b29
    classDef choice fill:#fff2cc,stroke:#ad7900,color:#4b3900
    class Available,Music,Later outcome
    class Shared,Handoff,Result choice
```

## Kontakt already works

Current availability, the user’s Bitwig experience and the authority needed for a future update are different facts. Missing old installer history creates a task only when it has a concrete consequence; a user report cannot manufacture missing current authority.

[Editable Mermaid source](working.mmd)

```mermaid
flowchart TD
    Return["Return to Kontakt after using it in Bitwig"] --> Read["Product reads current facts<br/>Keeps your successful-use report distinct"]
    Read --> Route{"Can the product safely make this<br/>installed plug-in available now?"}
    Route -->|Yes| Available["Show available for DAW use<br/>Keep your Bitwig experience visible"]
    Available --> Choice{"What do you want next?"}
    Choice -->|Keep making music| Music["Continue in Bitwig<br/>No installation completion ceremony"]
    Choice -->|Update or change something| Check["Check what that particular change requires"]
    Check --> Gap{"Is information needed for<br/>this particular change missing?"}
    Gap -->|No| Change["Explain consequences<br/>Continue through the change flow"]
    Gap -->|Yes| Explain["Explain why this change cannot run yet<br/>Keep current use available when it remains valid"]
    Route -->|No or unknown| Specific["Explain the actual current problem<br/>Your earlier success is still recorded"]
    Specific --> Recovery["Offer the relevant fix or help<br/>Preserve existing files and state"]
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

## Open the interactive viewer locally

From the repository root, run:

```sh
python3 -m http.server 8769 --bind 127.0.0.1 --directory docs/design-dossier
```

Then open <http://127.0.0.1:8769/flows/>. The viewer loads a pinned Mermaid
renderer from a CDN. The Markdown and Mermaid sources remain usable independently.
When editing a Mermaid source, keep its matching block in this walkthrough aligned.
