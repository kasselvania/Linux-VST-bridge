# AP4 — normal desktop save and reopen

**Working result, pending review in PR #59.** On the retained Steam Deck/Bitwig fixture, an ordinary Applications launch saved a real Windows AGain setting of **0.1650**, closed normally, and reopened it with fresh native and Windows processes. Both displayed gain controls restored to 0.1650. Playback before any control edit returned **5,250,048 independently compared samples with maximum error 0**, including 1,718,784 nonzero samples. The restored state came from the project through Windows `IComponent::setState`, not a driver value or cached default.

The same save session survived stopped transport and more than three minutes disconnected from Moonlight, then reconnected and saved successfully. No Sunshine restart was needed. Both Bitwig and its native plug-in host had **no `LVB_*` environment variables** on both Applications launches. The private owner supplied each fresh connection through the existing transport and launcher.

## Changes addressing review 5123893540

- The existing supervisor accepts an explicitly owned interactive lifetime without the automated 180-second stage expiry. Automated runs retain their existing duration limits. Native unload, disconnect or explicit owner stop ends the interactive session and uses existing owned cleanup.
- Windows waits for a new command without treating five seconds of inactivity as failure. EOF still fails. Once the first byte arrives, the complete header and payload share the existing five-second deadline; issued replies and plug-in calls stay bounded.
- The AP4 proxy discovers a private, current-user Unix socket when explicit test bindings are absent. A small single-instance preview owner starts the retained Windows host and keeps a lifetime connection. It never starts the DAW and supplies no restoration state. This is not a general broker.

Actual Windows component get/set state, the complete AGain payload, state identity checks, coherent save barriers and native controller `setComponentState` synchronization are preserved. The audio callback continues to use the existing preallocated queues; discovery, process ownership and diagnostics stay outside it. See [preview use and lifetime](AP4_PREVIEW.md).

## Focused development check

Observed September 5 Pacific / September 6 UTC, 2026, in the last approved development batch. The agent used Mac Computer Use through the established Moonlight connection, with independent SSH readbacks. This is a successful development observation submitted for review, not a replay under the retired acceptance protocol or a physical operator acceptance claim.

The test copied the preserved D9 mute project into a new disposable `AP4-Normal-Desktop` project. Initial mute recall and playback were observed before changing gain to 0.1650. After the stopped pause and Moonlight reconnect, Ctrl+S captured the real Windows state. File → Quit closed Bitwig and the owned Windows component normally. A fresh Applications launch reopened the saved copy; controls and playback were checked without changing the value, then File → Quit closed it again.

| Session | Returned samples compared | Before first edit | Nonzero samples | Maximum error |
| --- | ---: | ---: | ---: | ---: |
| Save after pause/reconnect | 36,856,320 | 6,012,928 at initial saved mute | 5,104,640 | 0 |
| Fresh project recall | 5,250,048 | 5,250,048 at saved 0.1650; zero edits | 1,718,784 | 0 |

Sample counts include silent processing while transport was stopped. Both sessions reported fault 0, zero callback rejections, clean native termination, Windows exit 0, empty owned process groups and retired stages. The inherited supervisor label is `scanner_completed`; the state and audio records establish the additional behavior, not that label alone.

Windows returned all **12 state bytes**. The saved snapshot and fresh restored readback have SHA-256 `1d423f2175a374737b5a457be154da200af88032a868edb7dc1c222f2cda81a2`, gain `0.165000007` in float32. The new saved project remains unchanged after reopening, SHA-256 `d8acb2fbb487cfa0039a43ef92619f5e06a2fbd63e1b0be988fe721615541a05`. D9's original mute project remains unchanged at `9ec2a12acb631e201335408ef975dd54b53b36c65859dc70ca426e7a7dafd7c5`.

## Fixture and builds

Steam Deck OLED/Galileo, x86_64, SteamOS **3.8.16** build `20260716.1`, kernel `6.16.12-valve24.5-1-neptune-616-gb2f7cfe85e45`, Plasma Wayland. Bitwig Flatpak **6.1**, commit `8a048e733e74dda8b897339436153a2d5df952f29d362dfcaca9d8e0d6f6c231`, Freedesktop runtime **25.08**. Existing permissions and overrides were preserved. The tested configuration is one retained WA0 Windows AGain effect, float32 stereo, 48 kHz, maximum 256 frames, **1024 samples / 21.33 ms added latency**.

