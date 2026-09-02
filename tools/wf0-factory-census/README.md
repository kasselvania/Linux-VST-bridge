# WF0 factory census harness

This directory implements only the authorized WF0 V7 proof transaction. The
private one-job GitHub Actions workflow builds the repository-owned Windows
x86_64 scanner, the bounded 22-module fault family, and the exact pinned SDK
AGain fixture twice with Visual Studio 2022, MSVC v143, and Windows SDK
10.0.19041.0. The authenticated Mac control plane admits the exact workflow
artifact by numeric run and artifact ID, creates the self-contained source
bundle, and transfers both independently verified handoffs. The Steam Deck
imports only those exact bytes, supervises fresh Runtime 4 / Proton 11 scan
environments, and renders the fixed sanitized 14-file evidence packet.

Plane entry points are deliberately separate:

```text
python tools/wf0-factory-census/build.py acquire ...
python tools/wf0-factory-census/artifacts.py mac-custody ...
/usr/bin/python3 tools/wf0-factory-census/artifacts.py import-source-handoff ...
/usr/bin/python3 tools/wf0-factory-census/run.py all ...
```

Source and artifact admission are explicit stopped gates. The mutating Deck
exercises then run as one fresh `all` transaction; before every exercise the
harness reproduces the clean detached 26-record implementation-source identity
and rehashes the content-addressed artifact set. Generated artifacts, handoffs,
and raw process streams remain private and untracked. A source or workflow blob
change invalidates every later receipt.

The scanner stops at class metadata. It exposes no command option or operation
for class instantiation and never launches Bitwig or Serum. The accepted WR0
environment is read-only protected state and is neither adopted nor repaired.
The Deck performs no GitHub operation.
