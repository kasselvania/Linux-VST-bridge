# AP2 sample comparison

The SDK host independently checked 1730 actual returned float32 samples; maximum absolute error 0.0.

Standard VST3 factory/component/processor interfaces only. Thirteen blocks in one persistent activation cover gain and length changes, zero gain, whole/one-channel silence, empty-queue gain persistence, zero-frame parameter flush and in-place buffers. Fresh module reopen checks default gain through another real Windows instance.

Windows owner/processing-thread transitions acknowledged the host. Both sessions stopped, joined, deactivated, terminated and unloaded; both mappings retired. Host references released; owned groups empty; disposable stage absent; protected state unchanged.

Offline processor-only bridge. Realtime/prefetch, 64-bit audio, unsupported automation/events, controller/editor and state persistence are not supported. AP1 remains the accepted frontier pending AP2 review and merge.
