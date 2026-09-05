# AP2 — Host-loaded native VST3 offline bridge

Authority: CURRENT_SLICE.md on codex/ap2-native-vst3-offline-bridge, issue #52.
AP1 remains accepted. Diagnostics are acceptance-ineligible; fresh AP2 acceptance
is a review candidate, never a relabelled AP1 observation.

The private AGainOfflineBridge.vst3 ELF bundle exposes one processor, FUID
84E8DE5F92554F5396FAE4133C935A18, bound to retained Windows module SHA-256
60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f.
No controller, editor, saved-state or normal DAW publication is supplied.
The factory labels the limited processor `AGain Offline Bridge`,
`Fx|OnlyOfflineProcess`. It accepts offline float32 stereo at 48000 Hz,
maximum 1..256 frames, at most 64 nonempty calls per activation.

The SDK host selects fresh binary-fraction inputs after activation. Its thirteen
nonempty blocks preserve the ten AP1-A2 cases and add an empty parameter queue,
a zero-frame gain flush followed by audio, and same-channel in-place buffers.
Every actual returned sample must equal independently computed input × gain,
with zero tolerance. Gain defaults to retained AGain's 1.0; ParamID 0 accepts
one finite normalized point at offset zero. Missing/empty queues retain it.
ParamID 2 may only remain off. Invalid requests cannot change cached gain or
consume a sequence. Output masks are independent full-width zero-sample claims.
A fresh unload/reopen checks default gain on 19 further stereo samples, using
a fresh mapping, session capability and Windows AGain instance.

The C++ shell uses the official SDK module/factory/AudioEffect boundary. Rust
owns mapping, authentication and single-slot sequencing extracted from AP1.
C ABI 1 in native-vst3-proxy/include/ap2_backend.h has opaque generation handles,
synchronous borrowed buffers and bounded scalar errors. Mutating calls refuse
concurrent entry. Panics are caught at the ABI; no native asynchronous worker
retains a host buffer. No audio samples are calculated by the native bridge.

Wire AP1 1.1 is unchanged. AP2 uses minor 2 with additional paired messages:
Activate/Activated 8/9 (request maximum u32), Start/Started 10/11,
Stop/Stopped 12/13, Deactivate/Deactivated 14/15. These use the current next
audio sequence without consuming it. Only Process/Done consume a sequence.
Windows setup/activation occur on its owner thread after Activate; processing
and start/stop occur on its distinct worker. Stop is acknowledged after join;
Deactivate precedes component termination/module unload; Closed follows unmap.
A zero-call start/stop is valid. A second attachment is refused.

The existing external supervisor supplies the private session binding and
launches Windows only after activation creates the endpoint. Discovery and
initialize alone create no endpoint. The host never uses private bridge exports.
The supervisor retains and closes the first session before releasing the
out-of-band reopen gate. The second module load receives a fresh exact binding.
Timeout, disconnect or invalid output permanently fails that session; there is
no replay, reconnect, stale output or local DSP fallback. Defensive clearing is
limited to validated writable output spans and accompanies a failed VST3 result.

Timeouts retain AP1's 180-second endpoint acceptance, 10-second Hello, 5-second
audio request/response and 10-second close bounds. AP2 lifecycle acknowledgement
uses 10 seconds; Windows control operations retain 5 seconds. The supervisor's
existing process watchdog/owned cleanup still applies. The host waits at most
30 seconds for the external clean-reopen gate. These are offline bounds only.

Builds retain pinned VST3 SDK 3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96,
Freedesktop SDK 25.08 GCC 15.2.0 and a PIC x86_64-unknown-linux-gnu Rust static
library, linked into the native ELF. The native SDK host is executed directly
on the declared Deck, proving its installed libc compatibility. Windows uses
the existing exact MSVC producer. Proton 11.0-2c/Runtime 4 is explicitly bound
through the AP1 runtime verifier; no protected environment is migrated.

One AP2 batch consists of the positive activation plus one clean reopen. Limits:
six Windows producer attempts, ten diagnostics in one campaign, two acceptance
candidates of one batch each. Every consumed reservation remains retained.
Local substituted-peer tests consume no live reservations. Independent host
words, Windows lifecycle, containment, runtime and protected-state readbacks
are retained through the existing checkpoint/atomic publication path. AP2's
bounded checkpoint ceiling is 512 KiB to retain both activations; it is
independent of normalization and admission. No prior evidence is rewritten.
