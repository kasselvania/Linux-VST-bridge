# UI1 — Guided plug-in onboarding

UI1 is a paired manager/frontend projection over the existing managed installer,
environment, supervision and inventory owners. It does not install, inspect,
prepare or publish by itself. This public canonical integration starts from
main after WD1 PR #169. It does not import private BG1, BETA0, RPR0, DIST0,
CACHY0 or UI2 implementation.

## Human stages and retained records

| User-facing stage | Manager fact | Next step |
|---|---|---|
| Ready to set up | Exact imported artifact, no environment | Continue setup with the package-selected standard runner |
| Ready to install | Manager-owned isolated environment, no vendor operation | Explicitly Run installer |
| Installer open | Exact live vendor operation | Complete the vendor UI; Focus and Stop remain exact controls |
| Needs attention | Failure or uncertain cleanup | Read the reason and use only offered recovery |
| Installer closed | Exact retired operation, result retained | Find installed plug-ins when offered |
| Plug-ins found | Exact inventory scan and product identities | View each exact Plug-ins record for its current compatibility and publication state |

An imported file is not an installation attempt. The Setup page groups all
attempts under one installer card. `Setup history` and `Technical details`
retain the old operation results, scans, environment IDs, runner details and
receipts. Discovered-product routing requires exact environment, module SHA-256
and VST3 class ID. A friendly product name is never a relation key.
Discovery does not itself publish a plug-in. A setup card with mixed historical
product states routes to the exact Plug-ins records and does not claim that all
of them are unavailable merely because the original scan found them together.
Exact installers owned by the FL Studio application's selected or historical
installation records stay in Workspaces. They do not receive plug-in Setup
actions merely because both flows use canonical installer custody.

## Closed import and presentation identity

The frontend gives the manager an inherited read-only file descriptor and only
the selected basename. The manager verifies and copies the bytes, hashes them,
and returns a typed schema-2 import result with exact SHA-256, size, format,
new-versus-existing disposition, display label and import time. The full source
path is not retained. Exact duplicate bytes return the existing installer and
focus the existing Setup card.

The schema-1 installer custody record is unchanged. A separate small schema-1
presentation sidecar binds a bounded label to its SHA-256. Suggested names come
from a sanitized basename; generic names include a short identity suffix.
`Rename` is a closed typed manager-offered action. It changes presentation only:
no artifact, environment, profile, candidate or publication authority follows
from a label. Existing schema-1 Lunacy records can be named without reimport.
PE/MSI metadata parsing was not needed for the first usable identity.

## Standard compatibility policy

The immutable software catalogue now includes an optional schema-1
`OnboardingRuntimePolicy` with one exact runner key and the display posture
`Standard · recommended`. Package adoption derives that key only from the
unmodified standard Proton/SLR runner ID
`proton-11.0-2c-25118279-slr4-4.0.20260805.254769`
with no specialty runner policy. Its key binds the complete serialized runner,
including pinned file identities. The current package must still verify all
runner files. The Setup primary offer contains that exact key and uses the
existing closed environment-creation action. It does not execute the vendor
installer. No specialty runner is an alternative in UI1. If the declared key
is missing or changed, Setup explains that the standard runtime is unavailable
and offers no environment mutation.

## Request presentation

The frontend tracks closed origins: initial snapshot, background Activity,
silent post-mutation refresh, explicit Refresh, installer import, and user
action. Only foreground import/action produces a persistent request notice.
Background Activity updates system state and exact operation reconciliation
without a spinner, foreground notice overwrite or control lockout. A click
made while Activity is in flight is queued once; exact operation feedback and
uncertain-acknowledgment reconciliation retain their existing manager identity
rules. The idle large Request status panel is gone. Notices can be dismissed
after the current operation releases.

## Wire and public integration boundary

The public UI0/WD1 operator schema remains 8. UI1 adds optional setup and
presentation fields to that snapshot and keeps existing closed request
validation and uncertain-acknowledgment reconciliation. Installer custody
records remain schema 1; the presentation sidecar and catalogue runtime policy
are separately versioned. No private schema-10/11 action or beta support
snapshot is imported. A later UI2 owner PR must separately reconcile the
installed guided-check and guided-result records.

## Evidence posture

Source tests and synthetic normal/narrow captures establish projection and
layout behavior only. The private UI1 Deck result is retained under its
original source authority; it is not a physical result for this new public
integration. Do not install this public branch on the Deck while the installed
UI2 generation still owns guided compatibility records. It must first be
reviewed alongside the separate UI2 and package-binding integration plan.

UI2 owns guided compatibility inspection, candidate preparation, physical
observation, review and publication disposition.
