# Signed user-space delivery

The native installer stages the canonical binary PKG0 payload under the user's
home directory. It never changes the protected operating system. Opening Setup
then uses the existing explicit Stop, Select version, Start and Restore previous
version controls. Staging a package does not select it or stop the working bridge.
Software rollback does not restore customer projects or vendor state.

## Package authority

The builder compiles a separately selected Ed25519 public key and key class into
the installer, packaged manager and packaged frontend. A key inside a download
cannot authorize that download. The signature covers the domain
`Linux VST Bridge user package v1`, key class, installer executable SHA-256 and
exact release manifest; the
manifest binds the complete payload and component roster by SHA-256. The builder
retains its private key outside the repository and artifacts.

`internal_test` is visibly identified in the installer. A customer release needs
an independently selected release key and the recorded distribution authority.
The wrapper refuses release signing while the compliance manifest leaves release,
complete notices or customer signing unselected. Internal signatures do not make
an engineering candidate a customer release.

The ELF trailer contains manifest, signature, canonical tar payload and a fixed
40-byte extent footer. Intake enforces bounded extents, exact regular-file roles,
paths, sizes, modes, digests, owner and complete directory roster. Links, duplicate
or undeclared files, changed source files and writable staged components refuse.
Replacing the launcher while retaining a valid signed payload also refuses.
Extraction uses private staging, new-file writes, a per-user intake lock and atomic
directory promotion. Insufficient free space removes only this installer's
unfinished staging directory and leaves the selected application intact.

Setup uses only the source executable's verified package root. An ordinary
selected-generation executable does not become a package-input authority.
The packaged supervisors must match the system Python minor version; intake
checks that ABI before adoption. Platform-specific builds remain necessary.
Declared native library requirements also belong to the target package.

## Build and verification

Build manager and frontend binaries with `LVB_PORTABLE_ED25519_PUBLIC_KEY` and
`LVB_PORTABLE_KEY_CLASS`, including `linux-vst-bridge-install`. Assemble the same
PKG0 component manifest and payload used for system packages. Wrap them with
`tools/portable/user_package.py`, supplying the private signer and separate public
identity. Do not ship the private key. The exact candidate command
`package-user-inspect <download>` checks signature and payload without staging or
selection. Installed qualification must also exercise the native installer and
normal Setup controls.

Build-side Rust notice collection follows the locked Linux release dependency
closure, excludes dev-only edges, verifies actual crate archives against Cargo
lock checksums, and captures license/notice texts including bundled font notices.
Workspace crates that omit root licenses use their exact archive-declared upstream
commit. Package inventory retains these URLs and text digests; no latest branch
is substituted. The SPDX inventory records upstream declarations without inventing
a concluded license. Runtime acquisition preserves upstream notices separately.

Internal36's native file-browser installer staged on Ubuntu and SteamOS without
changing selected software or protected-system files. Ubuntu's ordinary
Stop/Select/Start route selected the successor on the same populated account and
retained its plug-in registrations. The Deck preflight refused a missing
execution-pair projection for an already selected preparation kit; its working
version remains selected. This failed transition is retained in the
[installed development receipt](../evidence/self-service-delivery/internal36-installed-delivery.json).
Neither result establishes musical recall or hardware audio.

Internal37 completed the same populated Deck selection through ordinary Setup.
The separately saved Bitwig project reopened with its Pure LoFi and FRAGMENTS
settings, MIDI and automation intact and produced a captured stereo signal.
The native proxies were retained; this does not qualify successor audio changes.
Visible restoration returned the exact old application, registry and project,
but Setup then failed because selected activation required a version record that
the retained pre-package installation never had. The service remained inactive;
the full update/rollback journey failed. See the
[internal37 receipt](../evidence/self-service-delivery/internal37-installed-delivery.json).

The source successor shares one verified legacy classification between setup
and activation. Only the exact retained `session.py` / `ownership.py` layout may
lack a package record, with its private generation and all selected component
identities verified. It can then receive normal selected-service status and an
explicit Start after rollback. A missing modern record still refuses. The
status labels it `retained-installation`, never inventing a package version or
writing a generation record. Forty package-authority tests and strict manager
Clippy passed on macOS; the installed successor remains to be retested.

Source tests cover signature/class/key/payload changes, unsafe paths, duplicate
members, symlink substitution, low disk, Python mismatch, preserving selected
software and later-created projects across two staged versions. These are source
results. SteamOS populated selection, saved-project update/rollback and
physical audio still require installed results before this route is qualified.
