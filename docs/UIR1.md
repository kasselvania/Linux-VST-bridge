# UIR1 — generated input-retrieval differential and bounded pump repair

UIO1 is accepted and merged at `ddf256c60d3f770e3f2773724d260ed362a1267f`.
Review 5186750570 accepts its 8.237357339-second conservative lower bound from
exact X11 delivery to first observable Win32 hardware-mouse retrieval. Internal
Wine translation/admission/queue ownership remains unresolved.

UIR1 first compares a normal generated parent and child HWND under the actual
production `VendorView::pump`: idle, then finite posted/sent-message traffic.
The existing UIO1 observer and exact-window XTEST/X RECORD adapters are reused.
The fixed generated workload is intentionally capable of maintaining posted
traffic; it does not purport to reproduce Pigments' undisclosed internal traffic.

The test uses a private headless KWin virtual output and XWayland with a fresh
scratch Proton prefix, not the operator's display or authorized environment.
That isolates input and process ownership. It also limits extrapolation to the
physical desktop: compositor/output and vendor workload differences remain.

`tools/uir1/windows.cpp` calls the production pump and admits no vendor
binary. Its status is fixed scalar data in a private mapping. The UI thread
counts exact posted, sent and mouse handling; UIO1 independently observes mouse
retrieval and heartbeats. The companion producer has finite count/time limits.
At normal completion every submitted message must have been handled and both
windows must be destroyed. Hosted Windows tests verify that law before the
pinned-Proton differential.

The original generated result below selected the repair direction. Review
5186994847 accepted that baseline; the later repair and product confirmation
are retained separately below and in `evidence/uir1/repair/`.

The relevant specified behavior is that unfiltered `PeekMessage` services sent
and posted messages before hardware input. Its bounded outer call count does not
itself impose fairness between those message classes. The generated traffic
therefore tests a documented competing explanation; it does not assume it was
the internal cause of UIO1's vendor observation. See Microsoft's
[PeekMessageW contract](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-peekmessagew).

## Observed differential

The successful isolated run used the accepted pinned Proton/SLR files, a fresh
unlicensed prefix, the unchanged UIO1 observer/hook bytes and generated windows.
The observer remained attached for all three input pairs.

| Condition | Down receipt | Up receipt | Maximum heartbeat |
| --- | --- | --- | --- |
| Idle | issued-to-retrieval upper bound 2.910 ms | upper bound 1.498 ms | 4.131 ms |
| Bounded posted/sent traffic | X11-to-retrieval lower bound 4,962.810 ms | lower bound 4,874.841 ms | 6.108 ms |
| Idle after traffic | issued-to-retrieval upper bound 4.126 ms | upper bound 1.793 ms | 3.244 ms |

The different bounds are intentional. Fast retrieval can precede the Linux
X RECORD poll; a zero lower bound does not mean zero latency. Windows QPC is
projected with the existing Rust UIO1 bracket (round-trip uncertainty plus
100 ppm drift, at most ten seconds from a bracket). No X server clock is directly
subtracted from QPC.

All 4096 posted messages and 64 sent messages completed. There were no failed
sends, dropped observer records, dropped X RECORD records, hook errors or
unhook errors. All three Down/Up pairs reached the same child at client
coordinates (236,166), with focus on that child and the correct active parent.
The downstream mouse hook returned zero. Both windows were destroyed normally.

The bounded progress samples show all 64 sends complete at relative 1.200 s,
with 474 posts handled and the loaded mouse pair still pending. At 5.672 s,
4053 posts were handled and the loaded pair was still pending. At 5.735 s,
all 4096 posts and the pair were handled. Samples are observations at 50ms target
cadence, not exact dispatch timestamps. Their capacity is 1200; 193 were retained,
with zero overflow. See [the generated report](../evidence/uir1/differential.json).

## Decision and claim boundary

**The generated workload reproduces hardware-input retrieval starvation under
the generic production message pump.** Idle retrieval is prompt, posted
heartbeats remain prompt during the delay, sent traffic finishes well before
mouse retrieval, and mouse retrieval resumes when the finite posted chains
finish. A Pigments or renderer exception is not the next repair direction.

The selected next repair is bounded generic pump fairness between input and
posted work, preserving sent-message semantics, every queued message, and
normal translation/dispatch/quit ownership. The existing 128-message outer bound
limits iterations; it does not ensure hardware input gets a turn while posted
work is continuously available. The first checkpoint supplied the reproducible
baseline and selected that owner. The subsequent repair changes the generic
pump and its engineering publication/diagnostic support.

This does **not** prove that Pigments generated these particular messages or
that every part of UIO1's 8.237-second delay had this cause. Wine's internal
translation/admission timestamps remain unobserved. The isolated virtual
compositor is not a physical-desktop performance fixture. No further Pigments
session was run, and no renderer, audio, universal latency or gap-free claim is
made.

## Harness failures and cleanup

The first Windows compile omitted the COM declarations; adding `objbase.h`
resolved that compile failure. No generated result came from that build.

