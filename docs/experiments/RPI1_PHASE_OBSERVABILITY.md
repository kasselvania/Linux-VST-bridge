# RPI1 causal phase observation — source slice

Base: `6d55d343f2d273cbd3340925bd70b2f135b46c21`, tree
`9275dc53a911984279473db1771c52a0ca1ab1a0`. This isolated RPI1
experiment remains in draft PR #139. It does not change the active ordinary
repository slice.

## Claim and boundary

The next cooled Pi run can produce a bounded Pi-monotonic timeline that separates
JACK entry, native request publication, bridge-worker pickup, host send/reply,
result presentation, editor window damage, scheduling, and thermal clocks. The
installed editor-closed smoke below qualifies the callback readout and phase
capture only. Editor testing remains paused until compatible active cooling is
fitted and observed working.

The `rpi1-observe` feature is selected only by the RPI1 standalone build. Its
two preallocated 65,536-record SPSC rings have separate producers: the JACK
callback and the native bridge worker. The callback performs fixed writes and
monotonic clock reads only; it does no formatting, allocation, locking, file,
process, or network work. A third ordinary thread drains the rings into a
60-second or 65,536-record bound, whichever is reached first, and writes a
private `0600` JSONL file on the
first gap/fault, at editor lifecycle markers, at most every five seconds during
repeated gaps, and at retirement. Ring drops are counted. A sudden machine loss
before an incident flush can still lose buffered observations.

The queue **does not wait** inside the callback. The new records therefore
measure slot inspection and publication, and distinguish queue-full refusal
from a delayed bridge-worker pickup. They do not fabricate `request_wait` or
`reply_wait` phases. Worker timestamps distinguish preparation, send, reply,
validation, and result publication. The returned Windows `process_ns` is a
duration supplied by the existing protocol; it is not treated as a Pi clock
timestamp. `jack_get_cycle_times` and `jack_get_time` use JACK microseconds to
measure entry lateness; the ring's Linux `CLOCK_MONOTONIC` timestamps are kept
separate. The existing AP9/AP16 report remains available as independent
aggregate/retained-gap evidence.

`rpi1/health.py` keeps the one-Hz health record and emits separate `RPI1_THREAD`
records for up to 256 exact PID/TID/start-time identities. Each includes
`schedstat` runtime and runqueue-wait deltas, switches, faults, CPU, policy,
priority, and allowed CPU list. It covers the RPI1 cohorts and named
display/wineserver processes; the callback TID comes from the ring for exact
correlation. Thread names are retained without guessing an unnamed Wine UI
thread's role. Arm/V3D clocks, CPU governor/frequency policies, cooling-device
state, and CPU PSI extend health. `--phase-directory` enables a five-second
0.2-second burst after private phase-file changes or throttle transitions;
ordinary sampling remains one second. `threads_omitted` prevents a capped
sample from masquerading as a complete roster.

`rpi1/xdamage_observer.py` attaches only to the existing `:1` display. It
requires one exact `Pigments` title, emits the window ID, map/unmap/destroy and
focus events, and summarizes X Damage notifications once per second. The host
now prints `target_x11` with the existing editor lifecycle; the two IDs must
match before the damage record is attributed to the editor. Discovery is
polled at 250 ms, so its first *observed* damage is not necessarily the first
ever repaint. A missing damage notification alone is not proof that Pigments
did no rendering.

`rpi1/timeline.py` merges only Pi-monotonic records from the private phase,
health/thread, X Damage, and optional AP16 native files. It derives callback
duration, JACK entry lateness, request-publication and worker-queue delay,
send/reply duration, Windows process duration, deadline counts, and the longest
consecutive missing-result callback run. It does not align Mac wall-clock events
as if they were Pi timestamps. Its output must remain private until scrubbed.

## Pinned diagnostic probe

Read-only probing of the exact installed RPI1 pins found Box64 commit
`2f130fab1d6e1a4ee8a71dc60cfdfcc839ad192a` and binary SHA-256
`79cdd30e5480f5dfb8cd717af99d0e5a89eb55b31701de6fde7896a6a0e79ab6`.
That source's `env.h` declares `BOX64_ROLLING_LOG`, `BOX64_SHOWSEGV`,
`BOX64_SHOWBT`, and `BOX64_DYNAREC_PERFMAP`; the installed binary contains
all four names. The pinned Proton script SHA-256 is
`787504a79bacf6b303984a9a846cf47463f36599248e8306b26d5faf78267aad`;
its source reads `PROTON_LOG` and `PROTON_LOG_DIR`. This verifies presence of
the controls, not the behavior or overhead of a live diagnostic run.

