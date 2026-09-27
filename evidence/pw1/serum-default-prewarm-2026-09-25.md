# PW1 — Pi Serum default-state prewarm result

This is a private Raspberry Pi 5 **source-candidate** result. It is a repair
of the repeatable first-user-note delivery gap for the named Serum 2.1.5
default-state fixture. It is not an installed-service change, a general Serum
or Pi qualification, or an explanation of the older unnamed failed preset.
The normal installed service and retained PI-R/FN1 candidates were unchanged.

## Exact comparison

The fresh A arm used FN1 [PR #174](https://github.com/kasselvania/Linux-VST-bridge/pull/174)
head `1399a9655b6d1c8abb99f7058c8b34d28bcb76ab`, Pi native executable
SHA-256 `d56daa0787b00c4f558a8347743c6810b33bc9ad1e6ae62e9b6bb828f3939e98`.
That executable was built from FN1 diagnostic source
`39a4cdddca21fe6f3416f79d98e3a82e7fc1899a`; the later FN1 head changed
documentation only.
The B arm added only the PW1 opt-in startup sequence at
`870b39153d447ad88e9cbd7fce86bfe283987b40` (tree
`90f1df485f4f39592536f6b9f966affc49bd2991`), Pi native executable
SHA-256 `1bd2aa9e7dedc6510867428678bee1b4c9f953ba1a286d478dec5c3cdbe05da9`.
Both ARM binaries used Pi Rust/Cargo 1.98.1 with
`--release --locked --features jack-runtime`, thin LTO, one codegen unit,
panic-abort and the same dependency lock. The unchanged Windows production
host SHA-256 was
`b41eb3696e49f26ae715ac0656d9c0ced1383b3824008d998117f4dfbe404f24`.

Both arms used the same private Serum 2.1.5 module SHA-256
`501e7bb3dd9cafe416b3412df3d4e084c01b7468201e5690b9009d7ecd4e5283`,
the same 44,280-byte captured default state SHA-256
`41fa2e07500a7a8d46f9d833353f5a31cf4eb38117ea1fa5071ff607bf8a90ab`,
the same `ge-proton11-7-aarch64-umu1.4.4-slr4.0.20260914.260627`
Proton/FEX runtime, closed editor, JACK 48 kHz/512, protocol 12/map v1,
256-frame quantum, 512-frame reserve, ondemand governor, normal scheduling
and affinity, and the retained six-second two-note fixture. Its first note
occurred at frame 48,000; all runs reported six note-ons, five note-offs and
zero MIDI-write failures. `LVB_AP10_TRACE=1` and the production Windows host
trace were identical in A and B. Only the private evidence directory and
the B native `--prewarm` option differed. No runtime, vendor setting,
transport capacity, timeout or recovery policy changed.
The two private configuration files differed only in `evidence_directory`.

PW1 runs the shared `Instance::process` path while JACK is open but not yet
active. It submits one pitch-60/channel-0 note-on, its matching note-off and
six more 256-frame silent blocks, each waiting for worker completion. It
discards that audio, stops and deactivates the host, restores the exact same
saved state again, and requires a byte-for-byte identical returned state
envelope before normal activation and JACK readiness. An error or deadline
refuses readiness. The ordinary audio callback contains no new work.

## Physical result

The execution order was B1, B2, fresh A, B3. All runs used the same source-owned
fixture and completed 571 measured JACK callbacks and 1,142 measured audio
requests after readiness. The first B run used the final source and binary;
an earlier exploratory B run with a zero startup system timestamp is retained
privately but excluded from this table and claim.

| Arm | First-user-note Windows `process()` | First-user-note full service | Missing / expired frames | Delivered frames | Nonzero output samples per channel | Retirement |
|---|---:|---:|---:|---:|---:|---|
| Fresh A | 32.850 ms | 33.800 ms | 2,304 / 2,304 | 289,536 | 65,270 | clean |
| B1 | 1.866 ms | 2.577 ms | 0 / 0 | 291,840 | 66,164 | clean |
| B2 | 2.122 ms | 2.900 ms | 0 / 0 | 291,840 | 66,164 | clean |
| B3 | 1.953 ms | 2.599 ms | 0 / 0 | 291,840 | 66,164 | clean |

In fresh A, the retained gap records identify 1,024 startup frames at source
positions 0–768 and another **1,280 first-note frames** at positions
50,944–51,968. The note-bearing request was epoch 1, sequence 201, pitch 60,
channel 0, note identity 1, offset 128. A later pitch-64 note call took
1.937 ms in Windows. The B runs had zero gaps, zero JACK xruns and zero bridge
faults. Their first user note was epoch 2, sequence 210, source position
50,944, pitch 60/channel 0/identity 1, offset 128. It was processed and
published rather than replaced by silence. The 2,304-frame increase in
delivered audio exactly matches A's missing-frame count at this fixture.

The cold call was **moved into startup**, not made faster. B's muted
epoch-1 note call took 42.906–43.631 ms inside Windows `process()` and
47.720–48.702 ms for its full native request service. The complete eight-block
prewarm, lifecycle stop/deactivation and second state restore took
136.320–140.059 ms before readiness. Those numbers are from the identical
diagnostic configuration, not an uninstrumented release benchmark. No CPU
saving is claimed. The existing FN1 attribution supports FEX ARM64EC cold
translation/JIT activity during that call; PW1 does not identify the internal
FEX routine or prove that every preset has the same cold path.

In every B run, the second restore returned all 44,280 state bytes exactly;
the four subsequent control readbacks matched A: Main Vol 0.5, Filter 1 On 0,
Filter 1 Freq 0.5 and Macro 1 0. A separate B session processed 187 callbacks
with no MIDI after readiness: neither output channel had a nonzero sample,
and it reported no missing frames or faults before clean retirement. This
checks for a residual prewarm voice at idle. All A and B sessions returned
`PI_CLEAN_SHUTDOWN`; no owned Windows cohort or JACK client remained active.

## Boundary and next action

The byte-for-byte state envelope and control readbacks establish the selected
serialized state identity. They do not prove equality of every vendor-hidden
random or round-robin state, bit-exact audio, or long-session behavior. This
fixture has no separately retained audible recording. The extra startup
operation may be unsuitable for some vendors or presets, so the option stays
off by default and no installed selection changes. First-note delivery in
this exact default-state fixture is corrected; older Deck/Ubuntu residual
deadline misses, demanding Serum presets and the old unidentified incident
remain unresolved. Raw logs, private state, binaries and exact host paths
remain under the owner's private Pi candidate directory.

After FN1 and PW1 receive review, the next separate engineering action is to
reconcile BR1-style bounded exceptional recovery onto the shared core without
importing its old callback-side policy. It should not be used to restart Serum
for the deterministic cold note that this startup experiment handles.
