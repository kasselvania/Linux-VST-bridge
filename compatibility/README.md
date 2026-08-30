# Compatibility Profiles

This directory is reserved for future versioned declarative profiles after a slice accepts the schema and trust model.

No profile is currently accepted or verified.

A future profile must bind exact match conditions to a bounded set of reviewed capabilities such as runner selection, environment family, dependencies, graphics/editor posture, activation classification, content roots, scanner policy, and known limitations.

Profiles must not contain:

- shell, PowerShell, Python, JavaScript, or other arbitrary executable code;
- credentials, tokens, cookies, serials, or license data;
- proprietary binaries or patches;
- DRM or authorization bypasses;
- unreviewed remote download URLs;
- destructive repair actions without product-owned preview/rollback support.

Expected lifecycle:

```text
proposed
-> locally exercised
-> evidence retained
-> reviewed
-> verified for an exact matrix
-> superseded / withdrawn / known-regressed
```
