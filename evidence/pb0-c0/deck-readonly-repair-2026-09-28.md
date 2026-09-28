# PB0-C0 focused repair: staged read-only Deck result — 2026-09-28

## Boundary and installed beforestate

The first paired x86-64 Linux repair binaries were copied only into `/tmp` for this gate. Manager SHA-256 was `f9fd2721769f17c6ea470fd8853fff566475ff60f696fc3e3ea3c75d2788286d`; frontend SHA-256 was `6e2fde8fe04255ca2eb10a87f32012c0d62eefce61d138faae1baca796a28c96`. A subsequent current-publication truth correction required one final-source repeat, recorded below. The staged frontend was not launched because its fixed product entry point must use the selected manager, and selecting the staged pair was outside this read-only gate. The final PR head and tree bind the reviewed source; the paired ordinary release build is a separate exact-head check.

The selected installed generation remained `6e4c53bf8533741d0e9423dacca9dd542b7329707c2748769cc495386a100385`, with manager/frontend digests `414a953b8e7b1186ae1bf3ce969f09be8c5a011b12dcbcfb83f1c1637449ad71` / `0963941036532478dab3c028e7b54cae8dca002f77a23bfa848e0dd60db797c9`. Its schema-11 Activity readback reported service active, DSP 0, keepers 2, maintenance 0, pending transactions 0, stale transports 0, cleanup uncertainty false and latest operation completed. The two keepers were present before this gate and remained afterward. No DAW, plug-in, installer or manager action was launched or submitted.

The fixed [read-only inventory method](../../tools/pb0-c0/read-only-inventory.py) counted 4,600 bounded manager-owned JSON/TOML identities and six user route identities before staging readback. Metadata aggregate SHA-256 was `acec615f4366df92b7995553640fddd882800751de6bbb894b872b716d0119a1`; route aggregate was `3b504a29f5df2e51a71a45e2efde02aa879216f591ed0d12b6978b67e59f4339`. It excludes vendor prefix content and retains no private paths or record values. The prior PB0-R3 count of 4,262 and the first PB0-C0 count of 5,454 came from separately bounded inventories whose path lists were not retained. Their exact count difference cannot be assigned to a specific category from the receipts alone; those cross-campaign counts are not used as a state-change comparison. This gate compares one fixed inventory method before and after.

## Staged current overview and pulse

The fixed [staged measurement](../../tools/pb0-c0/staged-readback.py) used the Deck's selected Desktop display/session facts and made five sequential read-only `overview` calls:

| Run | 1 | 2 | 3 | 4 | 5 |
|---|---:|---:|---:|---:|---:|
| Seconds | 2.043 | 1.843 | 1.408 | 1.394 | 1.386 |

Median was **1.408 s**; maximum was **2.043 s**. The response was schema-1 current overview paired to operator schema 12. All five had canonically equal current state tokens, 16 product rows, four Setup rows, one FL workspace, `action_required`, and the same primary step: **Verify Bitwig audio settings**. The manager did not claim a physical device rate or Bitwig callback maximum from PipeWire graph metadata. The installed BEAM/Lunacy products and FL/WD1 workspace remained represented; this staged readback did not launch or requalify them.

The first run's bounded phase diagnostic recorded `token_before` 188 ms, software/catalogue/profiles 80 ms, current products 690 ms, current owners 153 ms, Setup records 2 ms, inventory discovery 94 ms, current Setup 180 ms, workspace/cards 66 ms, and final token/owner/artifact recheck 151 ms. Capacity took 1,733 ms but ran in parallel with the current projection; its residual wait was 427 ms. Platform probes took 55 ms and readiness resolution was below one millisecond at this precision. These are phase observations, not independent additive durations.

