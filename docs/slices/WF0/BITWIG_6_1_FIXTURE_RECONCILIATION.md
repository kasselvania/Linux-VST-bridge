# WF0 Bitwig 6.1 Protected-Fixture Reconciliation

## Authority

```yaml
schema: linux-vst-bridge-wf0-fixture-reconciliation/v1
slice_id: WF0
repository: kasselvania/Linux-VST-bridge
reconciliation_date: 2026-09-01
amendment_basis_commit: 15523c69567d24b256cb3c65cb6f06bfa07854be
amendment_basis_tree: a6e2fc8c7a564f50e9033094fde847387478de62
authority_phase: reconnaissance_and_design
implementation_authorized: false
operator_amendment_authorization: I authorize an ammendment.
```

The authorization applies to the immediately preceding proposal to return WF0
to the design gate and prepare a narrow Bitwig `6.1` stable-fixture correction.
It does not approve a design revision, implementation, installation, build,
live Windows exercise, evidence claim, pull-request merge, or successor slice.

The operator identified the former Bitwig `6.0.11` installation as participation
in a beta program and intentionally updated to the stable `6.1` release. This
record treats that update as an intentional fixture correction. It does not
silently weaken the exact-fixture law.

## Trigger and stop result

The authorized WF0 implementation preflight established the exact branch and
basis and then stopped at the protected-state snapshot with:

```text
WF0_PROTECTED_FIXTURE_DRIFT
```

The accepted helper expected the historical Bitwig identity:

```text
version: 6.0.11
application commit: 7b66aed37386ffff99e40bbd486f90ceee7ac1d5c59508c7952094122cebf50e
```

The mismatch was detected before any source edit, toolchain installation,
build, validator, Bitwig, Wine, Proton, Runtime, scanner, or fixture workload.
The MinGW extension remained absent. The dedicated implementation worktree and
the primary Deck checkout were restored to clean state after removing only
Python bytecode cache directories created by the read-only inspection.

## Exact live readback

The readback was bounded and content-free. No hostname, network address,
credential, account identifier, project content, plug-in state, vendor content,
registry content, raw process identifier, or private configuration bytes are
retained.

### Host posture

| Fact | Exact result |
|---|---|
| Machine class | Steam Deck Galileo |
| Architecture | `x86_64` |
| Operating system | SteamOS `3.8.16` |
| SteamOS read-only | enabled |
| Bitwig process | absent |

### Current Bitwig installation

| Fact | Exact result |
|---|---|
| Ref | `app/com.bitwig.BitwigStudio/x86_64/stable` |
| Version | `6.1` |
| Application commit | `8a048e733e74dda8b897339436153a2d5df952f29d362dfcaca9d8e0d6f6c231` |
| Origin | `flathub` |
| Installation scope | system |
| User-scope shadow | absent |
| Runtime ref | `org.freedesktop.Platform/x86_64/25.08` |
| Runtime commit | `bd44a6230581917d04f89812a4c21090c304d390edb73995af1c2f9fd8abf4e8` |
| User override SHA-256 | `1b4a6a6ed688f69dd3c36dcac8db008c5a41ed52170ea3e23dee984b0aae6a1e` |
| System override SHA-256 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| Permission-output SHA-256 | `c7f5a34dce104cc3d347dcaf89e135cc5ad73b891101d7587f78423858ad5c73` |

Only the Bitwig version and application commit differ from the V2-era accepted
snapshot. The runtime ref and commit, installation scope, absence of a user
shadow, and both override-byte identities remain exact.

### Other WF0 prerequisites

| Fact | Exact result |
|---|---|
| `origin/main` | `15523c69567d24b256cb3c65cb6f06bfa07854be` |
| `origin/main` tree | `a6e2fc8c7a564f50e9033094fde847387478de62` |
| Paused implementation branch basis | same commit and tree |
| Design blob | `d618cbf6b397f10947d50fd4824cd3e06ef55726` |
| Design SHA-256 | `f323e2b429c4f91c1821488d980b249901b8d82c3cceaa7bf9be9d874a455e24` |
| Forbidden workloads | all exact families absent; ordinary Steam helpers separate |
| Runtime/runner digest | `2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547` |
| Accepted WR0 environment identity | `d3ed38d7ed53e9a5973cbf22fc504f2479dbdd15616fe1bfc1d5e1cd0bf5f4c4` |
| WR0 contract-source digest | `c6543004fbcd70252393f0e0cab4f9ad7a31e85f433ce3e903e690244cc79541` |
| VST3 SDK root commit | `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96` |
| VST3 SDK root tree | `38343890fd1a0cedd48b7ec80ef17da15231b6c8` |
| VST3 SDK recursive submodules | all seven exact and clean |
| Freedesktop SDK commit | `b90ed309cc1d505dea48b6a2121c5dcfac22868120eee643b0596d31f96b9bb8` |
| MinGW user/system prestate | absent / absent |
| Approved MinGW remote commit | `f15a5a88eb09557f645860bfcb3c4a6bc267683fce06cb68436bd76376fca694` |

No competing implementation pull request existed for the paused WF0 branch.

## Historical evidence law

The following artifacts remain truthful historical records and must not be
rewritten to say `6.1`:

- SR0 evidence describing the observed `6.0.11` fixture;
- HP0 build, publication, validator, and Bitwig sandbox evidence;
- HP1 discovery and exact native instance-admission evidence;
- WR0 and WR0A evidence that protected the then-current `6.0.11` installation;
- HP0, HP1, and WR0 helper constants bound to those accepted historical runs.

Git history alone is not the preservation mechanism during WF0. Those tracked
paths remain outside the implementation envelope and must be byte-identical at
the final WF0 head.

HP1 proves its exact `6.0.11` matrix only. This reconciliation provides no
Bitwig `6.1` discovery, scan, instance, project, audio, GUI, state, or
compatibility evidence.

## V3 design consequence

The existing `ProtectedFixtureSnapshot` owner must distinguish two facts:

1. **Historical evidence identity** — exact tracked packet and governed-helper
   Git identities remain unchanged and retain their `6.0.11` statements.
2. **Current protected application identity** — Bitwig `6.1`, its exact app and
   runtime commits, installation scope, user-shadow absence, and override-byte
   hashes are read immediately before and after WF0 mutation.

The current snapshot belongs in the already approved WF0 implementation paths.
WF0 must not call the historical WR0 `capture_fixture()` helper as current
Bitwig authority because that helper correctly remains bound to its accepted
`6.0.11` run. No HP0, HP1, WR0, WR0A, Bitwig, or Flatpak source/evidence path is
added to the WF0 implementation envelope.

A mismatch in either historical identity or current protected application
identity remains `WF0_PROTECTED_FIXTURE_DRIFT`. The implementation must stop and
must not repair, downgrade, upgrade, launch, or reconfigure Bitwig.

## Unchanged design

The reconciliation does not change:

- the primary claim or claim ceiling;
- the official pinned AGain positive fixture;
- the exact MinGW, VST3 SDK, Runtime 4, Proton 11, or WR0 identities;
- the ten-owner architecture or process/thread topology;
- the scanner lifecycle, call-attribution law, or census schema;
- the 40 tracked implementation paths or 26-path source manifest;
- the 35 proof rows or 23 blocked results;
- the external mutation envelope;
- the prohibition on Bitwig execution; or
- any successor-slice authority.

## Result

```text
FIXTURE_RECONCILIATION_COMPLETE
IMPLEMENTATION_AUTHORIZED=false
FRESH_V3_REVIEW_REQUIRED=true
EXACT_OPERATOR_APPROVAL_REQUIRED=true
```
