# RPI2 FEX scalar tie code-generation gate

Base: `7cbf2490c601884ed84ef15ac79922c89ff22830` (approved bridge observer repair, draft PR #156).
Base tree: `c7f84e39cf7768afeaaecc79e99707964cc1a648`.
Branch: `codex/rpi2-fex-tied-scalar`. The canceled multicore/UI checkout and PR #156 are separate and remain untouched.

## Primary claim

Decide whether adding `"TiedSource": 0` to only FEX's `VFAddScalarInsert`, `VFSubScalarInsert`, `VFMulScalarInsert`, and `VFDivScalarInsert` preserves supported scalar behavior and reduces relevant emitted ARM64EC code. A build or DLL hash difference alone does not satisfy this claim.

## Authority and fixture

Basis: `AGENTS.md` slice, real-time, fixture and evidence laws; `GOVERNANCE.md` “What evidence means” and “Cost and safety”; `docs/ARCHITECTURE.md` “5.2 Runner builder and inventory” and “15. Test architecture”; `docs/experiments/RPI2.md` “Critical-path profile of the 256-frame Pigments candidate” and “Remaining worker kernel cost and emitted ARM (bounded follow-on)”; the 2026-09-24 device-agent handoff.

Pin FEX source `82510eb452b258959ef982be58a9c3c1bafc82a4`. Reuse the already staged matched Release ARM64EC builds and original pinned Wine/Proton/UMU integration on the Pi 5. Candidate A is unmodified self-built FEX; candidate B contains only the four IR hints. Use one source-owned x64 scalar fixture before considering any proprietary plug-in run.

## Scope and stop

In scope: the four-line FEX patch, source-owned scalar correctness cases, exact loaded-module identity, bounded JIT code reading, instruction-body comparison, concise sanitized evidence, and private reversible runner selection. Files in this branch are limited to `CURRENT_SLICE.md`, `rpi2/fex-vfscalar-tiedsource.patch`, the focused fixture/reader/comparator under `rpi2/`, and `evidence/rpi2/` records.

Correctness covers single and double precision add/subtract/multiply/divide, legacy and VEX scalar forms, destination aliasing, live first source, preserved upper lanes, representative single precision edge values and register pressure. Compare low-lane and upper-lane results to known expected values as well as A/B. Count full-vector moves, scalar inserts, vector stack spills/reloads and instructions in identical source-owned JIT blocks.

If B is incorrect, reject it. If B does not reduce the relevant generated code or adds offsetting spills, park it without a Pigments performance run. Only a correct relevant code reduction permits the separately specified bounded Pigments code-reader check; only that check could admit a later physical A/B/B/A. No governor, UI, multicore, kernel, Wine yield, plug-in, preset, audio-setting, activation or working-runner change belongs to this slice.

## Completion and nonclaims

The source-owned fixture passed for both A and B. The 25 captured instruction bodies were equal: 9 full-vector moves, 25 scalar inserts, 0 vector stack spills/reloads and 882 emitted instructions in each candidate. This triggers the early stop. **Decision: correct on the exercised fixture, but no relevant code improvement; park B.** The retained Pigments hot loop and physical A/B/B/A were not run. There is no speed, capacity or delivery claim.

The packaged runner and private selection were restored to their original module hash. The failed full-copy attempt's partial directory was removed; build caches, A/B modules and private diagnostic logs remain staged. Detailed sanitized identity and results are in `evidence/rpi2/fex-vfscalar-tiedsource-codegen.json`.
