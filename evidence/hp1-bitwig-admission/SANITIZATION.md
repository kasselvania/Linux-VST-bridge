# HP1 evidence sanitization

Classification: `passed`.

- Raw session, nonce, PIDs, full `/proc` maps, command lines, environments, Bitwig projects, account/license data, raw databases, and proprietary plug-in content are not retained.
- The private home prefix is `<HOME>` and the repository is `<REPO>`.
- Only exact relevant strings and bounded metadata from Bitwig-owned state are retained; cap, timeout, access, and parse failures propagate to `unknown/search_incomplete`.
- Process IDs are replaced by session labels; start ticks and identity digests preserve launch distinction without treating a PID as durable identity.
- Every evidence file is UTF-8 text or intentional JSON, and `hashes.sha256` covers every file other than itself.
