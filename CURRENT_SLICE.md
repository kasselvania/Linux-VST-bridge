# Current task: Part A hosting comparison completed

## Works now

The unchanged denormal installation plays the same Pure LoFi / Little Pleasures
project at 512/48k. Four Together priority samples, ten seconds apart, are retained
privately. The matched Within Bitwig capture completes 600.000126 seconds;
all 69,697 lifetime calls export completely, with 56,249 successful N512 calls
inside the active window. Together is restored, the project bytes are unchanged,
Bitwig exits normally and the test session confirms retirement. No build or install.

## Still broken

Within Bitwig retains six 512-frame stereo-zero spans (3,072 frames), +6 Bitwig
graph errors and +2 recorder errors. Baseline is eight blocks and 19.4 ms maximum
call spacing; Within reaches 21.221 ms. Recorder errors make the lower block count
an unclean comparison, not an established improvement. Dependable audio still fails.

## Most likely cause

Host audio scheduling, moderate confidence. Together's Bitwig audio-1 through
audio-8, Audio Task workers, data-loop and eight PluginsThreadPool workers are TS.
The plug-in host's ap3-transport is RR5; no Bitwig FF thread appears. Main/UI and
remote plug-in threads are TS; several support threads are batch class B.
Within's actual callers are audio-1 through audio-8, still OTHER0 throughout the
sampled census. Its longest arrival interval includes 18.759 ms outside the
preceding measured call. Removing separate plug-in hosting does not remove it.

## Doing next

Part B waits for the operator to say #216 is merged. GitHub reports it merged,
but no pull, rebuild, installation, caller-grant check or offline export occurred.
Then install the native/manager/supervisor update, inspect caller effective or
unavailable/refusal records, repeat Together for ten minutes and export offline.
Full captures, logs, private analysis and restoration records remain outside Git.
