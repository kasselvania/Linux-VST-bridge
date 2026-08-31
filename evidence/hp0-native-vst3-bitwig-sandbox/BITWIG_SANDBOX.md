# Bitwig Flatpak sandbox explicit-path load

Bitwig fixture:

| Field | Observed value |
|---|---|
| App | `com.bitwig.BitwigStudio` |
| Version | `6.0.11` |
| App commit | `7b66aed37386ffff99e40bbd486f90ceee7ac1d5c59508c7952094122cebf50e` |
| Scope / branch / arch | system / stable / x86_64 |
| Runtime | `org.freedesktop.Platform/x86_64/25.08` |
| Runtime version / commit | `freedesktop-sdk-25.08.16` / `bd44a6230581917d04f89812a4c21090c304d390edb73995af1c2f9fd8abf4e8` |
| User-scope app shadow | absent |

Declared filesystems include `host`, `/usr/lib`, and the PipeWire runtime
socket. Existing user overrides remained:

```text
[Context]
filesystems=/usr/lib/

[Environment]
VST_PATH=/app/extensions/Plugins/vst;=/usr/lib/
CLAP_PATH=
VST3_PATH=
```

Before and after execution, the repaired probe required the exact system app
ref, version, app commit, runtime ref, and system runtime commit above. It also
required the exact expected user/system override hashes and selected the app
with `flatpak run --system`. Synthetic wrong-app-commit and
wrong-runtime-commit cases both failed before sandbox entry without mutating
the real installation.

The application command was replaced by `sh`; the Bitwig application was not
launched. Inside that exact app sandbox, the exact published bundle and module
were visible and readable, the module SHA-256 was
`3abaa8a2051ea179e28986cc6fbad4cde2d0d5e9a844b18aca91cdf7c59e69d7`,
the ELF machine bytes were x86_64, `file` agreed, and `ldd` resolved the complete
dependency closure. The exact official validator was bound to the v2 build
receipt's historical commit `e2d87997dc200cd49ec0e6fc2a0e7646b7cfed62`,
tree `3c60d7f48b78677f65c9f2879aa49a2c0d913db6`, build-source manifest
SHA-256 `ee301e3e42b2381d5a9aa88ec64635482d098bc8cde50d33a57e6c397fb1e9ab`,
and validator SHA-256
`cccae776eb87fbbbf6ac9c34c78bf1548937c48db3843e0cf0f12d312650466f`.
The sandbox rehashed that exact executable before it loaded the published
bundle, exited `0`, and again reported 47 passed / 0 failed. A modified
validator copy was deterministically refused before execution.

Alternate sandbox build and publication roots are accepted only with explicit
test mode and one canonically contained test root. A build root outside that
root and paths with symlinked ancestors were refused before validator or
sandbox execution; escaped sentinel storage remained unchanged.

The sandbox reported exact effective
`VST_PATH=/app/extensions/Plugins/vst;=/usr/lib/`, with both `VST3_PATH` and
`CLAP_PATH` present but empty. Flatpak
also emitted its existing warning that `/usr/lib` is a reserved path and is not
shared via that filesystem entry. This did not prevent the explicit published
bundle load.

Override evidence:

| Scope | Before SHA-256 | After | Result |
|---|---|---|---|
| user | `1b4a6a6ed688f69dd3c36dcac8db008c5a41ed52170ea3e23dee984b0aae6a1e` | same | byte-identical |
| system | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` | same | byte-identical empty output |

Exact-name process checks were empty before and after. This is an explicit-path
sandbox-load result only; it is not Bitwig discovery, scan, instantiation, UI,
project, audio, or musical-operation evidence.
