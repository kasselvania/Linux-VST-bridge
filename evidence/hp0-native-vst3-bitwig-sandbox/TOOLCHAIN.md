# HP0 user-space toolchain

The exact SDK was `not_installed` in both user and system scope at HP0
pre-flight. An existing user `flathub` remote advertised the exact branch, so
the sole authorized toolchain mutation was performed:

```text
flatpak install --user --noninteractive flathub org.freedesktop.Sdk//25.08
```

Post-install identity:

| Field | Observed value |
|---|---|
| Classification | `installed_by_slice` |
| Ref | `runtime/org.freedesktop.Sdk/x86_64/25.08` |
| Version | `freedesktop-sdk-25.08.16` |
| Commit | `b90ed309cc1d505dea48b6a2121c5dcfac22868120eee643b0596d31f96b9bb8` |
| Parent | `31507d1bfc6a5bd5835aeb3cb07e7b7aeea69332502ecce1c671cea96ed3ebc2` |
| Origin | `flathub` |
| Scope | user |
| Architecture / branch | x86_64 / `25.08` |
| Installed size | 1.7 GB |
| Free space before install | approximately 706 GB |

Versions observed inside that exact SDK:

| Tool | Version |
|---|---|
| GCC / G++ | `15.2.0` |
| CMake | `4.4.2` |
| Ninja | `1.13.2` |
| pkg-config | `2.5.1` |
| Git | `2.55.0` |
| file | `5.47` |
| readelf | GNU Binutils `2.47.20260726` |
| ldd / glibc | `2.42` |
| ShellCheck | `not_installed` |
| clang-format | `not_installed` |
| clang-tidy | `not_installed` |

The configuration guard read `/.flatpak-info` and required the exact ref and
commit above. It also required GNU C++ `15.2.0`, Linux x86_64, and a 64-bit
compiler. No host compiler, package manager, root privilege, read-only-mode
change, or additional tool install was used.
