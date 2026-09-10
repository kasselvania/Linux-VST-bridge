# Third-party notices

This document identifies principal third-party software and interfaces used to
build or run Linux Audio Compatibility Bridge. It does not relicense any
third-party work. The authoritative license for each dependency remains the
license distributed by its copyright holder.

## Steinberg VST 3 SDK

The build uses, but does not vendor, the official Steinberg VST 3 SDK:

```text
version: 3.8.1
repository: https://github.com/steinbergmedia/vst3sdk
commit: 3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96
license: MIT
copyright: Steinberg Media Technologies GmbH
```

The repository verifies the exact SDK checkout and its license metadata before
building. Source distributions do not include the SDK. Binary distributions
that contain SDK code must preserve the following notice:

> MIT License
>
> Copyright (c) 2026, Steinberg Media Technologies GmbH
>
> Permission is hereby granted, free of charge, to any person obtaining a copy
> of this software and associated documentation files (the "Software"), to deal
> in the Software without restriction, including without limitation the rights
> to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
> copies of the Software, and to permit persons to whom the Software is
> furnished to do so, subject to the following conditions:
>
> The above copyright notice and this permission notice shall be included in all
> copies or substantial portions of the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
> IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
> FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
> AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
> LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
> OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
> SOFTWARE.

### VST trademark

VST is a trademark of Steinberg Media Technologies GmbH. The public product
name used by this repository is **Linux Audio Compatibility Bridge**. References
to VST or VST3 in source paths, technical documentation, protocol descriptions,
and the historical repository slug are descriptive references to the supported
plug-in format, not a claim of affiliation or ownership. No VST Compatible logo
is bundled. Any later use of Steinberg branding or the VST Compatible logo must
follow Steinberg's then-current usage guidelines.

## Rust dependencies

The exact Rust dependency graphs are recorded in the committed `Cargo.lock`
files. The current direct crates are:

- `serde` and `serde_json`;
- `sha1` and `sha2`;
- `libc`.

Those crates are distributed under MIT and/or Apache-2.0 terms. Their transitive
dependencies remain under the terms shipped by their respective authors. The
project's `MIT OR Apache-2.0` license applies only to project-owned work and does
not replace dependency notices.

A distributable release must generate and include a complete notice set from
the exact locked dependency graph rather than treating this human summary as an
exhaustive binary-attribution manifest.

## System and runtime components

The project builds against or interoperates with components that are not
relicensed or redistributed by this source repository, including:

- Linux system libraries such as X11/XCB and the C runtime;
- Microsoft Windows SDK and MSVC build tools used by hosted CI;
- Wine/Proton and Steam Linux Runtime components supplied separately by their
  respective projects and distributors;
- the host DAW and operating-system services.

Each remains governed by its own license and distribution terms.

## Proprietary plug-ins and user material

Commercial plug-ins, installers, presets, sample libraries, opaque plug-in
state, account data, authorization material, and DAW project files are not part
of the project license and must not be redistributed with this repository.
Evidence retained in the repository is intentionally limited to bounded,
sanitary identities and results rather than proprietary binaries or license
payloads.
