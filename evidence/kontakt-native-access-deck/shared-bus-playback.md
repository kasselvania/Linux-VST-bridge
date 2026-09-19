# Kontakt shared buses and first Bitwig sound test

On 2026-09-19, Kontakt 8 Player 8.13.1 played the installed Factory Selection
1.4.2 instrument **Pad - Noir** in Bitwig 6.1 on the Steam Deck. A complete
Bitwig close/restart restored the instrument from the saved project, and a
second MIDI playback produced audio. This continues the installation result in
`README.md`; the earlier preparation refusal remains historical evidence.

## Delivered implementation

Source `9d47606251e4522c7b4f9b549f4fb71cedb16960`, tree
`2eda2828508f0523f56cd10bfd861a4c3739a7b4`, contains the full shared output
implementation from `f5342e8` and the editor lifecycle repair. It is installed as
an experimental candidate, not an ordinary qualified publication.

- Preserve all declared SDK bus indices and metadata, including Kontakt's 32
  stereo audio outputs and event input/output. Bitwig actually requests output
  31; the earlier first-stereo-only attempt could not run this fixture.
- Protocol 1.13 selects shared mapping layout 2: two input planes, 64 output
  planes, 68,176 bytes. The Windows host still reads protocol 1.12/layout 1.
  Each active output has distinct SDK buffers and transport planes. Guards,
  finite samples, unused tails and silence flags are checked on every plane.
- One preallocated, pre-touched native slot pool owns additional output planes
  until callback delivery/discard. It costs 4 MiB per additional stereo output,
  up to 124 MiB for 32 outputs; a single-output instance needs no extra pool.
  The callback does not allocate/free this storage. This is a bounded first
  implementation, not a many-instance memory-efficiency result.
- Initial Windows transport readiness follows real component initialization.
  Complete bus setup uses the correct bounded C ABI size. SDK strings terminate
  at their first bounded NUL; unused String128 tails need not be zero.
- An editor may defer its size until platform attachment. Initial `getSize`
  failure leaves the hidden parent available for attachment; a valid size is
  still required afterward. `resizeView` delivers `onSize` when the previous
  size is unavailable. These are shared SDK-boundary repairs, not executable
  Kontakt profile exceptions.

The installed NI environment, corrected pinned Proton runner, Windows machine
identity, Native Access installation and other published profiles were retained.
No replacement installer, fabricated plug-in state or substitute DSP was used.

## Exact candidate and build

| Item | Identity |
| --- | --- |
| Candidate | `2c05013dea1a854461cacc5862c1184734efd5bc29a435059224f41be5fa9eaf` |
| Publication | `564b509da2fc7bcf7ef91d2d6da31a1d` |
| Publication SHA256 | `2d982e146a000be3b59b2f00e9af4c9b1867b7ce2917bd1d6a0d4411a222ac50` |
| Preparation kit SHA256 | `cd8824ac2c7108f5f0645f744934776a73fb848b5965d3d709b95ecdba1dcfba` |
| Native proxy SHA256 | `6d55fee01d8b6951648f06d9158de3a736cf53fcf414ee98837b99c6ac23cf41` |
| Windows host SHA256 | `3c88c89e581cba71663ee29940848ad733501bc845cc27569ba8114cca1168c2` |
| Host source manifest SHA256 | `6a246b87df99019df41f9d42389ded147745d214097bbf2a119a6d65ed636571` |
| Windows CI / artifact | `35433398104` / `10582230988` |

The Windows artifact's synthetic merge source
`68e3a7b8f5c6393d4bc91ed3e96ab24ac94f3186` has the same tree as the source above.
The existing packager verified host source hashes. Candidate native/host hashes
were read back on the Deck. Normal reinspection, preparation and exact candidate
replacement completed through the manager; no publication database was edited.

Windows CI passed the existing host/editor/bus checks, including the extended
deferred-size view and distinct 32-output buffers. Linux native CI, manager CI
and policy checks also passed. The Deck's native SDK tests passed 2/2
(`ap18-auxiliary-input`, `ap10-result-lifetime`). Rust checks cover 64-channel
mapping, last-plane overrun refusal, distinct output delivery and slot release.
The existing backend suite passed serially (71 passed, one ignored before the
additional focused checks); an initial parallel run hit its existing eight-slot
test registry limit. An intermediate editor build failed C++ enum inference and
was corrected with explicit `Steinberg::tresult`. Neither failure is a live pass.

