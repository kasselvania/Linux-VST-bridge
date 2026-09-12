# UIR1 — XWayland-to-Win32 input retrieval latency

Base: UIO1 merge `ddf256c60d3f770e3f2773724d260ed362a1267f`, tree
`31afc83d0096d73aacbb636e1bd823919402605c`.
The operator selected this slice after review 5186750570 accepted and merged
UIO1. [UIO1](docs/UIO1.md) remains immutable evidence: exact X11 delivery preceded
first observable Win32 hardware-mouse retrieval by at least 8.237357339 seconds.
It does not identify an internal Wine admission or queue timestamp.

## One claim

Determine whether the retrieval delay can occur with generated parent/child
windows and the production unfiltered `VendorView::pump`, on the exact pinned
Proton runner, without a vendor plug-in. Compare an idle queue with a declared,
finite posted/sent-message workload using the existing UIO1 XTEST, X RECORD,
thread-scoped Win32 hooks and bounded cross-clock brackets.

## Scope and decision

Development fixture, private isolated headless KWin/XWayland display, fresh
unlicensed scratch prefix, exact existing runner and diagnostic helpers. No
operator desktop input, vendor session, authorization, project, product host,
profile or publication mutation. No audio callback instrumentation.

An idle failure selects generic Wine/XWayland or pump integration. A load-only
failure selects queue/pump fairness as the generated mechanism, without claiming
that the same traffic existed in Pigments. If both work, the remaining direction
is the real vendor child-window/focus/capture interaction; no vendor test before
interpreting the fixture. No production repair before the fixture selects an
owner. Never discard messages or implement a product-name exception.

## Bounded verification

Use actual production pump code. Retain exact target/child, X11 receipt, first
Win32 retrieval, heartbeat, traffic submitted/handled, overflow and clock
uncertainty. Four posted chains admit at most 4096 messages over at most six
seconds, each with one explicitly synthetic 1ms handler wait. A separate producer
sends at most 64 messages with 500ms bounds. These are generated conditions, not
measured vendor workload. The fixture's overall lifetime is at most 60 seconds.

Run Windows fixture and affected tooling/CI checks. Preserve failed harness
attempts separately. Private process identities and mapping contents stay out of
Git. Public evidence contains aliases and scalar summaries. Positive containment
must retire the scratch runner, observers and compositor; no held synthetic
input or transient unit. Existing product sessions, service and keeper remain
untouched. One UIR1 PR, unmerged for review. 512 remains recommended; 256 remains
unqualified. No audio, renderer, general Wine or product performance claim.

## Completed first differential

The generated idle/loaded/idle comparison passed with exact cleanup. Idle mouse
retrieval upper bounds were at most 4.126 ms; loaded Down was delayed at least
4.962809666 seconds after X11 receipt while heartbeat latency stayed at most
6.1079 ms. All 4096 posts and 64 sends completed. This selects generic
message-pump fairness for the next repair, without proving the specific internal
traffic of Pigments. No vendor session or operator desktop input was used.

See [docs/UIR1.md](docs/UIR1.md) and [evidence/uir1](evidence/uir1/). The generated
fixture and owner-selection result are complete; production pump repair remains
separate, unimplemented work. Accepted products and publications are unchanged.
Disposition: `UIR1_GENERATED_RETRIEVAL_STARVATION_REPRODUCED`.
