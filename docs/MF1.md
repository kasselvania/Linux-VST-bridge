# MF1 — Human Operator Manager

The native Rust desktop app projects the installed manager's version1 operator
model. Eframe0.36.2/egui with Glow provides the initial X11/XWayland frontend;
there is no browser, WebView, network server or frontend state database. The
bounded Deck toolkit fixture launched from Applications, accepted a large-button
click and keyboard text/Tab focus, reported scale1.00 and closed positively.
This is a desktop mouse/keyboard proof, not physical touchscreen qualification.

The frontend calls only the fixed installed manager entry point with `operator
snapshot`, `operator activity` or `operator request`. Requests are bounded typed
JSON; there is no command/path/PID escape hatch. The manager refreshes and checks
the offered action against canonical state. Software and registry identity bind
the request token; publication and environment owners independently revalidate
their own locks and identities at mutation. Review candidates never become
ordinary actions. A retained ordinary revision is offered for rollback only when
it is an ancestor of the current physical publication. Recommended restoration
uses the same installed profiles, census, native catalogue and publisher as CLI.

Heavy exact readback runs on opening, manual refresh, an operation result or a
live-instance transition. A separate lightweight readback reports current leases,
transaction presence, transport and operation receipts. Neither UI nor manager
control code enters the DAW callback. UI close does not cancel operations.

Closed mutations run in manager-owned systemd units, independently of the UI.
ASC open retires the keeper through normal service stop after checking inactivity
under the admission lock. Its existing dedicated vendor unit retains process and
download ownership. After positive vendor retirement the operator owner resumes
the service and its exact registered keeper. Interrupted handoff is retained in a
resume receipt and can be reconciled; an unresolved vendor cohort never authorizes
service resumption. Stop targets only the registered ASC unit. Focus requests are
bound to its current operation and exact main executable/process identity; the
vendor supervisor makes an ordinary EWMH activation request and retains the result.
It does not select a window by title or inject mouse/keyboard events.

Environment rescan reuses the bounded VST3-root traversal and the supervised SDK
module-inspection route. Every factory class is inventoried, including module,
class, role, vendor, version and environment identity. An empty class selector is
the existing explicit module census. If subsequent component inspection refuses
a multi-class module, the complete factory census and that separate refusal are
retained. This does not qualify the component or authorize publication. Failed or
incomplete censuses are quarantined; unchanged failed bytes are not executed on
every rescan. Changed or missing inventory bytes are shown as needing attention.
Existing exact ordinary products remain bound to their accepted bytes.

Crash capture delegates to CA1. Account state remains uninspected. Incident display
and export use CA1's sanitized projection, not raw vendor logs. Exports go to the
manager-owned exports directory. The existing immutable software installer can
include the native frontend; its digest participates in the software generation,
and setup installs an owned application-menu entry. Previous software remains
available. No product native/Windows binary or profile is rebuilt for MF1.

Remaining verification and real-workflow results are recorded in this PR as they
complete. MF1 makes discovery operable; it does not automatically qualify or
publish newly installed products, replace ASC, handle vendor credentials, or
claim universal Linux/Wayland support. 512 remains recommended;256 unqualified.