Five sequential pulse calls each took **0.024 s**. Six further pulses at the selected ten-second idle cadence completed in 0.040–0.041 s each over 50.041 s; their direct manager child processes consumed 0.1526 CPU seconds and 66 process context switches in that interval. This is manager-process evidence only. The staged frontend was not run against a selected schema-12 manager, and no active audio session was present, so frontend idle CPU and audio deadline effect are not established here. The ordinary full Diagnostics Snapshot was measured at 38.894 s in the preceding receipt and was not rerun merely to collect another baseline.

The source-owned offered Setup action fixture produced a durable accepted receipt in **80 ms** without a Diagnostics Snapshot. This is source fixture timing, not a Deck mutation or an installer execution.

## Export, presentation and afterstate

`support-export-preview` constructed a parseable schema-2 support report in temporary audit memory. It contained 16 product summaries, the exact selected software generation and separately labeled manager/frontend digests. Package version was correctly absent for the older selected generation. The whole serialized report contained no live state token, support-export action, action `kind`, action arguments, home path, Wine prefix path, credential query value or token query value. No product-owned export record or network upload was created. The ordinary export action remains separately covered by source-owned offered-action and redaction tests.

Eight source-owned synthetic Home previews cover ready, action required, unsupported and unknown at 960 and 560 logical widths under [previews](previews/). They show layout only; no Deck frontend interaction is claimed.

The after inventory used the same script and returned **4,600** metadata identities, the same metadata aggregate `acec615f4366df92b7995553640fddd882800751de6bbb894b872b716d0119a1`, and the same six-route aggregate `3b504a29f5df2e51a71a45e2efde02aa879216f591ed0d12b6978b67e59f4339`. The installed Activity readback again showed the same generation, active service, DSP 0, keepers 2, maintenance 0, pending 0, stale 0 and cleanup uncertainty false. No installed generation, route, catalogue, publication, environment, runner, installer/onboarding record, UI2 record, FL workspace, vendor state or rollback authority was changed.

## Final-source repeat after current-publication correction

The current projection now distinguishes a physically valid `Removed` publication from a product actually selected for Bitwig. A removed product cannot receive a `Ready` status merely because its publication link is correctly absent. The ordinary Diagnostics projection uses the same selected-publication distinction. This source correction changed neither the installed Deck authority nor the 16 selected fixture rows, so the staged readback was repeated with the final source binaries.

The final staged audit manager SHA-256 was `03c9f417635d815c15a49f0d4494c5f85bb1b4bc87a586e184dda8116a66d43d`; paired frontend SHA-256 was `a3fd3086e2a01600f65c99f95fb18d826b73aeaebb5dd7eecf52529f5ed90cda`. The separately built ordinary release manager SHA-256 was `9d4419df7995790eeb12968ea0bcfe009c5cbca3c16eefe266538ae4a498d1b6`; it was not installed or run on the Deck. Five sequential staged overview calls took **2.039, 1.837, 1.393, 1.384 and 1.390 seconds**: median **1.393 s**, maximum **2.039 s**. Five pulse calls each took **0.024 s**. Every overview retained 16 products, four Setup rows, one FL workspace, schema-12 pairing, the same current token, `action_required`, and **Verify Bitwig audio settings** as its one primary step. The schema-2 support preview parsed with 16 product summaries, the same installed software-generation identity and no forbidden live action, token, home, prefix or credential material.

The fixed before/after method again returned 4,600 metadata identities with aggregate `acec615f4366df92b7995553640fddd882800751de6bbb894b872b716d0119a1` and six routes with aggregate `3b504a29f5df2e51a71a45e2efde02aa879216f591ed0d12b6978b67e59f4339` on both sides. Installed schema-11 Activity again reported active service, DSP 0, keepers 2, maintenance 0, pending 0, stale 0 and cleanup uncertainty false. The staged binaries were not selected; no product-owned export or operation record was created.

**Disposition:** the final-source staged interactive timing and current-state truth gate passed. This is source-only and read-only evidence. PR #188 remains draft and uninstalled pending independent review. A controlled immutable installation and real frontend/audio-idle check remain separate; PB1 has not begun.
