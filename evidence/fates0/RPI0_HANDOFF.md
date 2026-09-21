# FATES0 to RPI0 handoff

This handoff is not active until `hardware-contract.json` has
`acceptance_status: accepted`.

The intended interface is:

- JACK2 stereo output: accepted physical `system:playback_1/2` names retained in the
  final contract;
- JACK2 stereo input: accepted physical `system:capture_1/2` names retained in the
  final contract;
- JACK MIDI: exact admitted USB controller port retained in the final contract;
- controls: newline-delimited typed events from
  `/usr/local/libexec/fates0/controls.py --format json` outside the audio callback;
- OLED: bounded JSON requests over `/run/fates0/oled.sock`;
- no root requirement for the future host;
- Fates headphone output is diagnostic/user monitoring and is not a substitute for
  the accepted stereo line-output contract.

Do not infer device identities from these expected names. Consume the exact observed
values only after the contract is physically completed.
