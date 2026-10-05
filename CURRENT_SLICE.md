# Current task: stop the audio dropouts

## Goal

Pure LoFi plays in Bitwig on the Deck at 512 frames / 48 kHz, SameCallback,
editor open, for ten minutes without a dropped cycle; then thirty minutes with
normal interaction. Preserve projects, licensed state and the machine budget.

## Works now

Every native call can be paired with its Windows/vendor SDK call elapsed time.
The three-minute baseline retained all 16,875 pairs: typical whole call 3.18 ms,
Windows/vendor call 2.55 ms, remainder 0.64 ms. Vendor elapsed includes waits.

## Still broken

Earlier matched open and closed sessions each had three graph errors and three
silent 512-frame spans. The new baseline had one of each. Temporary Wine-server
and actual-caller realtime priority each left two of each; both were restored.
The earlier truncated recording with recorder errors was not a clean pass.

## Best explanation (about 70%)

Late host-to-plug-in admission or process handoff, rather than thread priority
alone. The caller-priority trial still had an 18.74 ms arrival interval, including
16.09 ms outside the preceding measured call. Typical vendor cost is not blame.

## Next change

Compare the same saved device/clip with Bitwig's hosting setting changed from
Together to with Bitwig. The proxy now loads in BitwigAudioEngine rather than
BitwigPluginHost. Keep build, runner, rate/block and priorities fixed; restore the
backed-up preference if drops persist. If improved, continue to the ten-minute
editor-open check, then normal interaction. No kernel-trace repair campaign.

Execution/source/SSH/Deck: Sol6.1 xhigh; GUI: Sol6.1 high; root orchestrates.
Landing uses a separate worktree. Raw diagnostics stay outside Git.
