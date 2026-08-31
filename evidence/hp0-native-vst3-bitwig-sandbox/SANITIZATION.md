# HP0 evidence sanitization

Classification: `passed`

- Real username, hostname, home prefix, network addresses, MAC addresses,
  serials, account identifiers, credentials, tokens, cookies, and private
  installer/license material are absent.
- User-local paths are represented as `<HOME>`; build paths are represented as
  `<REPO>`, `<VALIDATOR>`, or `<BUILT_BUNDLE>` where a path is needed.
- The evidence contains hashes and bounded metadata, not generated modules,
  validators, SDK source, build trees, the published bundle, proprietary
  plug-ins, Serum content, registry data, or complete process/environment dumps.
- Temporary Git worktrees, dynamic test roots, transaction identifiers,
  receipts, and escaped-path sentinels are not retained. The fixed ordinary
  receipt location is represented beneath `<HOME>`.
- Searches for the actual username, actual hostname, actual home path, IPv4,
  IPv6, MAC, email, password, token, cookie, serial, license, and credential
  terms were manually reviewed. Legitimate prose such as SDK `license` and
  `license material` is descriptive and carries no license data.
- All retained files are UTF-8 text or intentional UTF-8 JSON and remain within
  the bounded evidence directory.
- `hashes.sha256` verifies every retained evidence file other than itself.
