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
FL's own VST3 scan, piano-roll playback, preset change, parameter automation,
WAV export, clean cohort retirement, and fresh-session Serum use are pending
operator observation. A fresh session will not establish FLP recall. The
operator reports that their trial session cannot save an FLP; no purchase or
license change is required for the established installation result.
