# PC0 pinned-source reconnaissance

```yaml
slice: PC0
selection_revision: pc0-selection-v2
reconnaissance_status: complete
implementation_authorized: false
```

## Authority and bounded question

The selection basis is commit `858c240b104e090aaed8bd23ace04fd9a0dfd20e`,
tree `8a560faf07b793de8faab91952ee6f34f15a1e73`. Original activation
`5ec7ef4f2c5f19faffcf1f2b01656d698ac7cd7a` was revised by selection-authority
commit `f00824e196d2a71667d80a9cc622c03ea2cef403`, tree
`d895a44b98264f47549d4187303b00e9e9f85982`. The revised question is limited
to read-only calls legal while the accepted AGain component remains
**Initialized**.

The earlier material discovery is confirmed: the pinned SDK marks
`getLatencySamples` and `getTailSamples` as **Setup Done** calls. Reaching that
state requires `setupProcessing`, and the pinned `AudioEffect` implementation
stores a new process setup. The revised selection therefore excludes all
three calls rather than disguising a state mutation as observation.

## Exact source inspected

The SDK root remains commit `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`,
tree `38343890fd1a0cedd48b7ec80ef17da15231b6c8`; its `pluginterfaces` and
`public.sdk` submodules are respectively
`4f547e8e102b47de4a8b8aaf343c73b700786372` and
`586dc5e6c8012c3e4b01c79389375cbe96bdb1da`. Readback used these exact Git
objects:

| Pinned path | Git blob | Raw SHA-256 |
|---|---|---|
| `pluginterfaces/vst/ivstcomponent.h` | `e20ef5f429349bdccf55367d45051fcdb0ff97c5` | `cc587e34c009388d4187948c14a651df1481a01920abe895023094ca5b13faee` |
| `pluginterfaces/vst/ivstaudioprocessor.h` | `2a5428ceb3fd532a1e509a4a6a6dae4cda5191a3` | `6289b19c8300fb381da7688414fae52d5ca139371f910830532204b860bd6549` |
| `pluginterfaces/vst/vstspeaker.h` | `ee46d4a3d67492a81bfbdb6065e3de9f95916a14` | `f7942918bc52e78c2f061f77d2d28761f8a7b38a364cc3985bc22da21c8418d4` |
| `public.sdk/source/vst/vstbus.h` | `c2c3ea8205792563ecee410ffdc0469e3528fd4c` | `33bdebce779e2f38232eb79b41f378875e1bebf32e4e4650622ad5154256fde5` |
| `public.sdk/source/vst/vstbus.cpp` | `964a8c7a36b3f1ecdc2d2b1d8127dc14abc265e0` | `eec322a783ba364ea6a09895db7bdad91c61baa88e0c003c8adb7b1edec79e47` |
| `public.sdk/source/vst/vstcomponent.h` | `73eae47281bff80f888ed236259a7da93b7b511e` | `752df5165f99b7e9ab5cfba7b1420a5f6262bcbda50dd487564955d193971062` |
| `public.sdk/source/vst/vstcomponent.cpp` | `d1dced4a441d35b73717da26eea0ada97406a874` | `e9d8e5b4e25d319e378b8c8d547f34a3aa01f733c9ed3b60ddbe4c73237d7968` |
| `public.sdk/source/vst/vstaudioeffect.h` | `818cc4f3357c15c6f9a5ba649dbc70e87ced688f` | `d61a3f92770bcab1b6dfafd49ea9935de8576ab92e251569c5bc2756835c35d9` |
| `public.sdk/source/vst/vstaudioeffect.cpp` | `2c3535be777ec777c5f066e42807092168019310` | `997ff3d44bb9927def01d26c23a2aa11331d974921ee4a78b02e7345449e705b` |
| `public.sdk/samples/vst/again/source/again.h` | `061c0d4afb5f59410e75681d9998871f32fb1e72` | `304b289e902c302928e2a3ebdc117eeef6af2781855445712f515301d4295e23` |
| `public.sdk/samples/vst/again/source/again.cpp` | `4676454679e37f188b99c2ec6e6def6b825da173` | `05ff84588eac6ced18f26bba4e98632b28a2b5ee9d8c00340139fcc6b0efb00b` |

Repository inspection covered the accepted WC0/WA0 component session and
event writer plus the accepted DX0 driver, identity, custody, Deck execution,
normalization, supervision, result admission, and renderer paths named by the
selection.

## Contract findings

`IComponent::getBusCount(MediaType, BusDirection) -> int32` and
`IComponent::getBusInfo(MediaType, BusDirection, int32, BusInfo&) -> tresult`
are UI-thread/Initialized calls. `IAudioProcessor::getBusArrangement(BusDirection,
int32, SpeakerArrangement&) -> tresult` admits Initialized, and
`canProcessSampleSize(int32) -> tresult` admits Initialized or Connected. All
PC0 calls therefore fit the existing Windows scanner main thread without
setup, activation, or a processing thread.

AGain initialization adds, in order, `Stereo In` and `Stereo Out` with
`kStereo`, then `Event In` with one channel. The helper defaults are `kMain`
and `kDefaultActive`; no event output is added. `getBusInfo` writes the requested
media type and direction, then channel count, bounded name, bus type, and flags.
AGain explicitly returns `kResultTrue` for both `kSample32` and `kSample64`.

The expected normalized pre-setup contract is therefore:

- counts `(audio/input, audio/output, event/input, event/output) = (1,1,1,0)`;
- audio input index 0: `Stereo In`, 2 channels, `kMain`, default-active true,
  control-voltage false, arrangement `0x0000000000000003` / `kStereo`;
- audio output index 0: `Stereo Out` with the same structural fields and
  arrangement;
- event input index 0: `Event In`, 1 channel, `kMain`, default-active true,
  control-voltage false; no arrangement call;
- both selected sample sizes supported.

That is exactly 4 count, 3 information, 2 arrangement, and 2 sample-size
calls: 11 total. These are source expectations; only a later authorized live
transaction can establish them for the accepted binary/runtime fixture.

`SpeakerArrangement` is an exact 64-bit bitset. The pinned header's
seventh-order ambisonic constant uses all 64 bits, so a blanket “unknown high
bit” rejection would be false. PC0 can instead retain all bits, require the
population count to equal the matching audio `BusInfo.channelCount`, and attach
a deterministic recognized-layout label where exact (AGain is `kStereo`).

## Insertion and DX0 integration

The coherent insertion point is inside the accepted component-session source:
after `AudioProcessorInterfaceLease` acquires the live interface and before it
releases that reference. One bounded `PreSetupProcessingContractCensus` in the
existing component-session `.h/.cpp` pair can borrow the initialized component,
audio-processor interface, and event writer. No new C++ file, CMake target,
fixture, thread, or owner is needed.

DX0's six identity-domain algorithms remain sufficient, but its registry is
closed to `wa0-positive-regression-v1`. PC0 must add the closed plan
`pc0-pre-setup-processing-contract-v1`, a 14-record complete-source roster, the
existing 17-record Windows transitive build roster, the existing seven-record
Deck import/execution closure, and the two-record renderer closure. The accepted
fixture identity is reused unchanged. Source/artifact/handoff details remain
driver-owned; no identifier becomes an operator input.

Resolved unknowns: no setup-state call is needed; the event-bus flags are
default-active only; both sample sizes are supported; the truthful call count
is 11; no live negative fixture, new process owner, CMake path, or environment
path is required.

No live fixture was inspected. No repository product source, workflow,
artifact, custody state, Deck state, runtime, plug-in, DAW, or protected fixture
was executed or mutated during this reconnaissance.
