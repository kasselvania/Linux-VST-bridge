# Official VST3 validator results

The exact official validator built from the locked SDK was 1026664 bytes with
SHA-256 `cccae776eb87fbbbf6ac9c34c78bf1548937c48db3843e0cf0f12d312650466f`.
The SDK checkout and validator source were not edited.

For the app-sandbox run, this executable was not selected by name alone. The
probe required a v2 build receipt from historical implementation commit
`e2d87997dc200cd49ec0e6fc2a0e7646b7cfed62`, tree
`3c60d7f48b78677f65c9f2879aa49a2c0d913db6`; recomputed and matched source
manifest schema `linux-vst-bridge-hp0-build-source/v1` and SHA-256
`ee301e3e42b2381d5a9aa88ec64635482d098bc8cde50d33a57e6c397fb1e9ab`;
reverified the SDK, class/parameter, bundle, module, manifest, validator-path,
and validator-hash fields; rehashed the executable on the host; and rehashed it
again inside the Bitwig application sandbox before execution. All validator
hashes were the exact value above.

The same v2 receipt was reverified at the final evidence-only amended PR head.
That check passed because the canonical source manifest remained exact, not
because historical repository commit/tree fields were treated as current
identity.

Sanitized command shape for each build:

```text
timeout --signal=TERM --kill-after=5s 180s \
  flatpak run --user --command=<VALIDATOR> --unshare=network \
  --nofilesystem=host --nofilesystem=home --filesystem=<REPO>:ro \
  org.freedesktop.Sdk//25.08 <BUILT_BUNDLE>
```

| Result | Build 1 | Build 2 |
|---|---|---|
| Exit | `0` | `0` |
| Passed | `47` | `47` |
| Failed | `0` | `0` |
| Skipped | `unknown` — validator emitted no skipped counter | same |
| Classification | `passed` | `passed` |

The complete bounded summary covered editor-class census (no custom editor),
bus and parameter scans, MIDI mapping/learn, units/programs, terminate and
initialize, unit structure, valid 32-bit and 64-bit state transitions, bus
consistency and activation, stereo arrangement, sidechain handling,
suspend/resume, note expression, keyswitch, process-context requirements,
threaded-process invocation, silence flags and processing, parameter flushes
with missing/zero-channel buffers, variable block sizes, sample formats,
bypass persistence, stereo and deliberately rejected mono arrangements, and
block/sample automation exercises for both sample formats. Every registered
test ended in `[Succeeded]`.

The path-normalized logs differed only in repetition count for the existing
informational `Not all points have been read via IParameterChanges` notice:
twice in build 1 and once in build 2. Their normalized SHA-256 values were
`3082cb32ccebc36e3424ad914fbb0244ae1dd88b6acc4d2880f333f87fbfff58` and
`a9e237d96fbb2a6a3a34e4c35b2a1099f21abdc505523c700c2fca2b83cc52cc`.
No passed/failed outcome or retained class, bus, parameter, state, or
sample-size fact differed.

Factory and class readback matched the source lock:

```text
vendor = Kasselvania Research
name = LAB Host Probe
category = Audio Module Class
version = 0.1.0
sdkVersion = VST 3.8.1
cid = 6F4E7A5392E54B54A98AD6F714E0C201

name = LAB Host Probe Controller
category = Component Controller Class
cid = B9C42F0736C34E218E5A71D40C8F1B62
```

The validator observed one stereo audio input and output, gain default `0.5`,
and bypass default off. Informational output also recorded no snapshots, a
bypass string round-trip notice, and that the fixture consumes the final point
per block rather than every sample-accurate automation point. These notices did
not become failures; HP0 makes no sample-accurate automation claim.