Proton **11.0-2c**, build `25118279`, Steam Linux Runtime 4 **4.0.20260805.254769**, pressure-vessel `0.20260805.0`; launch-critical identity `20064247dc297d0228bbf63d5a49050238c518d14fbb77b61a8c73351db5ebf7`. The retained Windows AGain module SHA-256 is `60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f`.

Both binaries were built at `da56fb80cb4d689c9ba24d1fdb3803de912ed725`. Later changes add a loaded-proxy regression, the missing clean-CI dependency fetch and the owner's repository lookup; binary sources are unchanged. The live owner ran at `cd3d4670f30debbefff8f437611d17f2e0d6af69`.

- Windows producer [34010088604](https://github.com/kasselvania/Linux-VST-bridge/actions/runs/34010088604), artifact `9982219826`; manifest `f00325df1cefe08c4496d3007211d7ecebdd02a49d4dc42a6a2e0a94881ec615`.
- Windows build inputs: `df6480f32eb04197f59001928d79beaf28b61f91af29fc4808b82b0998c4e99a`.
- Native manifest: `fd7a24c07f63779270597ecb20f2a1249e04d6bbcbcacc25ad8beed2826c7df4`; native binary SHA-256 `0fcff99f28b573f1c40036d291d482c0296a8cfda8a71ed7a086a0ec3d4c1b84`.

The [sanitized result](../evidence/ap4-normal-desktop/result.json) retains exact fixture/build coordinates, original private-result hashes, state hashes, audio counters, GUI observations and cleanup. Raw records remain private on the Deck under `~/AP4-State-Test/preview/results/` and on the Mac under `~/Library/Application Support/Linux VST Bridge/development/ap4-normal-desktop-2026-09-06/`. Projects and binaries are not published.

## Focused regressions

- **38** AP4/AP3 execution and diagnostic-runtime Python tests passed.
- **15** Rust backend tests passed, including private discovery, owner-loss startup cancellation and unsafe/stale endpoint refusal.
- **5** owner tests passed: no Windows launch for malformed/expired startup, disconnect and explicit stop, retained failed containment, and the actual supervisor remaining alive beyond 180 seconds before owned cleanup.
- **5** socket-body cases passed locally and under actual Windows Winsock in the producer: a command after 6.1 seconds idle, EOF, partial header, partial payload and absent issued reply. Each active-message/reply timeout remained five seconds.
- **2** loaded native-proxy cases passed on Linux with a substituted Windows peer: discovery without test environment variables and lost restore reply. The latter failed explicitly with **zero processing calls** and no default fallback or retry. These are local regressions, not real-Windows evidence.

PR CI passed on the tested implementation. Its first clean run exposed an uncached locked `sha2` dependency; adding a locked dependency fetch fixed CI without changing a dependency or rebuilding the live artifacts. No historical workload sequence was replayed.

## Preserved history, cleanup and remaining limits

[D9 and failed A2](AP4_ATTEMPT_STATUS.md) retain their original provenance and labels. D9 still reports **20,284,928 samples, maximum error 0** at `cdadb96fda11f7fa8eb4862ad381f8487074b2a3`, classified `DIAGNOSTIC_NON_AUTHORITATIVE`, `acceptance_eligible: false`; its classified-result digest remains `73edc653be9a4559ffd584476933d7aac076441bb7b5c7c4ddd24d8b9e5b47aa`. Its actual full-state/reduction/bypass and gain/mute recall observations remain useful. A2 remains a failed `stage_timeout`, Windows exit -15. Neither candidate was replayed or relabeled.

Cumulative allowance: **10/10 development batches**, **2/2 legacy acceptance candidates**, **3/6 Windows producer attempts**. The new focused batch used the one remaining development allowance. Legacy ledgers and artifact bindings were not reset or rewritten. No further live workload is scheduled.

Final readback: no Bitwig or owned Windows hosts, no disposable stages, owner stopped, private socket removed, temporary plug-in publication removed and original preferences restored. The saved copy and original D9 project remain private and unchanged. Sunshine remains the separate supporting development service.

The preview requires prepared artifacts/runtime, the private plug-in location and a running preview owner. Only one instance is supported; a concurrent second instance is refused. No automatic installation, general broker, vendor editor, instrument/MIDI support, commercial compatibility, lower-latency suitability, reboot persistence or general Flatpak packaging is established. The preview is not left installed or running after the check. AP3 remains the accepted baseline until PR #59 is reviewed; **leave the PR unmerged**.
