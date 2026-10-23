# Windows DAWs, Max for Live and Push 3 controller mode

> Retired on 2026-10-22 under #504 E3. This document records the former
> FL Studio workspace; it is not current product authority. Existing
> installations, projects, content and ownership records remain in place.

Research checked 2026-10-14. No installations, device changes or authorization
actions were performed. The Test69 SteamOS/CachyOS private beta stays unchanged.

Continue the existing Windows-DAW workspace direction. FL Studio already has a
musical result; its next milestone is licensed project recall. For Push 3, start
from the background component the operator already observed on Windows. Map its
provisioning, launch and Live connection before adapting the public Linux route.
An independent upstream project supplies concrete helper integration and
physical tests; full local acceptance remains unfinished.

## What we already established

The [WD0 record](../../evidence/wd0/deck-trial-2026-09-25.md) and
[WD1 record](../../evidence/wd1/deck-serum-direct-2026-09-25.md) identify the Deck
fixture: **FL Studio 26.1.5.5618 trial, Serum 2.1.5 and
`proton-11.0-2c-fl-crypt32-order-v1`**. The user heard Serum piano-roll playback,
changed presets, used parameter automation and heard playback after a fresh
managed launch. Normal quit retired both sessions cleanly. This is established
use, not merely a window opening.

The selected audio backend, rate and buffer were not recorded. WAV export was
deferred by the user. WD0 retained an FLP file, but neither record establishes
saved-project reopening. Image-Line confirms that its trial permits saving and
exporting but blocks reopening; the earlier report that the trial could not save
must not become a product assumption. [Official trial limits](https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/app_feature.htm).

Repository records contain no local Live or Max for Live acceptance. The earlier
[Push result](../MIDI0_PUSH_NOTE_RELEASE.md) concerns controller-mode MIDI through
Bitwig: raw note-on/off reached Linux, while bridged instruments held notes.
The source correction was not physically qualified in that record. It supplies
no Live display, touchstrip, MPE or audio-interface result; firmware version was
not recorded.

## Architectural boundary and existing licenses

[WINDOWS_DAW_WORKSPACES.md](WINDOWS_DAW_WORKSPACES.md) already selects the
correct path: Windows FL/Live directly host Windows plug-ins inside separate
managed workspaces. Reuse installation supervision, runtime acquisition,
environment identity, process ownership and recovery. The native Linux proxy
and bridge audio transport do not belong between a Windows DAW and its plug-in.
FL and Live need distinct mutable environments; a runner change must preserve
the whole installed workload. Saved projects and authorization state must survive
runtime update and rollback without being rolled back themselves.