## Real playback and recall

Fixture: Steam Deck Galileo, Bitwig Flatpak 6.1/runtime 25.08, 48,000 Hz and
512 host frames; 256 transport frames and 512 reported bridge-latency frames.
Exact installed module/runner identities remain in the installation evidence.

The owned `KontaktStereo` project contains a 16-bar, 120 BPM MIDI clip. The
actual Kontakt browser loaded **Pad - Noir** from Factory Selection. The vendor
editor showed the instrument and active voices; Bitwig showed output meters.
No instrument had to be manually reloaded after reopening the saved project.

Each capture used `pw-record --target 0`, with only Bitwig's `out1` and `out2`
explicitly linked to the owned recorder. No microphone or unrelated application
was captured. Both float32 WAVs contain exactly 2,160,000 stereo frames (45 s).

| Observation | First loaded instrument | After full Bitwig restart |
| --- | --- | --- |
| Session | `d028a7f533c489ae9c148ee4bb2c723f` | `881f417639762d6b07da225977706fe6` |
| All samples finite | yes | yes |
| Nonzero samples | 3,297,856 | 3,298,880 |
| Peak | 0.04013851285 | 0.04013851285 |
| RMS | 0.00653741697 | 0.00653834200 |
| Callback rejections / discontinuities | 0 / 0 | 0 / 0 |
| Whole-session underrun frames | 170,496 | 512 |
| Whole-session underrun gaps | 3 | 2 |
| Editor opens / closes / failure | 2 / 2 / 0 | 1 / 1 / 0 |

The reopened instance applied a 135,686-byte opaque state and visibly restored
the same instrument. The saved project is 119,281 bytes, SHA256
`5f6094ad6e52e09edfe755dcc114a58b15497c99caf6bd79f51f83d58151b453`.
It was unchanged after the recall session. These are instrument recall and
measured audio results, not bit-identical state serialization or listening QA.

`pw-record` returned status 1 after the exact requested frame count in both
captures; its logs contain only the output filename and no error. The complete
WAV contents were independently parsed and measured. A zero recorder exit is
not claimed. Original recordings, projects and session files remain private.

## Failure history and remaining limits

`shared-bus-attempts.sanitized.json` retains all seven actual loads: startup
readiness timeout, controller string refusal, the diagnostic repeat, unsupported
output-31 activation, empty-instrument processing with editor-size refusal,
instrument playback, and successful project recall. Failed implementations were
not relabeled as passes.

The fifth empty-instrument run had 486,912 underrun frames. The first loaded
instrument session had 170,496 across its whole lifetime; the reopened session
had 512. This does not establish dropout-free startup, deadline/timing
qualification, or performance at other buffer sizes. Musical output was tested
on the main stereo pair; independently routing instruments to every auxiliary
output remains untested, though actual Kontakt accepted its complete layout and
the deterministic fixtures exercised distinct planes through output 31.

Initial editor foreground activation was refused. Selecting the actual Kontakt
window in the Deck taskbar made it usable. Broad focus/embedding behavior is not
qualified. The native callback reports clean retirement; Windows reports no
session error, confirmed cleanup and retired transport, but raw exit `-15` and
no normal vendor-retirement receipt. Graceful SDK/process retirement is not
claimed. There is no machine-reboot, long-session, many-instance, automation,
all-library, full-Kontakt-license or general Linux qualification.

## Trying the installed result

On the Deck, open Bitwig normally and choose **KontaktStereo** from Recent
Projects, then press Play. The saved project is at
`~/Bitwig Studio/Projects/KontaktStereo/KontaktStereo.bwproject` and already
contains Pad - Noir. The Kontakt device's window button opens the real editor;
select its taskbar window if it remains behind Bitwig. A new instrument track
can also insert **Kontakt 8** through the normal device browser.

The working experimental publication is retained. Bitwig and the owned recorder
were closed after testing. The MIDI-only baseline, saved instrument project,
original session reports and recordings are preserved privately under the owned
`kontakt-bus-playback-20260919` work directory. No unrelated project was changed.
Use the manager's existing candidate replacement/disable operations for rollback;
do not recreate the licensed NI prefix to repair an audio-bus or editor problem.
