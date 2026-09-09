# Linux Audio Compatibility Bridge

A managed bridge for using supported Windows audio plug-ins in native Linux DAWs without making musicians administer Wine prefixes, proxies or runtime versions.

Experimental engineering preview, not a consumer-ready release. Working repository name; not affiliated with Bitwig, Valve, Steinberg, Arturia, Xfer Records or another vendor.

## Accepted baseline: AP12 installed musical workflow

AP8–AP11 established the independent Linux VST3 proxy, actual Windows processing under a selected runner, state/recall, audio and note input, bounded returned events/parameter feedback, transient-gap continuation and same-instance detached editors. AP12 [#79](https://github.com/kasselvania/Linux-VST-bridge/pull/79), reviewed at `66430ac40d5a398ea2a0943b330b1056dc7ecb03`, adds persistent managed publication and automatic startup for the installed Arturia pair:

```text
Bitwig notes → Arturia Pure LoFi → Arturia Efx FRAGMENTS → output
```

The recorded workflow covers browser-selected instrument/effect roles, both real editors, recorded and replayed automation, edited-state saving, actual Deck reboot and recall without setup tooling, and effect removal while the instrument continues. It is accepted by [review 5149097123](https://github.com/kasselvania/Linux-VST-bridge/pull/79#pullrequestreview-5149097123) for the exact installed candidates, not all Arturia versions or original-installer authorization.

State capture now preserves genuine vendor bytes when supplemental parameter readback is unavailable. Completed ordinary save refusal stays distinct from process death. Fresh vendor-editor access can precede persistence availability. Reporting failure cannot prevent cleanup/transport retirement. Minimal independent fault status preserves an unfinished request before containment; detailed profiling remains optional.

A reproduced LoFi editor-removal crash was located in the pinned runner's Windows UI Automation implementation. The selected per-process accessibility override prevents that measured crash without changing the vendor binary, delay or timeout. It is a scoped workaround with an accessibility limitation, not a general runtime repair.

See [AP12 results](docs/AP12.md), [musical pair/reboot/removal](evidence/AP12/musical-pair-reboot-removal.json), [cause and workaround](evidence/AP12/delivery-cause-and-repair.json), and [fault-status contract](docs/AP12-FAULT-STATUS.md). Earlier [AP11](docs/AP11.md), [AP10](docs/AP10.md), [AP9](docs/AP9.md) and [AP8](docs/AP8_RESULT.md) evidence remains unchanged.

## Operating limits

Retain **512 added bridge frames per proxy**. At the recorded 48 kHz setup, LoFi adds 48 vendor frames and FRAGMENTS 192: the serial total is **1,264 frames / 26.333 ms**, matching Bitwig's 26.3 ms display. Bridge-added delay is 1,024 frames / 21.333 ms; device/DAW latency is additional. Mean service time and reported latency are not physical round trip or worst-case guarantees.

Short audio gaps remain even at 512. The reproduced removal crash does not explain the older untraced timeout or every scheduling outlier. Both selected Arturia processes disable Windows UI Automation/screen-reader integration; musical VST automation works. Historical resource-integrity warnings keep their original artifact/environment scope.

Float32 and documented bounded stereo/bus/event paths are implemented. Arbitrary multichannel/dynamic routing, every SDK interface, float64, native Wayland editor hosting and general customer-hardware reliability are not claimed. Operator-supplied modules, original installer outputs and authorization posture remain separate.

## Next direction

The next selected direction is AP13: improve delivery consistency on this real chain and validate a lower bridge-delay option. [#72](https://github.com/kasselvania/Linux-VST-bridge/issues/72) supplies the performance backlog. Lower latency must come from demonstrated delivery improvements and honest host-block timing, not a changed label, concealed gaps or larger hidden buffers. Read [CURRENT_SLICE.md](CURRENT_SLICE.md) on the prepared implementation branch for active authority.

[#80](https://github.com/kasselvania/Linux-VST-bridge/issues/80) tracks Advanced-panel expansion/redraw; [#74](https://github.com/kasselvania/Linux-VST-bridge/issues/74) tracks known-endpoint-loss notification; #73 retains the historical native-close crash and #77 Serum's lawful vendor-access/editor qualification. These do not require replaying all old campaigns.

## Start here

Read [AGENTS.md](AGENTS.md), [CURRENT_SLICE.md](CURRENT_SLICE.md), relevant production code and the [design dossier](docs/DESIGN_DOSSIER.md). Use ordinary implementation, proportionate testing and one reviewed PR. Reuse [SSH/Moonlight](docs/DECK_REMOTE_DESKTOP.md); launch Bitwig from Applications. Remote control is development tooling, not a playback dependency. Preserve dirty work and the operator's live project. Keep intended installations available and clean only temporary experiments.

`native-vst3-proxy/` is the Linux SDK proxy and Rust backend; `native-audio-client/` contains transport; `windows-factory-probe/` is the Windows SDK host; `bridge-manager/` owns registration/publication and installed supervision. Rust is primary, with C++ at SDK/platform edges. Proprietary binaries, presets, license data and credentials stay out of version control.

The independent bridge remains selected. Maintained Wine/Proton work, including the inspected `giang17/wine` fork, may supply a suitable runtime underneath it; it is not a yabridge pivot. Identify a matching defect before a coherent reversible runtime comparison. Full manager polish, general vendor updates/rollback, final product name and repository-wide licensing/distribution remain future decisions.
