# AP1 sample comparison

Linux read and independently checked 1344 float32 samples; maximum absolute error 0.0.

Eight changing blocks, 1/16/63/256 frames, gains 0.5/0.25/0.75, distinct stereo inputs and silence. Linux chose the retained seed only after Windows Ready. Actual input and returned words are retained.

One host, AGain instance, shared mapping and authenticated loopback control connection reused throughout. Control carried no audio. Both views witnessed the same backing mapping before module load.

Owner-thread setup/activation; distinct processing thread stopped and joined before deactivation, interface/component retirement and unload. Windows unmapped before Closed; Linux unmapped after Closed. Both owned groups empty, disposable stage absent, protected state unchanged.

Offline Linux/Windows numerical round trip only. No DAW, realtime, editor, commercial plug-in, state, event transport or arbitrary plug-in memory-safety claim. AP0 remains accepted pending AP1 review and merge.