The first Deck launch placed scratch storage on tmpfs. SLR could not hard-link
across filesystems and began an implicit runtime copy, filling the bounded
private log. The discarded-byte count was not retained before forced containment. The exact test cgroup was stopped before fixture/input admission;
the incomplete owned runtime copy was removed. Preflight now refuses that
filesystem mismatch.

The second launch used the same filesystem but normal Proton `run`, which
entered its built-in steam.exe shim and displayed a Wine C++ runtime error before
creating the fixture. It timed out and was positively contained; it supplies no
input result. Pinned runner source shows `getcompatpath` performs full prefix
setup and then winepath without that shim. The final harness uses that supported
initialization once, then the same `runinprefix` route used by the production
host. No runner files, version, authorized prefix or vendor files changed.

The successful run lasted 20.029 seconds including initialization and cleanup;
this is not an input-latency metric. Fixture and observer exited zero; owned
cohorts and process groups were empty. The isolated compositor also exited zero.
Owned failed-unit status was cleared after retaining its failure receipt. No
UIR1 unit remains. One pre-existing DSP and its environment keeper remain owned
by the ordinary product, which was not stopped. All ordinary publication and
artifact readbacks before/after are identical, and the protected project matches
its retained hash. CPUWeight 10000 remained under SteamOS foreground-booster
ownership; no scheduling setting was changed or performance benefit claimed.


## Bounded pump repair (review 5186994847)

`VendorView::pump` now offers `PM_QS_INPUT` retrieval after at most four
ordinary unfiltered retrievals. Paint and timer filters each receive one
opportunity per turn. The turn retains its 128 queued-message ceiling and has
at most 162 PeekMessage calls. User32 still owns sent calls; a vendor handler
can exceed a wall-time budget, so this is a fairness rule, not a UI deadline.
No message is dropped, collapsed, sent directly to a window procedure, or
special-cased by vendor. The audio path is unchanged.

The first implementation serviced paint/timers each round. It retrieved loaded
Down/Up promptly, but only 4091 posts admitted within the unchanged six-second
fixture bound; it is retained as an incomplete result. Servicing paint/timer
once per turn then passed the same workload: Down/Up at 159/218 handled posts
with all four chains active; all 4096 posts and 64 sends completed. Conservative
issued-to-retrieval upper bounds were 7.358/5.507 ms under load. During 400
additional bounded XTEST motions, 2109 posts progressed and 394 motion messages
were retrieved. OS motion coalescing is not a bridge dropped-message claim.
Paint/timer, quit-code preservation, dispatch-bound and positive cleanup checks
passed. See [repair evidence](../evidence/uir1/repair/differential-after.json).

One host-only Pigments revision-12 ReviewCandidate uses the sealed `uir1_input`
qualification selector and exact ordinary revision-11 parent. Stage, supervised
inspection, physical publication, serving and rollback reuse the existing
owners. The native, module, descriptor, runner and every technical capability
are unchanged. `qualify-ui stage PACKAGE`, `qualify-ui publish` and
`qualify-ui restore` cannot select caller-supplied profiles or binaries. Ordinary
UIO1 admission remains verified-only; `uio1 admit-uir1` independently admits only
the compiled active engineering candidate. Product confirmation is retained below.

## One Pigments confirmation and exact restoration

The single automatic check used the protected UIO1/AP18 project copy, stopped
transport, normal Bitwig launch, and the exact 1280×724 editor. Private capture
confirmed the Macro 1 target on Synth before input. One upward drag and one
Play-tab click were issued through UIO1; there was no host-parameter probe,
operator/Mac takeover, repeated page matrix or audio qualification.

| Action | Down first mouse-hook upper bound | Up upper bound | Result |
| --- | --- | --- | --- |
| Macro 1 drag | 62.053 ms | 19.641 ms | Begin → 0.108, 0.132, 0.156, 0.180 → End |
| Synth → Play | 7.758 ms | 76.423 ms | Play tab and page layout confirmed in private local capture |

Both pairs reached vendor child alias 2, with focus on that child, active parent
alias 1, correct client coordinates, HTCLIENT and downstream hook result zero.
Down preceded Up; capture belonged to the vendor child at Up. Corresponding
PeekMessage retrieval upper bounds were at most 76.725 ms. Macro and main-page
pixel changes were observed within 110.886/111.412 ms of Down. The earlier
navigation-region hover change is not used as page completion. Play's visualizer
continues animating, so no whole-frame stable hash is claimed.

The record contains 230 Win32 records, six VST gesture records and 157 frames,
with zero ring/X RECORD/GUI/frame overflow. Hooks unregistered and mappings
detached; helper/cohort cleanup was positive. Maximum heartbeat during the drag
was 158.961 ms. These diagnostic-on bounds confirm the selected actions became
usable; they are not a universal deadline or a causal rendering measurement.
See [product facts](../evidence/uir1/repair/product.json) and the
[interaction waterfall](../evidence/uir1/repair/product-waterfall.json).

