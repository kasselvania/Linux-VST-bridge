# Delivered host-block configuration

The disposable Ubuntu first run exposed a shared host configuration boundary:
Bitwig 6.1.1's native PipeWire backend requested a maximum of 1024 samples while
the bridge's selected buffering was 512. Typing 512 with an explicit 48 kHz
sample rate still returned to 1024. ALSA reported the device busy; JACK reported
no server. These observations do not establish a vendor-specific audio fault.

The delivery repair adds an explicit 1024-frame testing configuration. It keeps
the default at 512, leaves the DAW in charge of the device and rate, and never
silently changes the buffering of a working installation. A stopped product's
normal controls can select 1024 or restore 512. The manager verifies the exact
module, class and native digest against the selected prebuilt kit's declared
capability before allowing the larger setting. Legacy kits and binaries do not
inherit it from a manager update. Active instances prevent a change; new
activation reads the selected value. Vendor state and stable class IDs remain
separate from this preference.

The native proxy accepts a maximum of at most 1024 samples only when the
selected delay covers it. Audio still uses preallocated storage and bounded
256-frame transport chunks. The native SDK boundary reports actual vendor
latency plus the selected bridge delay. 1024 bridge frames adds 21.33 ms at
48 kHz; it does not become the historical qualified 512-frame configuration.
The setup owner verifies the reported delay, and one/two chained proxy tests
assert exact delayed sample order across full and partial host blocks. No
callback performs configuration, allocation or manager work.

Prebuilt index schema 2 declares each exact proxy's maximum bridge buffering.
Index schema 1 remains readable with its old 512-frame limit. The manager and
frontend use operator schema 14; retained request history keeps its original
schema. Restore 512 before selecting an older proxy that lacks the larger
envelope. Package-wide cross-generation rollback remains task 5.

This source repair must be exercised through normal package selection,
compatibility checking, publication and product controls before an installed
audio result is claimed. Vendor state refusal remains inspectable; no state is
fabricated and no trial or license restriction is bypassed.

The installed successor also exposed interrupted navigation during periodic
status refresh. The frontend now retains the exact matching product card while
refreshing, with actions disabled until fresh readback completes. An unchanged
state token and generation preserve the card; a changed identity discards it.
This is display continuity, not additional action authority.
