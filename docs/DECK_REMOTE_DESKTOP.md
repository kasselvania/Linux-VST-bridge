# Deck desktop access from the development Mac

**Status: actual Codex-through-Moonlight control verified on 2026-09-05.** Issue #55. Supporting work for AP3 #54; not a VST runtime dependency.

## Observed setup and control

Video observed: **true**. Keyboard/mouse verified: **true**. Codex-through-Moonlight verified: **true**. The actual Mac Computer Use session clicked KWrite's New File button, typed a unique two-line AP3 marker, disconnected with Ctrl+Alt+Shift+Q, reconnected to the same document, and typed a third reconnect marker. Fresh stream images showed each result. SSH and Sunshine remained available. The agent closed KWrite and discarded only this unsaved scratch document, then disconnected streaming. No streamed terminal was operated; the editor was started through SSH as a temporary user service.

The Deck uses Wayland, a 1280×800 logical desktop, portal capture and AMD VAAPI H.264 encoding. The operator approved portal capture, the local certificate/credential flow and pairing normally. Sunshine is the official user Flatpak `dev.lizardbyte.app.Sunshine`, version `2026.516.143833`, commit `62e90ef28d2c760d77e85233e1c5754c29f56e430b726947cdd79867915901e2`. The existing Mac Moonlight 6.1.0 was reused. An initial black stream resolved after capture negotiated; the observed control test used the live desktop.

Specific setup changes: installed that user Flatpak; copied its packaged `app-dev.lizardbyte.app.Sunshine.service` into the user's systemd directory and started it without enabling autostart; imported the existing graphical session variables into the user service manager; created Sunshine configuration under its Flatpak config directory. Existing uinput access worked, so no root, udev, firewall, immutable-base or input-permission changes were made. Pairing and portal material remain private. Configuration selects portal/VAAPI, IPv4, PC-only administration, mandatory LAN/WAN encryption, disabled UPnP, stream audio and gamepad, and H.264 only. Administration uses a loopback-only SSH forward. Existing pairings were preserved.

Moonlight now uses windowed 1280×800, 30 fps, 5 Mbps and desktop mouse mode. Host-speaker muting is disabled. Prior settings were 720p/60 fps/10 Mbps, borderless windowed, desktop mouse mode off and host-speaker muting on; VSync remains on and frame pacing off. No audio sink, microphone or Bitwig routing was changed. Sunshine is a separate development service and must not be counted as a leaked product process.

Start/stop on the Deck with `systemctl --user start app-dev.lizardbyte.app.Sunshine.service` / `systemctl --user stop app-dev.lizardbyte.app.Sunshine.service`. Connect the paired Deck entry in Moonlight and choose Desktop; Ctrl+Alt+Shift+Q disconnects while preserving Deck applications. SSH remains independent. Rollback: stop the service, remove only the copied user service and run `systemctl --user daemon-reload`, then uninstall the added user Flatpak if desired; retain its private configuration unless the operator explicitly chooses to remove pairing data. Restore the listed Moonlight settings if desired. This requires a reachable, logged-in, awake Desktop Mode session; cold boot, login screens and Gaming Mode were not tested.

## AP4 recovery readback

The operator explicitly authorized a persistent Sunshine-only KDE capture grant while away from the Deck. Applied the documented `flatpak permission-set kde-authorized remote-desktop dev.lizardbyte.app.Sunshine yes`; the prior entry was absent. A private before/after note retains rollback with `flatpak permission-remove kde-authorized remote-desktop dev.lizardbyte.app.Sunshine`. Pairing, encryption and SSH were preserved. `system_tray = disabled` is a separate optional workaround with the original config retained privately.

Live desktop video, agent clicks and one disconnect/reconnect succeeded after the grant. A later reconnect still produced no video and Sunshine aborted; restarting only its user service restored the stream without a physical Share prompt. The tray workaround is therefore not a proven cure. For the remaining test, stop the service after disconnecting the primary measurement stream and start it fresh before GUI work; keep that stream connected through all three GUI stages. This changes the supporting service lifecycle, not plug-in processing or measurement criteria.

## Topology and purpose

Mac-side permitted Codex Computer Use -> Moonlight window -> Sunshine -> the existing Steam Deck Desktop Mode session. The existing authenticated SSH channel remains independent for commands, builds, logs, files and recovery. Do not install Sunshine on the Mac or confuse the Deck's usual Moonlight client role with this reverse direction.

This should let the engineer operate ordinary Deck GUI flows without requiring the operator beside it. It does not promise bootloader, login-screen, suspended/offline-host or unattended credential access. Both machines need a reachable usable session. No change to authentication, screen locking or cold-boot policy is included. A scoped awake inhibitor while the operator's test session is running is permitted; remove it on exit rather than globally disabling sleep.

## Install the ordinary tools

Use current official prebuilt Sunshine and Moonlight packages. Record actual versions and installation method; do not build either application or start a distro/graphics project. Inspect the actual Deck session (X11 versus Wayland), resolution and input-device permissions before choosing its capture backend.

