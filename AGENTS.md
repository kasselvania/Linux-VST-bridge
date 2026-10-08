# Working rules

## What we are building

A managed compatibility product that lets native Linux DAWs use Windows audio
plug-ins. A musician installs, authorizes, plays, saves, reopens and updates a
plug-in through the product, without administering Wine, Proton or prefixes.

**The deliverable is working software.** A report, trace, evidence file, review
or commit count is not progress. Progress is something a musician can now do
that they could not do before, or a defect that is now fixed and shown fixed.

The operator's current instruction comes first, then this file, then
`CURRENT_SLICE.md`, then `docs/ARCHITECTURE.md`. If two of them conflict, say so
in one sentence and follow the higher one.

## How to work: fix it, then show it

When something is broken, the job is to make it work.

1. **State your best explanation.** Say what you think the cause is and how
   confident you are. "Probably X, about 70%" is a useful engineering statement.
   "Cause not established" is not an acceptable place to stop while any
   experiment that could establish it remains untried.
2. **Change one thing and compare.** The fastest proof of a cause is to change
   the suspected thing and see whether the symptom goes away. This is not
   speculation; it is the experiment. Run it.
3. **Keep the change or revert it.** If the symptom is gone, keep it, add a
   regression test where one is possible, and move on. If not, revert, update
   your explanation and try the next most likely cause.
4. **Stop measuring when you can act.** Use the simplest measurement that
   answers the question. If a measuring tool fails twice, replace it with a
   simpler one; do not repair the tool as a project of its own.

Three attempts on one symptom without a change in the outcome means the approach
is wrong. Step back, say so, and propose a different approach.

## What you may do without asking

These are routine. Do them, then report what happened.

- Change scheduling priority, CPU affinity, environment variables, Wine/Proton
  options, buffer sizes or build flags on a test machine or the maintainer's
  Deck, for an experiment. Restore the previous value afterwards unless the
  change is being kept.
- Add or remove diagnostic code, timers and test fixtures.
- Change product source, including the engine, transport, manager and Windows
  host, when that is the fix.
- Build, install and roll back test packages on project machines.
- Rerun a test.

## What needs the operator first

- Anything that could destroy or alter user projects, licensed plug-in
  installations, activation state or account data.
- Spending money or exceeding the declared machine resource limits.
- Changing what the product is for, dropping a target platform, or changing
  what counts as done for the beta.
- A permanent change to the maintainer's machines outside the product's own
  files, such as system packages or root filesystem edits.

Ask one specific question and carry on with whatever does not depend on the
answer.

## How to report

Write naturally and concisely for a musician who owns the project. Use the
format and level of detail that suit the task. Explain results and remaining
problems in plain language, including relevant numbers when they help.

Leave out commit hashes, package names, tree IDs and lists of things you are not
claiming, unless the operator asks. Do not write a sentence whose only purpose is
to avoid being wrong. If a result is uncertain, say how uncertain in a few words.

Never report a failure as a success, and never hide one. That is the whole of
the honesty rule; it does not require a paragraph of caveats.

## Records

- Raw traces, captures and logs stay outside Git.
- Do not create a per-run report or evidence file for a diagnostic run. The
  commit message and the status update are the record.
- Add an evidence file only when it backs a claim in `docs/SUPPORT_MATRIX.md`,
  and keep it to the numbers that back the claim.
- `docs/SUPPORT_MATRIX.md` says what works on what. `docs/FAILURE_CLASSES.md`
  lists open defects. Update each with a line or two. Do not start another
  status document, ledger, campaign or receipt system.
- `CURRENT_SLICE.md` fits on one screen, about forty lines: the goal, the best
  explanation, the next changes, and what done looks like.
- Test builds are numbered simply. Do not invent a new candidate name per run.

## Landing code

- `main` must move. Merge working increments in small pull requests.
- A stack is at most two pull requests deep. If it is deeper, stop and land it.
- Code going to `main` gets a review. Diagnostics, experiments and notes do not.
- Green tests are required for a merge. They are not the definition of done; a
  musician being able to use the thing is.

## Design rules

- The DAW loads a native Linux proxy. A supervised Windows host loads the
  Windows plug-in under a pinned runner. The DAW never loads a Windows binary.
- Rust is the product language. C++ is confined to the VST3 SDK and Win32
  edges, behind a versioned C ABI. Process boundaries use a versioned protocol.
- Build general mechanisms. A plug-in is a test case, not a design unit. Do not
  add product-name branches or per-binary builds to cover a missing mechanism.
  A vendor-specific workaround needs a vendor-specific reason and belongs in
  declarative profile data where possible.
- An unfamiliar plug-in or distribution is untested, not forbidden. Refuse only
  for a specific missing capability, and name it.
- Identity is by content and class, not by path or display name. Never silently
  substitute a different build, preset, environment or content root.
- Saved projects and plug-in state must survive save, restart, reboot, update
  and rollback before a plug-in is called usable.
- The product acquires its own runtime. Steam, a customer-installed Wine, an
  SDK or a compiler is never a prerequisite.
- No single Wine prefix for every vendor and no single process for every
  plug-in by default.
- Editor, scanner, installer and audio failures are reported as what they are.

## Real-time rules

On the audio callback path: no heap allocation after activation, no file,
network or process work, no ordinary logging, no unbounded lock or wait, and no
dependence on the manager or an editor. Every wait has a stated bound and a
stated result when it expires. A dead Windows host must be detected quickly and
must not stall the DAW.

Measure the whole plug-in call as the DAW sees it. A bridge counter reading zero
is not proof that the audio arrived on time.

## Protections that do not bend

These are not process; they are limits. No experiment overrides them.

- Vendor licensing belongs to the vendor and the user. Never bypass, emulate,
  forge, intercept or redistribute licensing material.
- Never commit or log credentials, tokens, licence files, activation data,
  serials, account identifiers, proprietary installers, plug-in binaries, vendor
  presets or paid content.
- Diagnostic exports are allow-listed. Never archive a whole prefix.
- Do not copy yabridge source. Read it as prior art and write original code.
- Do not redistribute Proton, Wine, DXVK, VST3 SDK files, installers or
  plug-ins until the distribution obligations are recorded.
- Do not disable SteamOS's protected base or make root filesystem changes a
  product requirement.
- Profiles are data, not scripts. No shell commands, binary patches or hidden
  downloads in a profile.
- Never delete an uncertain ownership record to make a recovery look successful.
- Do not imply affiliation with or certification by any DAW, runtime or plug-in
  vendor.
