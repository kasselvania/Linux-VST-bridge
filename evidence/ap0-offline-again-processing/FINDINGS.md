# AP0 sample comparison

Compared every returned sample: 96 float32 samples; maximum absolute error 0.0.

48000 Hz, kOffline, 16 frames per stereo block. Gains 0.5 and 0.25 on distinct left/right patterns, followed by a silent block at gain 0.25. Exact comparison passed, inputs/guards unchanged, no nonfinite or unwritten samples, and silence flags correct.

Setup and activation ran on the owner thread. A distinct processing thread stopped and joined before deactivation, interface/component retirement and module unload. Exit zero, complete in-process shutdown, owned-process containment, stage absent, protected state unchanged.

Offline numerical correctness only: no real-time, DAW, proxy, IPC, live audio, editor, state, commercial plug-in or arbitrary-memory-safety claim.

Acceptance verified, awaiting technical-lead review. PC0 remains accepted until AP0 review and merge.