The native unit may set `LVB_RPI1_DIAGNOSTICS` to a comma-separated subset of
`box64-crash`, `box64-perfmap`, and `proton-log`. Unknown/duplicate values
refuse. Default is empty; none of the modes is enabled by this slice.
`proton-log` creates a private per-session directory. Its logs can contain
sensitive vendor/environment details and are not publishable without review.
The selector does not itself forward arbitrary `BOX64_*`, Proton, Wine, or
shell strings. The systemd user manager's ambient environment and any Box64
rc files still need to be inventoried in the physical receipt; this source
change does not claim they are absent.

## Validation and next physical gate

Cross-target AArch64 Linux source checking and focused Rust/Python tests are
the source gate. The idle Pi returned Arm/V3D clocks and thread data, with no
sysfs cooling device; the X Damage extension accepted an observer on the
existing display and the observer retired.

## Installed editor-closed readout smoke

The exact source commit `a38599f82d9f2c3fc8efe2a1f74a730f33818ddd`
(tree `49eb39acd29b979badac33137035f10080acaeed`) was transferred to a new
private Pi source directory. A checksum comparison of every changed file found
no byte difference. Native offline release build produced an AArch64 executable
with SHA-256 `4d69b1c8167f93882be222df26750ebe772eb9fbe872b1775e29c2f215e05a80`.
The prior installed executable, SHA-256
`0531f8bef318dfcb2d50bdace0e9e1a650c3c23e4097ea418a77db2dc554d427`,
was copied into a private rollback directory before the candidate replaced it.

The first service start refused before JACK/Pigments because its private
Xauthority hash no longer matched the current `:1` authority file. The refusal
was retained. That file was current-user-owned, mode `0600`, and successfully
authenticated an X11 root query. Only the private configuration's Xauthority
digest was refreshed, with the previous configuration backed up; no graphical
policy or runtime source was changed.

The repeat started the exact Pigments host with the editor closed and no MIDI
or audio routing test. A live `status` read at 1,539 callbacks reported zero
xruns, deadline misses, process failures and terminal bridge fault. One
startup missing-result episode began at expected position zero; the final
callback total was 3,267 with 3,072 missing frames in that startup episode.
The `mark` command flushed private phase evidence, and ordinary `quit` yielded
all 127 retirement milestones and `RPI1_CLEAN_SHUTDOWN`. The unit, translated
cohort, JACK ports, command FIFO and health observer then retired.

The private phase file is mode `0600`, with 92,073 phase records, both producer
domains, one gap marker, zero fault markers and zero SPSC ring-drop records.
The first gap caused an early flush. The 65,536-record control history then
retained the last part of the run; callback sequences 7–201 are absent from
the captured middle, a **capture hole**, not an additional audio gap. The
updated timeline reporter exposes such holes explicitly. Under this observed
event rate, the bound retained about 33 seconds of callback records, so the
requested 30–60-second window is at its lower end. No physical result is
claimed for a longer editor session.

During the Pi-monotonic phase interval, 88 health samples recorded 55.1–60.05°C,
4.92048–5.0585 V, zero active voltage/thermal bits, up to 107 sampled threads
and no omitted threads. The completed merged report measured 3,072 JACK
entries with maximum entry lateness 98 µs, 6,144 request publications with
maximum slot-inspect-to-publish time 11.5 µs, and zero callbacks over one JACK
period. The phase file and its private merged report remain on the Pi; only
sanitized results are retained in the repository. These figures are an
editor-closed idle-processing baseline, not editor responsiveness, clean audio,
or sustained thermal qualification.

After fitting active cooling, verify cooler state and no active thermal flag
under a short warm-up. Then run the single attended campaign from the tech-lead
assessment: headless baseline, editor mapped idle, one interaction, close and
reopen, then normal retirement. Stop at the first non-startup gap and preserve
the phase, thread, X Damage, health and existing native/Windows evidence.
Do not change JACK periods, bridge delay, vendor quantum, Box64 mode, runner,
affinity, priority, screen sharing, state, or audio routing in this slice.
