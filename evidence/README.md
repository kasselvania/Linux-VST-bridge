# Evidence

This directory holds sanitized retained evidence for exact repository slices and fixtures.

Suggested layout after a slice owns it:

```text
evidence/
  <slice-id>/
    BASIS.md
    fixture.json
    commands-or-harness.md
    results/
    negative-results/
    ACCEPTANCE.md
```

Every evidence packet states:

- exact commit/tree and slice;
- host/distro/kernel/session;
- DAW/install mode/runtime branch;
- runner/environment/proxy/protocol/profile identities;
- plug-in build and license-channel classification without secret material;
- operation and expected result;
- observed result;
- claim level and explicit nonclaims;
- sanitization performed.

Never commit:

- commercial installers or plug-ins;
- full prefixes/environments;
- license or offline-authorization files;
- serial numbers;
- credentials, cookies, tokens, account names, or emails;
- paid presets/sample libraries;
- personal projects/audio;
- unredacted home paths, hostnames, IP addresses, or device serials.

Use hashes and local fixture descriptors to refer to proprietary artifacts. A screenshot is supporting evidence, not a substitute for structured protocol/state/audio results.
