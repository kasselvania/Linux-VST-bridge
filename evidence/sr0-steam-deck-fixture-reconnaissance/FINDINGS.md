# SR0 findings

| Technical-lead question | Classification | Answer |
|---|---|---|
| Exact host hardware model | `observed` | Galileo |
| Exact host OS release | `observed` | SteamOS |
| Exact graphical session type | `observed` | wayland |
| Exact Bitwig Flatpak installed state | `observed` | true |
| Exact Bitwig version | `observed` | 6.0.11 |
| Exact Bitwig runtime | `observed` | org.freedesktop.Platform/x86_64/25.08 |
| Installed Linux Audio extension state | `not_found_in_bounded_locations` | no installed org.freedesktop.LinuxAudio runtime/extension on exact branch 25.08 |
| Flatpak runtime census completeness | `observed` | completed for exact branch 25.08 |
| Effective Bitwig VST3 path | `observed` | empty |
| Effective Bitwig CLAP path | `observed` | empty |
| Serum 2 installer in bounded locations | `not_found_in_bounded_locations` | no installer candidate in declared fixture-input or Downloads search |
| Serum 2 module in bounded locations | `observed` | 5 retained module candidate(s); search_complete=true |
| Other Serum/Xfer content in bounded locations | `not_found_in_bounded_locations` | no other Serum/Xfer named candidate in declared roots |
| Artifact-search completeness | `observed` | all contributing roots completed within declared bounds |
| Prefix-roster completeness | `observed` | all non-alias declared prefix roots completed within bounds; retained_prefixes=34 |
| License channel | `operator_input_required` | Xfer direct/owned, Splice paid-off lifetime/Xfer, active Splice Rent-to-Own, or another exact channel |
| Next GUI step | `gui_session_required` | Any later installer or authorization work requires an operator-controlled graphical session. |

## Observed

The retained packet records the host, session discovery result, local capacity, installed Flatpak metadata, audio command/session posture, tool versions, declared runner roots, immediate prefix inventory, bounded Serum/Xfer matches, and current exact-name process counts. Every completeness-sensitive census also retains its completion or incomplete state.

The host graphical session is `wayland`/`KDE`; Bitwig declares effective sockets `x11;pulseaudio;`. Bitwig launch and actual XWayland behavior remain unexercised.

Package metadata declares VST3 and CLAP extension paths, while the current read-only user override inspection reports effective VST3 and CLAP values as `empty` and `empty`. No override was changed.

A native yabridge-style Serum2 proxy bundle and corresponding Windows Serum2 module are observed by metadata/hash. The `yabridgectl` and `yabridge` commands are not installed in the SSH capture PATH; the existing artifacts are not an operability claim.

## Unknown or operator-controlled

- The exact lawful Serum 2 license channel remains `operator_input_required`.
- Any installation outside the declared bounded paths remains `unknown`.
- Credential entry and graphical installer/authorization behavior require an operator-controlled GUI in a later authorized slice.
- Prefix approximate sizes remain `unknown` because recursive traversal was deliberately not performed.

## Explicitly out of scope

No claim that Serum 2 installs, authorizes, scans, opens, processes audio, appears in Bitwig, or interoperates with Proton or a bridge.
