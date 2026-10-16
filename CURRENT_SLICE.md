# Current slice — recover one shared compatibility runtime

Goal: recover the proven runtime recipes into Git and deliver one coherent,
qualified compatibility component set through the normal bridge release.
The old issue queue is paused; this first slice is runtime recovery, not a UI rewrite.

Best explanation: confirmed runtime fragmentation separates working maintainer
setups from what testers receive. Different component compositions and local
recipe paths hide fixes from normal delivery. The live provenance/recovery task
is [#430](https://github.com/kasselvania/Linux-VST-bridge/issues/430); its inventory owns the roster.
Recover the full DComp build/composition recipe and Steamuser policy patch;
BG1 D3D11 coherence source and launcher/content derivation recipe; and WebView2
acquisition/distribution basis. UIA/MSI/touch/certificate recipes already exist.

Boundary: an existing other-system backup is active; restoration is unverified.
Preserve it and the working Deck. No wipe, licensed-state migration or runtime
replacement is part of the governing-docs increment.

Next changes:
1. The #430 recovery owner imports the missing original patches/recipes and their
   provenance into Git, without private vendor data or builds on working machines.
2. Choose the Proton/GE base by combined tests. Build one common engine/component
   set with installer, crash-guard, graphics, touch and certificate corrections;
   typed configuration selects supported paths within that same set.
3. Deliver through runtime acquisition and a numbered package. Retain exact old
   release/setup bindings for continuity/rollback; do not hide vendor engine forks
   inside one archive or silently switch licensed environments.
4. Qualify combined corrections and baseline behavior on disposable CachyOS and
   SteamOS fixtures, then plan explicit adoption for preserved working setups.

First source increment: missing DComp/BG1 recipe recovery. A later common build
entry point under tools/runtime/ feeds bridge-manager/src/runtime_delivery.rs;
experimental_runner.rs is a retained migration route, not the new default.

Done: a clean Git recipe builds the declared common set; normal acquisition and
installation use it; real patch cases plus baseline plug-ins install, play,
save/reopen and survive restart, update and rollback in actual DAWs on both target
fixtures. Preserve class/automation/content identity and licensed state. Record
actual support and failures in the existing matrix/classes; no per-run report.
Profile/status/storage/audio/platform ownership remains a separate review proposal.
