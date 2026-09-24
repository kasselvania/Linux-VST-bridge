# Digitalis JACK-period comparison, 2026-09-24

Fixture: Pi 5/ShieldXL, Digitalis v1.1 Windows x64 VST3, the saved owner-approved sound slot from PR #159, 48 kHz, vendor quantum 256 frames, bridge reserve 2,048 frames, editor closed during matched captures. The native bridge, Windows host, runner and plug-in were identical across the two captures. Audio was routed from ShieldXL stereo inputs through the plug-in to the JACK capture helper. Private source audio, output audio, state and full logs remain off Git.

| Observation | JACK 512 | JACK 256 |
| --- | ---: | ---: |
| Fresh process, same state | yes | yes |
| Source-owned physical stereo capture | completed, five seconds | completed, five seconds |
| Captured audio | finite, nonzero stereo | finite, nonzero stereo |
| Capture xruns / bad blocks | 0 / 0 | 0 / 0 |
| Reported missing frames / gaps | 0 / 0 | 0 / 0 |
| Reported process / callback failures | 0 / 0 | 0 / 0 |
| Reported vendor / bridge frames | 4,096 / 2,048 | 4,096 / 2,048 |
| Session result | exit 0, clean shutdown | exit 0, clean shutdown |

The operator initially said 256 felt the same as 512, then corrected the listening impression: 256 felt snappier. This is a subjective observation. After the session, the retained synchronized JACK-port input/output files were compared offline using float32 stereo samples and FFT cross-correlation. For the left channel in four consecutive one-second windows, the strongest lag was 6,150–6,151 frames at JACK 512 and 6,147–6,152 frames at JACK 256. Correlation magnitudes were 0.75–0.91 within those windows; the full five-second files gave peaks of 6,151 and 6,148 frames with correlation magnitudes 0.95 and 0.96. The right channels agreed. This measured JACK-port-relative lag is 128.06–128.17 ms and agrees with the reported 4,096 vendor plus 2,048 bridge frames to within eight frames. The raw audio stays private. No analog input-to-output loopback, independent direct-monitor route, or 128-frame reserve was measured. The shorter JACK period is not claimed to make Digitalis nearly imperceptible.

After 256 passed, a fresh 128-frame process reached ready and exited cleanly. Before any 128-frame capture or listening pass, the operator asked to stop that comparison and leave Digitalis open at 256 frames. The 128 case therefore has no delivery or feel result.

A separate 256-frame live session was started with the editor open and physical input/output ports connected. Its private systemd owner stopped on the operator's request, with child exit 0, clean routing retirement, and JACK restored to 512 frames. The operator then asked for a 128-frame interactive session. That session reached editor readiness but was stopped when the operator clarified that the Digitalis processing quantum and bridge setting should also descend; the brief session still used the old 256-frame quantum and 2,048-frame reserve, so it is not evidence for a matched 128 configuration. Its child exited 0, the ports disconnected, and JACK again returned to 512 frames. Neither session produced an analog loopback measurement.

The period change is runtime-only through `jack_bufsize`; the JACK service configuration remains 512 frames. No runner or vendor installation changed. The working packaged artifacts and canceled multicore/UI checkout were left intact.
