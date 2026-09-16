# IS4 — Manager-owned installer capability policy

Base `60eb1736e52b4894ecabd499489471aab8685a79`, tree
`9bb4d62f2d16dc1e0fcb1a177d655e6a0f8c28bf`. IS3's source-owned evidence is
accepted. Native Access fallback and historical causality remain unproved.

## Contract and ownership

Operator model 5 adds `installer_start_with_policy`, with exact offered onboarding
identity and a closed PowerShell enum: `inherited` or `intentionally_unavailable`.
`required_interpreter` and arbitrary environment values are rejected. Existing
`installer_start` remains the unchanged schema-2 inherited launch, without policy
metadata. The UI offers a separate clearly labelled intentional-absence action.
It uses the same inactivity, stale-token, initial-transaction, unit, cgroup, exact
Focus/Stop, cleanup and operation-bound service recovery owners.

`installer_policy::bind` constructs schema-3 installer specs. The binding includes
operation, complete environment/revision, exact installer, full software-record
SHA-256 and exact manager/supervisor/ownership artifacts. It does not mutate any
software, environment, import or historical attempt. The supervisor verifies the
binding and executing session/ownership identities before bootstrap. It applies
only after successful prefix initialization, immediately before target launch.
Requested policy and effective environment identity are separate in the result;
effective launch configuration alone does not prove loader or vendor behavior.
Bounded public receipts contain hashes and lengths, not inherited environment
values. Private operation specs retain the existing exact location authority.

## Override law

Inherited mode returns all environment entries unchanged. Intentional absence
splits WINEDLLOVERRIDES on semicolons. At most one exact case-insensitive
`powershell.exe` name may occur. A single rule is replaced in place with
`powershell.exe=`; otherwise that rule is appended. Every unrelated segment is
retained byte-for-byte, including order, case, whitespace and values. Multiple
exact PowerShell rules, a mixed comma-separated name list containing PowerShell,
a malformed exact rule, NUL or an oversized value refuse. Wildcard/other module
names are never rewritten. No operator-HOME override, registry load-order change,
runner replacement, fake interpreter or vendor-name rule is introduced.

## Rollback law

Policy is an **operation-only field**, not a new Software or Environment field.
The existing strict software schema is intentionally unchanged. Explicit rollback
selects only the older immutable manager/frontend/supervisor generation. Its new
installer launches have the original schema-2 shape and cannot inherit a prior
operation's choice. Old terminal operation/spec/evidence records remain retained.
Rollback is still subject to existing inactive/positive-cleanup setup gates; a
live policy operation cannot be handed to an older supervisor. The generated
setup regression selects a legacy package after policy construction, parses its
software.json through a strict pre-policy schema, verifies absence of a policy
field and proves the newer generation and retained operation remain identical.

## Source-owned proof

`tools/is4` seals a fixed package derived from the accepted IS3 custody law.
The unchanged x86 consumer/report/environment parser come directly from IS3.
The package additionally binds the production Rust policy source, its generated
example adapter, the built Linux policy owner, Cargo-linked source commit/tree,
and the fixed Zig/Rust recipe and compiler identities. The example calls the
same production `bind()` function used by manager launch; it is not installed.

The staged supervisor verifies the sealed package, exact pinned runner and current
installed generation, then constructs one disposable software-generation binding
for the staged Rust owner/session/ownership bytes. It does not install this
software. Three fresh prefixes exercise inherited / intentionally unavailable /
restored inherited through the production policy, with the existing IS3 seam used
**only for bounded observation**, never its Unix override selection. Comparator
requires exact root, full identity equality, policy binding, cleanup, baseline
false-success and refusal-126/fallback-43 behavior. No synthetic result can replace
the physical generated consumer. Prior IS3 evidence remains unchanged.

## Scope and gate

No Native Access, commercial product, installation, interpreter, policy deployment,
registry, publication, existing prefix or project mutation. No native audio/runtime
behavior changes; AP10 excluded. Return a draft PR for source/generated review.
Only later acceptance, merge, deliberate immutable installation and separate human
authorization permit one linked vendor continuation, outside this implementation.
