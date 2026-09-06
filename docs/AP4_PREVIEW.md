# AP4 private desktop preview

The native AGain preview can connect from an ordinary **Bitwig Studio Applications launch**. A private session owner launches the existing Windows host on demand and supervises only its own Windows process group. It never launches or kills the DAW. This is development tooling for the retained reference fixture, not a general plug-in broker or installer.

## Connection and lifetime

When no explicit test binding is present, the AP4 proxy connects to `~/AP4-State-Test/preview/owner.sock`. The directory and socket must be private and owned by the current user. The owner gives one instance a fresh private transport directory and session identifier. The existing authenticated loopback handshake and shared audio mapping then connect the actual Windows component. No saved gain or plug-in state travels through discovery.

The local connection stays open for the instance's lifetime. A healthy instance has no test-duration expiry. Ordinary waiting, stopped transport and Moonlight disconnection do not revoke it. Normal unload closes the Windows component through the existing transport. Native process loss or an explicit owner stop invokes the existing bounded process cleanup. The owner keeps a short two-second grace for normal Windows teardown after native disconnect. Each subsequent instance gets a fresh stage, mapping and Windows process.

Windows distinguishes waiting for the first byte of a **new command** from receiving a message or waiting for a reply. An empty connection is polled without expiring the instance; EOF still fails. Once a frame begins, its header and payload share one five-second deadline. Hello, response, component-state, readiness and cleanup timeouts remain bounded. Owner loss also cancels native transport startup and fails an idle worker. No retry or cached default conceals a failed restore.

The existing Windows `IComponent::getState` / `setState`, complete 12-byte AGain payload, native state envelope, save barrier and controller `setComponentState` synchronization remain the persistence path. The DAW owns the saved project. There is no state sidecar used for restoration.

## Using the development preview

Use the existing retained Windows/native artifacts and pinned Proton 11.0-2c / Steam Linux Runtime 4 fixture on the Deck. Builds and transfers use the existing helpers. The owner runs from a checkout with repository history because it reuses the same pinned launcher dependencies as the previous AP4 worker.

1. Publish the built `AGainQueuedBridge.vst3` bundle in the existing private test plug-in directory, `~/AP4-State-Test/plugins`, selected in Bitwig's normal VST3 locations. Preserve any pre-existing publication and preferences. The focused check reused the previously prepared AP4 preferences.
2. Start the owner through the existing SSH command path, with the exact retained host root and Windows build-input digest:

   ```sh
   systemd-run --user --unit=lvb-ap4-preview \
     --property=KillMode=mixed --property=TimeoutStopSec=30 --property=UMask=0077 \
     /usr/bin/python3 /path/to/checkout/tools/ap4_preview.py \
     --host-root /path/to/retained/host-manifest \
     --build-input <windows-build-input-sha256>
   ```

3. Open **Bitwig Studio from Applications**, then open the disposable project. No DAW command wrapper, environment injection, desktop-entry edit or extra Flatpak permission is required on the tested Bitwig 6.1 installation. Its existing `filesystems=host` permission supplies visibility; this is a version-specific observation.
4. Close Bitwig normally when finished. Stop the owner with `systemctl --user stop lvb-ap4-preview`. Restore the preferences/publication changed for the test. Keep the saved test project and useful private results.

The owner refuses an existing socket, including a stale socket, rather than stealing another session. After an abnormal owner death, inspect the owned processes and retained stage before removing that exact stale endpoint. An active second instance is refused promptly; there is no queued restore that might later load the wrong session.

## Diagnostics and limits

Private results are retained under `~/AP4-State-Test/preview/results`. They include the native state hashes, Windows lifecycle/call results, independent returned-sample comparison, original failures and cleanup outcome. Report writing occurs outside the audio callback. A containment failure retains the stage and stops the owner. These local files can contain private paths and process coordinates; they are not automatically published.

This preview remains one reference effect at a time: float32 stereo, 48 kHz, 1–256-frame callbacks, and 1024 samples of added latency. It requires the prepared runtime and artifact stores plus a running user-session owner. It does not establish commercial plug-in support, instruments, vendor editors, multi-instance operation, low-latency suitability, restart after reboot, automatic installation or general Flatpak packaging. Remote video still depends on the separate Sunshine/Moonlight setup.

See [the focused normal-desktop result](AP4_RESULT.md) and [the preserved D9/A2 record](AP4_ATTEMPT_STATUS.md). The historical receipts and their labels remain unchanged. The test restored its temporary publication/preferences and stopped the owner; preview setup is required before another manual use.
