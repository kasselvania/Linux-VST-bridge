# Game Mode: preserve Bitwig audio and plug-in behavior

Research proposal, 2026-10-14. **Bitwig already launches in Game Mode.** The
operator reports audio settings changing on entry; the problem is system audio
and video policy affecting plug-ins. The first job is to reproduce that change
with the same project and fix the responsible boundary. Shortcut creation is
not a work item.

## What the installed Deck establishes

Read-only inspection verified `steamdeck`, Galileo, SteamOS 3.8.16, currently
KDE Wayland; PipeWire 1.6.4, WirePlumber 0.5.14 and gamescope 3.16.23.4. No mode,
application, route or setting was changed. There is no paired Game Mode readback
or reproduced failure yet.

The installed Valve scripts identify concrete differences:

| Boundary | Installed behavior | Implication |
|---|---|---|
| Audio controls | `Deck:/usr/lib/steamos/gamescope-session:78–85` enables Steam volume-key handling and disables Steam's automatic device switching because WirePlumber handles it. | Separate volume changes from device/profile/route selection. Do not attribute every change to Steam itself. |
| Session/display | `Deck:/usr/bin/start-gamescope-session:9–22` sets application session type to X11, selects gamescope portal backends and clears inherited `DISPLAY`/`XAUTHORITY`. | Desktop and Game Mode are different display/portal contexts. Service-launched editors must receive the current DAW's context. |
| Graphics policy | `Deck:/usr/lib/steamos/gamescope-session:87–91,157–176,243–267` enables per-game Xwaylands, dynamic frame limiting and gamescope WSI; advertises Galileo HDR defaults; launches two Xwaylands at a 1280×800 application size. | Compare effective renderer, resolution, scaling, refresh, limiter and window association. Script defaults do not prove a particular application's effective settings. |

The user PipeWire service is attached to `default.target`; WirePlumber follows
PipeWire (`Deck:/usr/lib/systemd/user/pipewire.service:16–32` and
`wireplumber.service:1–20`). The inspected session scripts contain no explicit
Game Mode sample-rate/quantum override. This does not rule out dynamic changes
from Steam, another client, device selection or restored policy.

## Audio: four distinct settings layers

The idle Desktop readback has configured/default output **Speakers**, clock
settings **48 kHz / 512 frames**, minimum 256, maximum 8192, and forced rate/quantum
both zero. These match `Deck:/run/pipewire/pipewire.conf.d/context-properties.conf:1–6`.
The built-in audio nodes separately expose ALSA period size **1024** and headroom
**1024**. Those device properties are not Bitwig's callback size or a measured
round-trip latency. The installed hardware policy also disables the built-in
card's Pro Audio profile (`Deck:/run/wireplumber/wireplumber.conf.d/alsa-card1.conf:33–59`).
This is existing hardware policy, not a demonstrated mode-switch change.