Existing licenses should use the vendors' ordinary authorization flows, subject
to their version/edition and installation allowances. Both vendors document
online and official offline routes; Linux success still needs demonstration.
Preserve environment identity and keep the user in control of credentials and
activation files. [Image-Line unlock](https://support.image-line.com/action/knowledgebase?ans=119),
[Ableton authorization](https://help.ableton.com/hc/en-us/articles/209773585-Authorizing-Live-Online),
[Ableton offline authorization](https://help.ableton.com/hc/en-us/articles/360000573444-Authorizing-Live-Offline).
Image-Line acknowledges Wine while declining native Linux support; Ableton's
published desktop requirements list Windows/macOS, with AVX2 required for Live 12
on x86 Windows. Compatibility work does not imply vendor support.
[Image-Line](https://support.image-line.com/action/knowledgebase?ans=1018),
[Ableton requirements](https://help.ableton.com/hc/en-us/articles/115001663530-Live-Minimum-System-Requirements).

## Push 3: recover the background component and its lifecycle

**Operator-observed baseline:** first connecting Push on Windows installs/starts
a Live-associated background component without an explicit announcement from
Live. That is part of the existing behavior to recover. It means some
Live-associated software is present; it does not establish that the component
contains all of Live or works independently of it.

Separate provisioning from execution. Ableton says connecting Push with Live
installed automatically installs its display driver, with a manual installer in
Live's `Redist` directory; launching Live triggers driver reinstallation. The
audio driver is included with Live 11.3+. Neither statement identifies the
operator's background process or proves it is registered as a Windows service.
[Display provisioning](https://help.ableton.com/hc/en-us/articles/10295607009436-Push-3-Display-Driver-Windows),
[audio provisioning](https://help.ableton.com/hc/en-us/articles/8483166334748-Push-3-Technical-FAQ).

The **shibco/ableton-linux** hardware report supplies an actual execution chain:
Live automatically starts Ableton's packaged `Push3.exe`, passing
`--parent-process-id=<pid>`. The helper opens USB, loads a Push script and
handshakes with Live. This establishes a distinct helper process under Wine;
it does not identify the Windows service/autostart mechanism the operator saw.
Its exact installed path, first-connection installation event and lifetime after
Live exits remain readback gaps. [Physical process report](https://github.com/shibco/ableton-linux/pull/152#issuecomment-5311436349).

The inspected [2026-10-03 source snapshot](https://github.com/shibco/ableton-linux/tree/aceb236a81656c609274afbf27fd54eb5017e5e2)
has [prefix setup](https://github.com/shibco/ableton-linux/blob/aceb236a81656c609274afbf27fd54eb5017e5e2/scripts/setup-prefix.sh)
select a built-in libusb bridge specifically for Push helpers. This configures
their USB access; it does not launch a replacement daemon. The
[MIDI patch](https://github.com/shibco/ableton-linux/blob/main/patches/0105-winealsa-make-MIDI-topology-dynamic-and-recover-hotplug.patch)
supplies discovery identity; the
[USB patch](https://github.com/shibco/ableton-linux/blob/main/patches/0106-libusb-1.0-extend-the-host-bridge-for-Push-3.patch)
forwards helper calls into host libusb. The route reuses the vendor helper
without porting its Windows kernel drivers.

The [technical note](https://github.com/shibco/ableton-linux/blob/main/notes/ABLETON-WINE-PUSH3-USB.md)
separates the remaining layers:

| Layer | Responsibility and evidence |
| --- | --- |
| Discovery and MIDI | Wine presents the USB identity to Live, which starts the helper. Linux `snd-usb-audio` owns MIDI interface 5. |
| Helper hardware access | `Push3.exe` claims display interface 0 and xPort interface 6 through host libusb and opens the Live MIDI port. |
| Control integration | The helper loads a Push script and exchanges a greeting with Live. The exact split of controller logic and Live session/device state needs readback; the helper is not evidence of a complete independent DAW. |
| Audio | Linux `snd-usb-audio` owns interfaces 1–4. Reported PipeWire nodes do not establish audible recording or dropout behavior. |

Upstream Live 12.4.3 tests exercised display, pads and encoders on Ubuntu/Arch,
including controller hardware. Latest integrated-patch acceptance remains open;
earlier runs had MIDI feedback/queue errors. Touchstrip, MPE and reconnection
need local checks. Standalone Live inside Push is a different execution path.

The repository declares [LGPL-2.1](https://github.com/shibco/ableton-linux/blob/main/LICENCE),
making source reuse a candidate subject to recording component notices and
distribution obligations. Its [current package requirements](https://github.com/shibco/ableton-linux#requirements)
include PipeWire 1.4.2, glibc 2.35 and active NTSync on Linux 6.14+; its USB setup
uses a system udev rule. Verify the selected machine first. Do not turn those
requirements into silent SteamOS base changes. A separate pinned runtime can be
evaluated without replacing FL's established runner or importing the upstream
installer as another product manager.

## Live and Max for Live need a real Set

Max for Live is a Live execution feature, not a VST3 that our existing proxy can
automatically expose. Suite includes it; Standard requires the appropriate
add-on. Start with Live's bundled compatible Max and the user's entitlements.
[Ableton licensing](https://help.ableton.com/hc/en-us/articles/360001296900-Activating-Max-for-Live-with-a-Live-Standard-license),
[Max runtime selection](https://help.ableton.com/hc/en-us/articles/209070309-Using-a-separate-Max-for-Live-installation).

A device can depend on Windows externals, packages, samples, JavaScript, graphics,
network services or its own authorization. Freezing bundles dependencies; it
does not translate a macOS-only external into a Windows binary. Max's audio/MIDI
passes through Live, and parameter storage must actually participate in saved
Set state. [Cycling '74 dependencies](https://docs.cycling74.com/userguide/m4l/live_freezing/),
[runtime and state rules](https://docs.cycling74.com/userguide/m4l/live_limitations/).
Test one bundled device and one device the user relies on, including its editor,
automation and reopening. Passing the bundled example cannot qualify the whole
library. Upstream supports a Max route but still documents a Live 11 first-launch
repair, so select exact versions instead of assuming interchangeability.
[Upstream limitation](https://github.com/shibco/ableton-linux/blob/main/TROUBLESHOOTING.md#live-11-max-for-live-fails-after-the-first-launch).

## Two discriminating next experiments

Choose their order by available hardware and licenses; Push investigation does
not wait for FL qualification.

1. **Trace Push's helper lifecycle and compare Linux.** If a working Windows
   setup or retained observations are available, record process ancestry,
   command lines, installed helper and
   driver paths, service/startup entries and USB/MIDI ownership across: Live
   closed; ordinary plug-in; Live launch; normal Live quit; reconnect. Use
   retained installer/PnP logs to distinguish first-run installation from later
   launch, without reinstalling to recreate history. Identify what survives
   Live's exit and which display/control/audio functions remain. An available
   Windows baseline is useful, not a prerequisite to create another installation;
   otherwise retain the exact Windows lifecycle as an open question. Compare
   the same sequence in an isolated Linux workspace using the public helper
   route. This decides whether the missing work is provisioning, discovery,
   helper lifetime, USB access or the Live handshake. Preserve firmware and
   stop before any proposed update or permanent host change. Follow with an
   audible saved Set, the two Max devices, and separate Push input/output use.
2. **Finish FL's existing musical journey.** With explicit authorization for the
   user's normal license flow, retain the existing workspace and create a new
   test project containing Serum and automation. Name the audio backend, play,
   export, save, quit and reopen after reboot. Compare the pattern, preset and
   automation, then qualify update/rollback separately. First musician acceptance:
   the same saved song plays correctly without environment administration.

## Opening for the next thread

> Continue the Windows-DAW direction from this note. FL Studio trial with Serum 2
> already played, automated and relaunched on the Deck. Preserve that workspace
> and Test69's SteamOS/CachyOS beta. Push already provisions a background
> component on first Windows connection. Trace installation/startup, parent
> process, hardware access and Live handshake in the public Linux helper route;
> compare available Windows observations without requiring a new Windows setup.
> Do not assume that helper is all of Live. Then establish
> audible saved-Set use with my selected Max devices and finish FL licensed
> save/reopen through normal vendor authorization. Keep accounts, projects and
> activation state private; ask before firmware or permanent system changes.
> Reuse the existing manager and workspace architecture.
