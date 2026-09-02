# WF0 evidence sanitization

The packet contains
only public source/dependency identities, safe-relative artifact paths, hashes,
typed public workflow/artifact metadata, bounded normalized factory facts,
proof dispositions, and explicit nonclaims. It contains no raw process
identifier, command line, private absolute path, hostname, network address,
credential, key, token, cookie, account identifier, license material,
Steam compatibility-prefix state tree, proprietary state, installer, plug-in binary, SDK source, build
tree, source bundle, artifact archive, or generated PE.

All fourteen paths were UTF-8/NUL/size checked, hash-closed, roster-closed, and
scanned for private paths, credentials, network identifiers, binary signatures,
and staged proprietary content. The public AGain factory contact field is SDK
metadata from the open reference fixture, not a user identifier.