1. **Device/profile/route:** WirePlumber selects and restores the card profile,
   speaker/headphone/HDMI route and defaults. Live `device.restore-profile`,
   `device.restore-routes` and `linking.follow-default-target` are all true.
   A stream following the default can move when that default changes. Compare
   configured default, effective default and Bitwig's actual linked sink; they
   are not interchangeable. [WirePlumber policy](https://pipewire.pages.freedesktop.org/wireplumber/daemon/configuration/settings.html)
2. **Graph timing:** configuration defaults, runtime metadata and active clients
   can influence rate/quantum. A latency request is advisory; forced node settings
   can override it. Read running driver/node timing alongside metadata rather
   than assuming that `512` in a file is effective.
   [PipeWire configuration](https://docs.pipewire.org/page_man_pipewire_conf_5.html),
   [node scheduling](https://docs.pipewire.org/page_man_pipewire-props_7.html)
3. **Hardware buffering:** ALSA period size, headroom, negotiated format and
   device latency are separate from graph quantum. A route/profile change can
   change this layer. [WirePlumber ALSA buffering](https://pipewire.pages.freedesktop.org/wireplumber/daemon/configuration/alsa.html)
4. **DAW/bridge configuration:** Bitwig chooses its backend and supplies the
   plug-in's sample rate, maximum block and actual blocks. Existing
   [FC-AUDIO-002 evidence](../FAILURE_CLASSES.md#L1804-L1827) shows
   `PIPEWIRE_QUANTUM=512/48000` did not make Bitwig's internal maximum 512 on an
   Ubuntu fixture. That demonstrates the distinction, not this Deck's cause.
   Current [readiness](../../bridge-manager/src/readiness.rs#L561-L569) reads
   clock metadata but leaves device rate and DAW maximum unknown; it cannot
   explain this transition by itself.

Access is another axis: Bitwig reaching PipeWire does not grant it ownership of
system defaults. A Flatpak's permissions remain relevant, but cannot establish
that its requested buffer or device route won.

## Graphics and scheduling can affect audio too

Gamescope controls presentation and supports frame limiting/scaling; its Vulkan
layer reads a limiter file. That makes frame policy a concrete variable for an
editor-related failure, not proof that a frame cap caused audio trouble.
[Gamescope](https://github.com/ValveSoftware/gamescope#readme),
[limiter implementation](https://github.com/ValveSoftware/gamescope/blob/2b5cf191d1047d6895516a834de8a72ac1c342f3/layer/VkLayer_FROG_gamescope_wsi.cpp#L465-L489)

The bridge's [graphical context](../../bridge-manager/src/transport_storage.rs#L29-L105)
passes selected display/bus fields, while its
[runner environment](../../bridge-manager/runtime/session.py#L234-L248) clears
Steam IDs and does not copy all gamescope variables. Consequently the native
DAW and service-launched Windows editor may receive different graphics policy.
Check the actual renderer and window/display association before changing this
boundary; blindly inheriting Steam's entire environment is not a fix.

Also compare Steam performance limits and effective CPU scheduling. The repository
already [identified Desktop foreground boosting](../AP17.md#L514) changing bridge
CPU weight when focus changed. That finding does not establish a Game Mode
failure, but prevents treating the two sessions as equivalent CPU conditions.

## First useful slice: repair one reproduced policy change

Use a disposable copy of one retained Bitwig project and the same plug-in,
runner, device and audio passage. Keep the delivered beta unchanged.

1. Capture Desktop, Game Mode and return-to-Desktop values for the four audio
   layers above. Record the observed symptom alongside whole-DAW callback timing
   and audible output. Use bounded, filtered `pw-dump`, `pw-metadata`, `pw-top`
   and native instance readback; exclude project paths and account data.
2. Capture current DAW/editor display, renderer, scale, refresh/frame limit and
   CPU/GPU limits. Exercise the same editor action. If audio policy is unchanged,
   investigate the graphics/scheduling delta rather than forcing a new buffer.
3. Change one implicated setting temporarily through its normal owner and repeat
   the failing passage. Restore it to confirm causality, then retain the smallest
   product fix or supported scoped setting. Never disable normal volume/device
   controls globally or remove speaker protection to make a test pass.
4. Show the repaired passage, editor action and saved-state reopen working across
   another mode round trip. Check ordinary output-device switching and suspend/
   resume for recovery without stuck notes, lost state or a stalled DAW.

The manager explains relevant configuration and scoped recovery. Bitwig owns its
backend/devices; PipeWire/WirePlumber own system audio; the proxy/host handles
legal format transitions and editor context. Keep those owners intact. Setup,
controllers and vendor browser flows become later work only when their specific
behavior is the demonstrated problem.

## Next-thread prompt

> Bitwig launch in Game Mode already works. Read AGENTS.md and this brief, then
> fix the operator's reported audio/video policy change affecting plug-ins.
> Compare the same retained project and plug-in in Desktop and Game Mode:
> device/profile/route, graph timing, hardware buffering, actual DAW blocks,
> renderer/display/frame policy and scheduling. Identify the changed setting and
> reproduce its musical or editor symptom. Change one implicated setting, compare
> and restore, then implement the smallest justified repair. Preserve normal
> system controls, projects, activation and the delivered beta. Follow AGENTS.md
> for routine experiments and permanent machine changes; land reviewed, green
> increments and demonstrate the repaired mode-switch journey.
