# Current task: keep the usable Pure LoFi Buffered 512 configuration

## Goal

Pure LoFi plays in Bitwig on the Deck at 512 frames / 48 kHz without missing
blocks or graph errors through thirty minutes of normal use and offline export.
Preserve projects, licensed state, pinned runner and machine headroom.

## Works now

The unchanged installed build, Together/Buffered 512/editor closed, completes
1,800 seconds with Moonlight connected, transport and mixer interaction:
zero missing blocks, Bitwig ERR+0 (1 to 1), recorder ERR0, largest gap 15.33 ms.
All 168,747 DAW calls succeed: median/p99/max 0.038/0.056/0.095 ms.
One offline WAV export succeeds (675 offline calls); Bitwig and both hosts
survive. Normal quit preserves projects, publications and preferences.
Editor OPEN with Moonlight disconnected for the full 600-second capture is
also clean: zero missing blocks, ERR+0, recorder ERR0, largest gap 16.96 ms.
Buffered 512 stays selected. No rebuild or scheduling change for either run.

## Still broken

Editor OPEN while Moonlight streams remains the failing configuration:
two missing 512-frame blocks, Bitwig ERR+2, largest gap 20.15 ms.
The Wine host's busiest threads are lvb-audio (13.5%) and host.exe (11.7%);
its next host.exe thread is 9.1%. sh_opt0..2 belong to Wine host, not wineserver.
Retained ps percentages are per-thread lifetime averages.

## Most likely cause

Moonlight/Sunshine capture and video encoding load with the animated editor,
about 80%. Disconnecting the stream removes the misses with the editor open;
closing the editor also stays clean during thirty minutes of streamed use.

## Doing next

Keep Buffered 512 and the editor closed during streamed playback as the first
usable configuration on this fixture. PR #218 is merged; Run 2 is deprioritised
by the operator, so its supervisor switch remains uninstalled and untested.
Raw captures stay outside Git. Any editor-open mitigation needs a new instruction.

Source/SSH/Deck: Sol6.1 xhigh; GUI: Sol6.1 high; root orchestrates.
