# Shared X11 touch-routing successor (experimental)

This directory contains original source and a patch against the exact
candidate-C Wine lineage. It contains no Proton, Wine, Serum, or compiled
binary. Do not alter the installed candidate-C runner in place.

The action-bound Deck observation is summarized in
[`candidate-c-action-capture.json`](../../evidence/serum-x11-touch-routing/candidate-c-action-capture.json).
The complete private traces remain on the fixture and are not distributable
evidence. The selected source boundary is root-level raw-touch delivery:
after a touch-opened popup became visible with capture, the old editor child
continued receiving `WM_POINTERUPDATE` with `INCONTACT`. The trace did not
retain the complete touch Begin/End sequence, so the patched physical run is
the causal test.

Apply `wine-x11-touch-release.patch` from the preceding slice to a private
checkout of Wine `dc26e61847081a1b5cb0733dc30feba6ee575482`, then apply
`wine-x11-window-touch-v2.patch`. The latter selects XI2 Touch Begin/Update/End
on each actual Wine X window instead of selecting a second root-level raw-touch
stream. It maps `XIDeviceEvent.event` through Wine's registered X window
context to the target HWND, retires per-window selection even when raw mouse
input is active, and keeps candidate C's released-contact correction.
The reference for per-window XI2 selection and event-relative coordinates is
[upstream Wine `mouse.c` at `1977760e`](https://github.com/wine-mirror/wine/blob/1977760e3745c58ee9c2b8aeb93e54e5fe7f14e2/dlls/winex11.drv/mouse.c).

`test_touch_flags.c` tests the production mapping header against the pinned
offline SDK. `popup_fixture.c` is an original Win32 physical-input fixture:
it has an ordinary control, owned transient popup, pointer and compatibility
mouse counters, capture/focus logging, and a heartbeat. It never synthesizes
mouse or touch input. Its exit condition requires balanced Down/Up, no released
contact, no retained capture, and at least two popup opens and closes. The
human operator must exercise both mouse and touchscreen; an executable pass
without that physical sequence is not acceptance.
`run_popup_fixture.py` launches it only from the sealed successor tree in an
isolated unlicensed prefix and waits for the operator to close it. It never
injects input, and its private result contains only exit and digest facts.

Build Wine with the offline Steam Runtime 4 SDK digest
`sha256:97526b794ce1a9bed5f891084462260b3a02399569f7438a3a57b5a253001db9`
using the prior full reference-build configuration and an isolated install
prefix. `seal_candidate.py compose` verifies the exact candidate-C environment
and the patched source, copies its immutable runner to a new private tree, and
replaces only `version` and x86-64 Unix `winex11.so`. `seal` runs the mapping
test and an unlicensed Windows command in an isolated prefix, verifies exact
cleanup and unchanged tree bytes, and writes a private manifest for the
existing `experimental-runner` owner. It does not advance the environment.

Candidate D, if materialized, remains an engineering `review_candidate` with
candidate C as exact predecessor. A built runner, fixture result or published
candidate does not establish a physical Serum touch repair. Only the bounded
human-operated Serum session and complete retirement can close FC-UI-003.