Candidate 12 fingerprint is
`b8d6289d5a50a941deec82b0c3ac644b4c905da684fce8a861f4e847019d11c6`.
Its exact host is
`50c09be65eb2d2930f744afc16212f953c0b948f47132fa6cac1c90bbb91773d`;
native and all technical capabilities remain ordinary revision-11 bytes/data.
The candidate publication `886e48242434864def572a4f5620a9f9` retains ordinary
`bf9d2e4a96e4a7169f6a72bcf0937ebd` as its exact parent. The installed manager
uses source `d752707f30c165562c706d9f34e5b8b9debaabf2`; the retained Windows
artifact remains from source `14e9f913ff4aff441c466bdd3a04786f229fc405` and its
actual synthetic workflow checkout, not the later evidence head.

Bitwig quit normally after declining only the diagnostic edits to the protected
copy. The accepted process-scoped retirement committed all seven milestones (mask 127);
the exact cohort was contained, transport retired and the lease removed. This
is not clean SDK destruction. `qualify-ui restore` physically restored ordinary
Pigments 11; candidate 12 remains inactive. LoFi/FRAGMENTS 10 readbacks exactly
match the before state. The new immutable manager software and prior software
remain retained. Original project bytes and the working copy that preserved
pre-existing unsaved edits are unchanged.

Service/keeper are active, DSP leases and transactions zero, no stale tmpfs,
Bitwig/helper/debugger/diagnostic unit or held input; tracing off. CPUWeight is
unset/effective 100 after normal quit; SteamOS owned its foreground boost during
the functional test. One startup group/512 missing and expired frames remains
in whole-session counters, without attribution or an audio-performance claim.
512 remains selected/supported/recommended and 256 unqualified.

Manager 72+11, report 3, strict Clippy, UIO1 10 and isolation 2 tests passed;
Linux CI covered all 41 runtime tests. All four manager-source workflows passed,
including Windows SDK/editor fixtures and native result-lifetime coverage.
Exact final-head checks are in PR #97. Baseline evidence is unchanged; the first
4091-post repair attempt remains classified incomplete. This repair checkpoint
remained draft until the acceptance transition recorded below.


## UIR1 accepted ordinary revision 13

Review 5187281110 selected head `9f745d75b44b63dd0c095061f1f273189c7a19f8`
and tree `d991280785278e193ed7187fcc45e7dbbb4cd98a`. Root Pigments revision 13
is a new `verified_exact_fixture` profile, equal to candidate 12 except for
revision, claim and accepted evidence. Its fingerprint is
`a74dd61397dcdc9bfcf4a1f39de74eb00f1ca48e3a634b3c03eddc1a416dbfc7`.
Revisions 1–12 remain immutable. Historical ordinary 11 is retained at
`compatibility/ap18/revision-11/arturia-pigments.json`; UIR1 candidate 12 remains
at `compatibility/uir1/arturia-pigments.json` and cannot ordinary-activate.

The no-argument `accept-ui` command uses existing atomic software setup. It
requires the exact restored ordinary-11 publication, completed candidate-12
publication/transaction, review and evidence identities, exact installed
software, inactive DSP and no pending transaction. It adds the accepted host
and source manifest to the immutable ordinary catalogue, retaining the old
host for rollback. No artifact is downloaded or rebuilt by acceptance.
Normal `managed preview` / `managed publish` creates ordinary 13 without a
qualification marker, with exact ordinary 11 as immediate rollback parent.
The immutable acceptance receipt retains candidate 12 as review provenance.
LoFi and FRAGMENTS 10, all native artifacts and the default host remain unchanged.

The one ordinary load/editor/normal-quit smoke passed. The accepted drag,
page, fairness, audio and historical campaigns are not repeated. Process-scoped
vendor retirement remains explicit; no clean SDK destruction, universal UI
latency, renderer or audio-performance claim is added. 512 remains recommended;
256 remains unqualified.


The ordinary transition is installed and verified. Publication
`9ee2c2fc214dfa373f0dcb594831a426` has exact ordinary-11 parent
`bf9d2e4a96e4a7169f6a72bcf0937ebd` and no qualification marker. Manager source
`7c4da2f92af96f5a06c36f7f5be7e2df41eac415` created immutable software revision
`35b58d63ad517675597c274b837ec1363dcca6c7ac0d9020558903c731b1c971`.
The Windows host remains the accepted `50c09be6…91773d` artifact; no host/native
rebuild was used for installation. Catalogue identity, complete artifact hashes,
profiles, ancestry and physical readback are in `evidence/uir1/acceptance/`.

One ordinary instance loaded the protected project and opened its editor. Normal
quit, followed by declining temporary smoke edits, committed retirement mask 127,
joined processing without exception, and retired the exact Windows cohort,
transport and lease. Bitwig frontend/engine/plugin-host are absent. Revision 13
remains active, candidate 12 inactive, service and keeper active, transactions
and stale transports zero, tracing off. Project hashes remain unchanged.
SteamOS retained CPUWeight 10000 after this functional smoke; the bridge did not
modify scheduling and no timing/performance claim is made. One stopped/pre-note
512-frame gap remains under issue #90. No drag/page or other qualification was
repeated. Final-head hosted checks and merge-review status are recorded on PR #97.
