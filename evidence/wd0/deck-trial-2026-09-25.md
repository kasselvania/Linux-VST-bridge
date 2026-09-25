# WD0 Steam Deck trial-mode result — 2026-09-25

This is a bounded managed-Windows-DAW result. Private installer, prefix, project,
account, and diagnostic bytes remain on the Deck. The source and installed
generations are distinct facts.

## Exact installation and ownership

| Fact | Retained result |
|---|---|
| Source used for installed WD0 package | PR #163 head `8b2f97e5f9e34e53ea62c490f57de93da15a652e`, tree `367bda952402761bea3fa53a89d2ed743915a2a7` |
| WD0 package generation | `ea1b4468223479a6864321c57a6c61ce3023f415b02731a328c56e1085fab2d2` |
| Installed manager/frontend software generation | `9d9b6ba09c4318781bf0b010d91745343a2c650e507af9a0d989b19749a53811` |
| FL workspace | `762bafec213cefe91dbe14d67ee1b1c6`; same environment ID and root before and after reinstall |
| Selected official FL installer | Advertised release `26.1.5.5618`, SHA-256 `87b2f0fe47fa443b6e7e904fb904df11ee0c3adfa83581818016018840b7dd84` |
| Current FL executable | `FL Studio 2026/FL64.exe` under the workspace prefix; SHA-256 `22467af39a0ef64efb43cce02dc121245dbb006277e401f416d6c3f4b7b79243` |
| Observed executable file version | The executable's `FileVersion` and `ProductVersion` string resources both read `26.1.5.5618`; the manager's optional observed-version field remains unset |
| Selected workspace runner | `proton-11.0-2c-fl-crypt32-order-v1`; environment revision 2 |
| Audio observation | No backend/rate/buffer observation was recorded in workspace status; no ASIO or physical MIDI qualification follows from this result |

The original schema-1 workspace was `Uninstalled` at revision 12. Read-only
migration preserved its first installation operation
`322f31d398949fd2f98ddc5dcdf6fe1f` and clean uninstall operation
`295edb5f1e6e7ada74cd82392376ada6`, with their exact receipts. The new
manager-owned install used a different operation,
`6ac36cd84240e1eb8fcccd014b76b994`, and persisted schema 2 without
changing the workspace ID, prefix, project, preference, or export roots. The
new installer's outer launcher exited 199 with
`outer_nonzero_stage_unknown`; exact FL files were discovered and retained as
`needs_user_action`, not falsely recorded as a clean installer exit.

A later normal managed launch has exact operation
`b40032e6dd6cc4c63f3ec738643e5fd2` and a completed receipt with zero
owned processes and confirmed cleanup. The current readback projects the
workspace as `ready`, with no active owner or useful current failure. Historical
installer failure remains in the installation history. The operator reports FL
Studio functioning in demo mode. A 54,814-byte `.flp` exists in the workspace's
Image-Line project directory; this establishes a saved file, not licensed
project recall.

## Native-bridge isolation

The native registry was byte-identical before manager replacement and after
FL installation: SHA-256
`01e29055a5b22bf0c007b4e2215ddf14acb7c74fbf890c0bc8437c0ca4df1c77`.
The later canonical operator snapshot reported six selected native products
(Pigments, Pure LoFi, Efx FRAGMENTS, Serum 2, Blackhole, Kontakt), zero DSP,
zero maintenance, zero pending transactions, zero stale transports, and no
cleanup uncertainty. Serum 2 still selects `x11_touch_routing_v2`; Serum 2 FX
remains a separate unpublished attention item. Manager/frontend package
replacement changed its own software generation; it did not turn FL into a
native publication or consume a bridge DSP lease.

## Trial-mode scope

Licensed reopening of a saved `.flp` is deferred while FL remains in trial
mode. A fresh application launch or fresh-session playback does not prove
project recall. No trial restriction is bypassed. Third-party Windows VST3 use
inside FL belongs to WD1 and is not established here.
