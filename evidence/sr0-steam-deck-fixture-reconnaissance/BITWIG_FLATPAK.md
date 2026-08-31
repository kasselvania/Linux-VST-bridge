# SR0 Bitwig Flatpak

Bitwig was not launched. Values are read from installed Flatpak metadata only.

| Fact | Classification | Value |
|---|---|---|
| Installed | `observed` | true |
| Version | `observed` | 6.0.11 |
| Branch | `observed` | stable |
| Origin | `observed` | flathub |
| Architecture | `observed` | x86_64 |
| Runtime | `observed` | org.freedesktop.Platform/x86_64/25.08 |
| Installation scope | `observed` | system |
| Plug-in sandbox mode | `unknown` | not available from Flatpak metadata; Bitwig settings not inspected |
| Declared VST path | `observed` | /app/extensions/Plugins/vst |
| Declared VST3 path | `observed` | /app/extensions/Plugins/vst3 |
| Declared CLAP path | `observed` | /app/extensions/Plugins/clap |
| Effective VST path | `observed` | /app/extensions/Plugins/vst;=/usr/lib/ |
| Effective VST3 path | `observed` | empty |
| Effective CLAP path | `observed` | empty |
| Graphics sockets | `observed` | x11;pulseaudio; |
| Installed Linux Audio extensions | `not_found_in_bounded_locations` | no installed org.freedesktop.LinuxAudio runtime/extension on exact branch 25.08 |
| Runtime/extension census completeness | `observed` | completed for exact branch 25.08 |
| Override inspection completeness | `observed` | user and system override commands completed |

## Effective installed permissions

From `flatpak info --show-permissions`:

```ini
[Context]
shared=network;ipc;
sockets=x11;pulseaudio;
devices=all;
features=multiarch;
filesystems=xdg-run/pipewire-0;/usr/lib;host;
persistent=.BitwigStudio;Bitwig Studio;.java;

[Session Bus Policy]
org.freedesktop.Notifications=talk

[Environment]
VST_PATH=/app/extensions/Plugins/vst;=/usr/lib/
ALSA_CONFIG_PATH=
CLAP_PATH=
VST3_PATH=
```

## Package metadata declarations

```ini
[Context]
shared=ipc;network;
sockets=pulseaudio;x11;
devices=all;
features=multiarch;
filesystems=xdg-run/pipewire-0;host;
persistent=.BitwigStudio;Bitwig Studio;.java;

[Session Bus Policy]
org.freedesktop.Notifications=talk

[Environment]
VST_PATH=/app/extensions/Plugins/vst
ALSA_CONFIG_PATH=
CLAP_PATH=/app/extensions/Plugins/clap
VST3_PATH=/app/extensions/Plugins/vst3

[Extension org.freedesktop.LinuxAudio.Plugins]
directory=extensions/Plugins
no-autodownload=true
subdirectories=true
add-ld-path=lib
merge-dirs=vst;vst3;clap
version=25.08

```

## Explicit Flatpak overrides

Read-only user and system override inspection:

```ini
[user overrides]
[Context]
filesystems=/usr/lib/

[Environment]
VST_PATH=/app/extensions/Plugins/vst;=/usr/lib/
CLAP_PATH=
VST3_PATH=

[system overrides]
```

## Matching Freedesktop runtime and Linux Audio extensions

```text
org.freedesktop.Platform	x86_64	25.08	freedesktop-sdk-25.08.16	system	flathub
org.freedesktop.Platform	x86_64	25.08	freedesktop-sdk-25.08.16	user	flathub
```

## Flatpak collection completion

| Role | Application | Classification | Status | Detail |
|---|---|---|---|---|
| `application_census` | `com.bitwig.BitwigStudio` | `observed` | `completed` | application_present |
| `app_info` | `com.bitwig.BitwigStudio` | `observed` | `completed` | status=completed;command_exit=0;filter_exit=0;bytes=693;rows_seen=0;rows_retained=0 |
| `permissions` | `com.bitwig.BitwigStudio` | `observed` | `completed` | status=completed;command_exit=0;filter_exit=0;bytes=339;rows_seen=0;rows_retained=0 |
| `metadata` | `com.bitwig.BitwigStudio` | `observed` | `completed` | status=completed;command_exit=0;filter_exit=0;bytes=1213;rows_seen=0;rows_retained=0 |
| `user_overrides` | `com.bitwig.BitwigStudio` | `observed` | `completed` | status=completed;command_exit=0;filter_exit=0;bytes=117;rows_seen=0;rows_retained=0 |
| `system_overrides` | `com.bitwig.BitwigStudio` | `observed` | `completed` | status=completed;command_exit=0;filter_exit=0;bytes=0;rows_seen=0;rows_retained=0 |
| `metadata_excerpt` | `com.bitwig.BitwigStudio` | `observed` | `completed` | relevant_rows=25 |
| `runtime_census` | `com.bitwig.BitwigStudio` | `observed` | `completed` | status=completed;command_exit=0;filter_exit=0;bytes=1691;rows_seen=0;rows_retained=0; expected_branch=25.08; matching_rows=2 |