On SteamOS, prefer a user-installed official Flatpak where its capture backend fits; an official user-local AppImage is an allowed alternative for existing X11. Neither format supplies KMS capture in current upstream documentation. For AMD/X11 use an available X11 capture backend rather than NVIDIA-only NvFBC; for Wayland use the available supported portal/KWin route. Verify support in the installed release rather than copying options from newer development docs. Prefer working AMD hardware encoding, but a modest software-encoded stream is enough to prove GUI control if it does not interfere with testing. Do not replace Mesa/KWin/kernel or disable SteamOS read-only protection to make streaming work.

Input access matters as much as video. Inspect the package's documented additional-install/input setup before applying it. Use the supported narrow uinput permission/udev path where needed, not world-writable input devices, root Sunshine or a broad input/security workaround. A necessary administrator/password or portal approval is an operator action through the normal local prompt. Record how to undo the exact service/rule/config changes. Do not erase an existing Sunshine/Moonlight installation or pairing.

Start Sunshine in the actual logged-in desktop user's graphical environment, not an unrelated root/SSH display. Initially use its built-in Desktop entry without application-launch/kill commands. A user-session start/stop service is sufficient; system boot before login, Gaming Mode and virtual monitors are outside this task.

## Connect privately

Pair only the operator's Mac to the Deck. Keep credentials, TLS/private keys, pairing material and host addresses in private local configuration, never Git or reports. Restrict access to the trusted LAN/client (or an already configured private Tailscale route). Do not enable UPnP, router forwarding, public admin access or broad firewall exceptions. Prefer mandatory stream encryption when supported. Keep the administration UI local to the Deck and reach it through authenticated SSH port forwarding when practical; app binding/firewall choices must still allow that local route.

On the Mac, use Moonlight windowed at the Deck's native resolution (normally 1280x800; confirm), SDR/H.264, 30 fps and a modest bitrate initially. Enable its desktop/direct mouse mode rather than game-style relative capture. Keep the video area visible and stable in size. The documented PC-client mouse-mode toggle is Ctrl+Alt+Shift+M; stream disconnect is Ctrl+Alt+Shift+Q. Verify the actual Mac bindings and a clean escape rather than trapping the operator's input. Disable stream audio initially (`stream_audio = disabled` where supported); do not replace audio sinks, microphones, Bitwig devices or PipeWire routing. Audio may later be enabled deliberately for listening, not measurement.

## Prove the whole agent-control chain

First show that Moonlight receives the correct live Deck desktop and actual mouse/keyboard input works. Then use the **actual Mac-side Computer Use tool**, with its normal app approval, Screen Recording and Accessibility permissions, to select a harmless Deck GUI window and type a unique marker into a new scratch text-editor document. Verify the resulting text and a click/drag/scroll through fresh images; screenshots alone or a human clicking do not prove the agent can control it. A local coordinate click may need Moonlight's direct mouse mode; do not assume streamed pixels have a normal accessibility tree.

Use this for Bitwig and ordinary application UI. Do not drive a streamed terminal, scripts pasted into GUI launchers, the agent itself, permission dialogs or administrator authentication to bypass a tool's excluded apps or sandbox. Shell work stays on the approved shell/SSH path. If Computer Use refuses the remote client, lacks permissions or cannot inject usable input, report that particular boundary; do not spoof the app or substitute a prohibited automation route.

Disconnect/reconnect Moonlight and show the same Deck applications remain running, no stuck input remains, and SSH still works. Document the actual start, stop, reconnect and rollback commands and the normal-session limitations. Temporary passwords or one-time approvals need the operator only at that step, not a new design review. While waiting for permission, continue the independent VST work.

## Separate measurements and operational results

Record three booleans separately: video observed, keyboard/mouse verified, Codex-through-Moonlight verified. Record package/capture/encoder/session details and the specific persistent setup changes, not raw system dumps. Add the installed service commands and a small idempotent helper only where it materially reduces repeat work.

The remote service is not part of disposable Windows process cleanup. Its installation happens before a new declared product-test baseline; never hide an unexpected before/after change by taking another baseline. Stop active streaming for the primary audio-timing measurement, then label a short stream-active check separately. Stream playback delay/quality is not VST processing latency or sample accuracy.

## Upstream references checked during preparation

- [Sunshine setup](https://docs.lizardbyte.dev/projects/sunshine/latest/md_docs_2getting__started.html): prebuilt/user-level installation, package limits, user service, pairing and Desktop entry.
- [Sunshine configuration](https://docs.lizardbyte.dev/projects/sunshine/latest/md_docs_2configuration.html): capture/encoder selection, input, network limits and optional audio.
- [Moonlight PC setup](https://github.com/moonlight-stream/moonlight-docs/wiki/Setup-Guide): direct pointer mode, windowed streaming, disconnect and private-network use.
- [OpenAI Computer Use](https://developers.openai.com/codex/app/computer-use/): local app control and separate system/app permissions. The combined Moonlight path still needs its own practical test; the documentation does not certify this exact composition.
