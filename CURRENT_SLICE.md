# UIR1 — bounded Windows input/message fairness

Base: UIO1 merge `ddf256c60d3f770e3f2773724d260ed362a1267f`, tree
`31afc83d0096d73aacbb636e1bd823919402605c`. Review 5186994847 accepted the
UIR1 differential at `77fd8714105f303b202ce61ecff786b317da7a80` and selected
the generic `VendorView::pump` owner. Continue in PR #97, draft and unmerged.

## One claim and completed result

The Windows editor pump gives hardware input regular retrieval opportunities
while ordinary posted, sent, paint and timer work continues. The same generated
pinned-Proton fixture now handles mouse Down/Up while all four posted chains
remain active, and one automatic Pigments drag/page click confirms the repair
in the exact product. This is bounded retrieval fairness, not a wall-time UI
deadline or a Wine-contract violation.

The accepted unfiltered baseline delayed loaded Down/Up at least 4.963/4.875
seconds until the finite posted workload drained. The repaired pump handled
them after 159/218 posts, with four chains still active. All 4096 posts and
64 sends completed. Posted work advanced by 2109 messages under 400 additional
bounded XTEST motions. Paint/timer, WM_QUIT, exact child/focus/coordinates,
zero observation overflow and the 128-dispatch ceiling passed.

The single Pigments confirmation produced Macro 1 Begin → four Values → End
and changed Synth to Play. Conservative issued-to-first-mouse-hook upper bounds
were 62.053/19.641 ms for drag Down/Up and 7.758/76.423 ms for page Down/Up.
Relevant local pixel changes appeared within 112 ms of Down. No repeated page
matrix, live pre-repair run, operator/Mac control, or audio campaign was used.

## Owners and limits

`VendorView::pump` interleaves `PM_QS_INPUT` with at most four ordinary unfiltered
retrievals. Paint and timer each receive one filtered opportunity per turn.
There are at most 128 queued dispatches and 162 PeekMessage calls per turn.
User32 retains sent-message semantics. A vendor handler can exceed a wall-time
budget. No message is discarded, collapsed, synthesized or sent directly to a
window procedure. No vendor-name exception, audio callback or transport change.

The generated comparison uses an isolated headless KWin/XWayland display,
fresh unlicensed prefix, real parent/child HWND, and the actual production pump.
Four chains admit at most 4096 posts within six seconds with an explicitly
synthetic 1 ms handler wait; 64 sends each have a 500 ms bound. The workload is
not asserted to equal Pigments internals. UIO1 supplies exact-window XTEST,
X RECORD, Win32 hooks and conservative Linux/QPC brackets; no internal Wine
admission timestamp is inferred. Private raw frames/identities stay out of Git.

## Candidate and restoration

Pump source: `14e9f913ff4aff441c466bdd3a04786f229fc405`. The retained Windows
workflow checked out synthetic commit `964642bc8567557c579cf5949922b4aaff5b072f`,
with the same tree. Manager/software source:
`d752707f30c165562c706d9f34e5b8b9debaabf2`.

One immutable Pigments revision-12 ReviewCandidate changes only the host/source
binding relative to ordinary revision 11. `qualify-ui` uses the sealed
`uir1_input` identity and existing stage/publication/rollback owners. The exact
ordinary revision-11 parent is required. Ordinary activation and ordinary UIO1
admission remain verified-only; the closed diagnostic selector admits only the
compiled engineering candidate. Native/module/descriptor/capabilities and
LoFi/FRAGMENTS publications are unchanged; no new acceptance mechanism.

After the check, Bitwig quit normally with process-scoped vendor retirement,
positive transport/cohort cleanup and zero DSP leases. Exact ordinary Pigments
11 is physically restored; candidate 12 remains inactive. LoFi/FRAGMENTS 10
match the before readback. Original project bytes and a saved copy retaining
pre-existing unsaved edits are preserved. Service/keeper active; no transaction,
tmpfs session, helper, debugger, diagnostic unit or held input. Tracing off;
CPUWeight unset/effective 100 after quit, without scheduler modification.

## Validation and nonclaims

Manager 72 library + 11 binary tests, report 3 tests, strict Clippy, UIO1 10
tests, isolation 2 tests, Windows SDK/editor fixtures and all four source-head
workflows passed. Runtime tests: 17 pass/24 platform skips on macOS, all 41
covered by Linux CI. All 16 candidate publication/rollback boundaries remain
covered. Final evidence-head checks and exact head/tree are recorded in PR #97.

No generic Wine defect, universal latency, renderer improvement, audio
requalification or synthetic/vendor workload equivalence is claimed. Diagnostic
heartbeat reached 158.961 ms during the drag. One startup 512-frame gap remains
in the whole stopped/pre-note session counters; residual delivery stays #90.
512 remains selected/supported/recommended; 256 remains unqualified. Review is
pending; do not ordinarily activate the candidate or merge this PR here.

See [docs/UIR1.md](docs/UIR1.md) and [repair evidence](evidence/uir1/repair/).
Disposition: `UIR1_BOUNDED_PUMP_FAIRNESS_GENERATED_AND_PIGMENTS_CONFIRMED`.

## Accepted repair: ordinary transition authorized

Review 5187281110 accepts exact head `9f745d75b44b63dd0c095061f1f273189c7a19f8`
and tree `d991280785278e193ed7187fcc45e7dbbb4cd98a`. The only remaining work
is immutable Pigments 13 ordinary authority from candidate 12, using existing
setup/catalogue/publication owners and exact ordinary-11 parent. Preserve all
technical constraints, prior profiles, artifacts and evidence. No new pump,
renderer, audio or input testing. One ordinary load/editor/normal-quit smoke;
leave 13 active on success and return the same PR for final merge review.
