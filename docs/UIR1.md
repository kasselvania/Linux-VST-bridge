# UIR1 — generated input-retrieval differential

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

`tools/uir1/windows.cpp` calls the unchanged production pump and admits no vendor
binary. Its status is fixed scalar data in a private mapping. The UI thread
counts exact posted, sent and mouse handling; UIO1 independently observes mouse
retrieval and heartbeats. The companion producer has finite count/time limits.
At normal completion every submitted message must have been handled and both
windows must be destroyed. Hosted Windows tests verify that law before the
pinned-Proton differential.

The generated result below selects a repair direction; no production repair is installed.

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
work is continuously available. This PR supplies the reproducible baseline and
selects that owner. It does not yet change the product pump or install a repair.

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
the compiled active engineering candidate. Product confirmation remains pending.
