# SR0 sanitization

## Collection minimization

- Raw normalized data exists only transiently under the ignored `evidence/raw/sr0-steam-deck-fixture-reconnaissance/` path and is removed when capture exits.
- No environment dump, registry dump, prefix copy, settings dump, full process list, process command line, display socket name, device serial, browser state, or proprietary file content is retained.
- File candidates are represented only by sanitized path, type, size, modification time, and SHA-256 for regular files.
- Directory traversal is bounded by declared roots, depth, filesystem, row count, time, and bytes. Every traversal retains completion, timeout, failure, or truncation status.
- Search roots are canonicalized beneath the canonical home and rejected if any existing component is a symlink; rejected roots are not traversed.

## Replacements

- Real home path -> `<HOME>`
- Username -> `<USER>`
- Hostname -> `<HOST>`
- Email, IPv4, IPv6, MAC, UUID-like, and external-media label patterns -> typed placeholders

The word “Deck” remains only where it denotes the Steam Deck fixture or an exact project/branch identifier; username-bearing path and account contexts are replaced.

## Claim discipline

A missing Serum/Xfer match is retained as `not_found_in_bounded_locations` only when every contributing root and command completed within time, byte, row, recursion, filesystem, symlink, and candidate bounds. Any incomplete contributor propagates to `unknown`/`search_incomplete`; it is never generalized to absence. Legitimate prose such as “license channel” remains because it contains no license material.

## Raw/staging posture

The capture checks that the raw path is ignored and contains no tracked file before and after packet generation. `hashes.sha256` covers every retained packet file other than itself.
