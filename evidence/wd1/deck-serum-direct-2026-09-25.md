# WD1 Steam Deck direct Serum installation — 2026-09-25

This record separates manager-owned installation from FL discovery and musical
use. Private installer, prefix, account, and diagnostic bytes remain on the
Deck. The installed generation precedes the later source correction for the
Workspaces refresh delay; that correction is not part of this physical result.

## Exact managed installation

| Fact | Retained result |
|---|---|
| Installed WD1 source | PR #169 head `a113e9765e88550881fb4406cb68312ffe0158b8`, tree `6d99da2aa9fa0bb1e0762528b6f2b0a731f09a36` |
| Installed manager/frontend SHA-256 | `287fb35449a3046dde8aff931efd00d2b89cf31bcbe523511507a7b2b0b2df44` / `ce2966240d2608b44c1de01d33ae7ea02bf6c985f2d3b16b8b8a4d6ffd5c04b9` |
| WD0 tooling package generation | `407b31c32c0a17898f18d2b6cc7eb3664af618d0d93f1fe14590df50f609f3b4` |
| FL workspace | `762bafec213cefe91dbe14d67ee1b1c6`; same environment, prefix, projects, preferences, exports, and runner as before |
| FL application | Official `26.1.5.5618`, trial mode; prior installation and uninstall histories unchanged |
| FL runner | `proton-11.0-2c-fl-crypt32-order-v1`; audio device remains unknown and was not changed |
| Serum installer | Operator-selected official `2.1.5`; SHA-256 `507b726d97bf78920157f3817aff003b9ee38ee961f4efd318cf43216370f695` from existing canonical custody |
| Serum installation operation | `4f4c01aebbfe33355ba46b5436d3c604`; one retained product installation record, outcome `installed` |
| Installed VST3 module | `Serum2.vst3` in the FL prefix; SHA-256 `501e7bb3dd9cafe416b3412df3d4e084c01b7468201e5690b9009d7ecd4e5283`; current module digest verifies |

The operator used Workspaces to select the exact release, started the installer
once, completed the official installer UI, and pressed the manager's exact
completion readback action once. The supervised installer result was completed
with confirmed cleanup. The manager then reported Serum `installed`. The FL
application's selected installer, current executable, two installation records,
and one uninstall record remained exact. Only the workspace revision and the
separate Serum product state changed.

The open frontend kept its earlier `installing` snapshot after the installer
worker retired. A single read-only Refresh exposed the manager-offered
completion action. PR #169 now includes a source correction for the lightweight
poll to notice this retirement; it has not been installed on the Deck for this
physical pass.

## Native isolation

The native registry's SHA-256 before WD1 and after Serum installation remained
`01e29055a5b22bf0c007b4e2215ddf14acb7c74fbf890c0bc8437c0ca4df1c77`.
The pre-install and post-manager-replacement canonical snapshots had identical
native product records, including all six selected publications and the separate
Serum 2 FX attention item. Direct FL hosting created no native publication or
bridge DSP lease.

## FL-side acceptance

FL Studio launched through the managed workspace after Serum installation.
The operator reports that Serum loaded in FL, played audible notes from FL's
piano roll, and played the same pattern with a changed Serum preset. The Serum
editor initially exceeded the visible Deck area, hiding its lower-right resize
corner. The operator reported the window working well after using
[Xfer's documented logo-menu resize control](https://xferrecords.com/web-manual/serum-2/resizing-the-ui);
the exact selected zoom percentage was not recorded. WAV export, clean cohort
retirement, and fresh-session Serum use remain unobserved.
A fresh session will not establish FLP recall. The operator reports that their
trial session cannot save an FLP; no purchase or license change is required for
the established installation and audible musical result.

The operator subsequently created an FL automation clip and reported that the
Serum parameter automation worked during playback. The exact automated control
and curve were not retained. The operator declined a WAV export in this session;
export is **not tested**, rather than passed or failed. No FLP save or recall is
claimed. A read-only manager check while FL was open showed service active,
bridge DSP `0`, maintenance `0`, pending
transactions `0`, stale transports `0`, and cleanup uncertainty false. The
native registry hash remained the same as above.

The operator quit FL normally. Session operation
`508d66925c720fa0157341883633788a` retired with result `completed`,
`cleanup_confirmed=true`, and `owned_live=0`. Workspace readback returned
`ready` with confirmed cleanup; Serum's installed module still verified. The
native bridge remained idle at DSP `0`, maintenance `0`, pending transactions
`0`, stale transports `0`, and no cleanup uncertainty.

## Refreshed manager generation

After FL and the old frontend were closed, the source correction for automatic
Workspaces refresh was installed as a paired manager/frontend generation from
PR #169 head `5f5b3820080c3b8af7aad43ec923dce1668f3f3d`, tree
`815725f90fe29f4ab3018ccccc5220c4809c0bba`. The installed manager SHA-256
is `4197d6bd2102c9653e171c6333a15b67fcdefb9a605ff9866c922b7708e491f6`;
frontend SHA-256 is
`17bd37f05780479d543bb46587a9d229eae39ec6de3ec6509506224d6e8358dc`;
WD0 tooling package generation is
`587432f5c5acf757c52db51953080512616125c8a8afa7aa39ddf3aa661dad9e`.
The prior installed generation was retained as the rollback target.

The workspace record, FL installation history, and Serum installation history
were exact before and after this software replacement. The canonical native
product records matched the previous six-product snapshot; native registry SHA
remained `01e29055a5b22bf0c007b4e2215ddf14acb7c74fbf890c0bc8437c0ca4df1c77`.
The new manager reported FL `ready`, Serum `installed`, DSP `0`, and confirmed
cleanup. Fresh-session Serum playback remains pending operator observation.
