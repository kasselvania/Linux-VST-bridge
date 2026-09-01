# WR0 evidence sanitization

Classification: `passed`.

- Evidence has an exact allow-listed roster, UTF-8/NUL/size checks, parseable JSON, and a verified `hashes.sha256`.
- Private home/repository paths use `<HOME>`/`<REPO>`; the real hostname and Linux user are absent.
- PIDs, command lines, process environments/maps, raw session/launch directories, prefix files, registry content, Windows machine identifiers, SIDs, Steam account data, credentials, vendor state, proprietary binaries, and Serum content are absent.
- Runner/runtime identity is metadata and SHA-256 only; no installed binary or source payload is copied.
