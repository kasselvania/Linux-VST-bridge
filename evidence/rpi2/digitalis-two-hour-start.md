# Digitalis two-hour run: admission only

The operator requested an informal two-hour session with the editor open,
48 kHz and JACK / processing quantum / bridge reserve at 128 / 128 / 128.
It began 2026-09-24 at 19:42:52 UTC (12:42:52 Pacific) and its existing outer
guard is scheduled to stop it at approximately 21:42:52 UTC (14:42:52 Pacific).
This record establishes startup, not a completed two-hour result.

The original stale session was retired through its owner, restoring JACK 512.
An initial fresh launch reopened correctly, but attempting to alter the
running Windows unit returned `Cannot set property RuntimeMaxUSec, or unknown
property.` Its five-minute limit remained unchanged. That session was stopped
through the owner before the startup fix was installed.

`rpi1/standalone/src/supervisor.rs` now accepts the explicit native-owner
environment setting `LVB_RPI1_RUNTIME_SECONDS` in 30–7200 seconds, retaining
the 300-second default. The private session wrapper passes its selected
session duration into the native process. The Windows host receives its
limit when its exact transient unit is created. No system-wide unit or
runtime configuration was changed.

The existing timing-selector source bundle was reused; only the supervisor
file changed in the native build inputs. Updated bundle SHA-256:
`31c3289297f5e5b4775c58fa63dd61d8f90567dae3962c5764e2d2029f5d2f58`.
Native executable SHA-256:
`5e2aee82d0ea8d59c466e4d600ef6f5b783beec27d5b67bfbc24bae3ddb6e925`.
The Pi offline locked release test for runtime selection passed (1 test),
and the `jack-runtime,rpi2-quantum` release binary built. The pre-existing
unused `wire` function warning remains; no warning cleanup was attempted.

The new native file is private and separate from the original timing
candidate. Both private helper files were backed up as `.before-two-hour`.
The selector now points to the new digest-bound executable. It retains the
saved sound, Windows host, Digitalis module, runtime, memory/task bounds,
75°C/current-power-warning stop and owner-controlled route cleanup. No
preset, trim, dry/wet parameter or bypass change was made.

The [starting receipt](digitalis-two-hour-start.json) verifies:

- Windows host active with `RuntimeMaxUSec=2h`; native wrapper 2h40s and outer
  owner 2h1m40s leave time for ordered shutdown.
- Digitalis's X11 editor window is viewable; 74,491-byte saved slot restored.
- Stereo capture/playback connected; 128 / 128 / 128 at 48 kHz, map-v2 with
  512-frame capacity, vendor latency 4,096 frames.
- Bypass false, output trim 1; no terminal failure, zero JACK xruns and zero
  process/callback failures at the starting snapshot.
- The starting snapshot already has 1,408 missing frames and five gaps,
  approximately 58 seconds after launch. These counts remain visible and
  must not be erased or presented as a clean-audio pass.
- 50.15°C, `throttled=0x0` at that snapshot.

The service was left running for the returning operator. No physical latency
measurement or CPU profile was performed. The uncommitted earlier
128-frame implementation remains in its original checkout; this commit
contains the runtime selector, scope and new session evidence only.
