# HP1 Bitwig admission tooling

This directory owns the bounded, operator-assisted HP1 session. The tools do
not launch Bitwig, invoke the official validator during either graphical
session, rebuild or republish the HP0 probe, change a Flatpak override, alter a
Bitwig plug-in location, save a project, or start Wine, Proton, UMU, or
yabridge.

## Exact inputs

HP1 accepts only the repository basis, Steam Deck/SteamOS fixture, Bitwig
6.0.11 app and Freedesktop 25.08 runtime commits, Flatpak override bytes, HP0
publication and receipt, HP0 build-source manifest, official validator, and
Serum preservation identities fixed in `CURRENT_SLICE.md` and
`monitor-instance.py`.

Before any HP1 tracked edit, the operator agent ran the accepted HP0
publication inspection, final-head build-receipt verification, and exact
Bitwig app-sandbox validator preflight. Their raw key/value readbacks remain
outside Git beneath a canonical user-cache directory and are required by the
HP1 preflight. The accepted build fixture is:

```text
<REPO>/build/hp0-second-repair-2
```

## Session route

From the exact Steam Deck checkout:

```bash
tools/hp1-bitwig-admission/begin-session.sh \
  --build-dir /home/deck/code/Linux-VST-bridge/build/hp0-second-repair-2 \
  --preedit-proof /absolute/path/beneath/the/user/cache
```

`begin-session.sh` creates a cryptographically random session ID and nonce,
runs the exact fixture capture and the 17 synthetic negative cases, takes the
bounded Bitwig-owned state baseline, starts the process/module monitor, and
prints `HP1_OPERATOR_ACTION_REQUIRED`. Raw state is confined to:

```text
$XDG_CACHE_HOME/linux-vst-bridge/hp1/sessions/<SESSION_ID>/
```

or, when `XDG_CACHE_HOME` is unset:

```text
<HOME>/.cache/linux-vst-bridge/hp1/sessions/<SESSION_ID>/
```

Every dynamic root is absolute, canonical, below the real user cache, and has
no symlinked component. Synthetic fixtures use temporary siblings beneath the
same cache and are removed. The live monitor identifies the named Bitwig entry
process, walks upward only through the same UID and exact Flatpak app-scope
cgroup, and uses that highest verified ancestor as the application-tree root.
This admits a plug-in host that is a sibling of the named Bitwig process beneath
the same wrapper while still requiring real Unix ancestry, identical app scope,
and identical UID. It records bounded process identity and ancestry, the one
exact matching `/proc/<pid>/maps` entry, and named forbidden process classes.
It never retains command lines, environments, or a complete process map.

Monitor progress without changing the session:

```bash
tools/hp1-bitwig-admission/monitor-instance.py status \
  --session-id <SESSION_ID>
```

If status prints `operator_action=quit_bitwig_normally_now`, quit Bitwig
normally at once and do not continue the admission checklist. The monitor stays
active until the Bitwig tree, observed descendants, exact mapping, validator,
and forbidden process are all gone, then retains the contaminated blocked
state and clean-exit result.

The monitor polls every 250 ms for at most four hours. Bitwig-owned readback is
limited to seven declared roots, same-filesystem traversal, depth 2--5, 256
candidate files, 4 MiB per file, 16 MiB total content, and 15 seconds. A
timeout, candidate cap, byte cap, inaccessible file/directory, symlink, or
other incomplete contributor becomes `unknown/search_incomplete`; it cannot
be converted to absence or success.

## Operator GUI checklist

Do this only after the tool prints the checkpoint and while its monitor reports
`waiting_session_1`.

Session 1:

1. Launch Bitwig normally from its existing desktop/application entry. Do not
   use a terminal command, modified environment, or alternate Flatpak launch.
2. Wait for ordinary startup activity to settle and search the device/browser
   surface for `LAB Host Probe`.
3. If absent, perform at most one ordinary built-in Bitwig plug-in rescan,
   without adding or changing a plug-in location, then search once more.
4. If still absent, change nothing, quit Bitwig normally, and use the blocked
   confirmation below.
5. If present, verify vendor `Kasselvania Research` where exposed. Create a new
   unsaved temporary project and blank audio track; insert exactly one probe.
6. Leave it active until the monitor reports `session_1_mapping=captured`.
   Optionally observe whether the generic device surface exposes Gain and
   Bypass; this is supporting evidence only.
7. Remove the instance and quit Bitwig normally without saving. Wait for
   `session_1_shutdown=clean` and monitor state `waiting_session_2`.

Session 2, only after Session 1 succeeds:

1. Launch Bitwig normally again and do not rescan.
2. Search for the probe, insert exactly one instance on a blank audio track,
   and leave it active until `session_2_mapping=captured`.
3. Remove it and quit normally without saving. Wait for
   `session_2_shutdown=clean` and `completed_two_sessions`.

Reply in the same agent thread with exactly one line:

```text
HP1_GUI_CONFIRM <NONCE> SESSION1_FOUND INSERTED REMOVED QUIT SESSION2_FOUND_WITHOUT_RESCAN INSERTED REMOVED QUIT GAIN_GENERIC_VIEW=<OBSERVED|NOT_EVALUATED> BYPASS_GENERIC_VIEW=<OBSERVED|NOT_OBSERVED|NOT_EVALUATED> HOST_BYPASS_BINDING=UNKNOWN
```

or, for the bounded absent result:

```text
HP1_GUI_CONFIRM <NONCE> NOT_FOUND_AFTER_ALLOWED_SCAN QUIT
```

`BYPASS_GENERIC_VIEW=NOT_OBSERVED` means no separately labeled Bypass parameter
was visible in the generic parameter surface. It does not mean bypass is absent:
the probe declares one VST3 parameter with `kIsBypass`, and Bitwig may represent
that through host-owned device controls. HP1 performs no parameter-change or
DSP test, so `HOST_BYPASS_BINDING` remains `UNKNOWN` in every success receipt.

The collector accepts no paraphrase, wrong nonce, omitted step, reused nonce,
unsupported parameter combination, or success without two complete
mapping/shutdown receipts. Operator transport may use either literal
underscores or uniformly Markdown-escaped `\_` sequences. Mixed forms and
every other escape are refused. The receipt retains distinct hashes for the
original transport and the normalized semantic confirmation.

After an accepted operator line, the agent supplies it on standard input to:

```bash
tools/hp1-bitwig-admission/collect-session.sh \
  --session-id <SESSION_ID> \
  --build-dir /home/deck/code/Linux-VST-bridge/build/hp0-second-repair-2 \
  --preedit-proof /absolute/path/beneath/the/user/cache

tools/hp1-bitwig-admission/sanitize.sh --session-id <SESSION_ID>
```

## Claim ceiling and cleanup

Successful HP1 evidence proves only exact native class discovery and instance
admission in two normal launches of the pinned Bitwig fixture. A bounded absent
result is `HP1_HOST_PATH_BLOCKED`. Neither result proves audio/DSP correctness,
automation, project state, custom UI, Serum operation, Windows hosting, bridge
operation, performance, crash recovery, another DAW, or general Linux support.

The operator quits Bitwig normally without saving. No tool deletes application
state or the accepted HP0 publication. Raw cache sessions are not tracked and
may be removed manually only after the committed evidence and PR are verified.
