# SR0 toolchain and runtime inventory

## Development toolchain

| Tool | Classification | Version/result |
|---|---|---|
| `bash` | `observed` | GNU bash, version 5.3.3(1)-release (x86_64-pc-linux-gnu) |
| `git` | `observed` | git version 2.50.1 |
| `python3` | `observed` | Python 3.13.5 |
| `codex` | `not_installed` | — |
| `rustc` | `not_installed` | — |
| `cargo` | `not_installed` | — |
| `cmake` | `not_installed` | — |
| `ninja` | `not_installed` | — |
| `gcc` | `not_installed` | — |
| `g++` | `not_installed` | — |
| `clang` | `not_installed` | — |
| `clang++` | `not_installed` | — |
| `pkg-config` | `not_installed` | — |
| `flatpak` | `observed` | Flatpak 1.16.6 |

## Audio and compatibility commands

| Tool | Classification | Version/result |
|---|---|---|
| `pipewire` | `observed` | pipewire; Compiled with libpipewire 1.6.4; Linked with libpipewire 1.6.4 |
| `wireplumber` | `observed` | wireplumber; Compiled with libwireplumber 0.5.14; Linked with libwireplumber 0.5.14 |
| `pactl` | `observed` | pactl 17.0-43-g3e2bb |
| `aplay` | `observed` | aplay: version 1.2.14 by Jaroslav Kysela <EMAIL> |
| `wpctl` | `observed` | installed; this build exposes no read-only version option |
| `pw-cli` | `observed` | pw-cli; Compiled with libpipewire 1.6.4; Linked with libpipewire 1.6.4 |
| `wine` | `not_installed` | — |
| `wineserver` | `not_installed` | — |
| `winetricks` | `not_installed` | — |
| `umu-run` | `not_installed` | — |
| `protontricks` | `not_installed` | — |
| `yabridgectl` | `not_installed` | — |
| `yabridge` | `not_installed` | — |

## Audio/session facts

| Fact | Classification | Value |
|---|---|---|
| PipeWire session available | `observed` | true |
| PipeWire Audio/Sink count | `observed` | 2 |
| PipeWire Audio/Source count | `observed` | 1 |
| PipeWire Audio/Device count | `observed` | 2 |
| PipeWire output-stream count | `observed` | 0 |
| PipeWire input-stream count | `observed` | 0 |
| PulseAudio compatibility available | `observed` | true |
| ALSA available | `observed` | true |
| ALSA playback-card count | `observed` | 7 |
| Default route class | `unknown` | not exposed without retaining device names |
| Default route API | `observed` | alsa |
| Default route device name | `explicitly_out_of_scope` | private device names are not retained |
