# Deck desktop in an operator's browser

This optional maintainer helper uses KDE Desktop Sharing and noVNC. It does not
capture the operator computer's mouse or keyboard. The Deck desktop and cursor
are shared with anyone using the Deck itself.

Run on the operator computer with Python 3.12 or newer, SSH access to the Deck,
and Zig installed:

```sh
python3 tools/deck-browser/start.py deck@steamdeck
```

Open the printed local URL in the app's browser preview. KDE may request sharing
on a fresh server start; ordinary browser reconnects reuse the approved session. The viewer starts with control disabled; use
**Enable control**, **Pause control**, **Release keys**, or **Disconnect**.
Keep automated input focused in an expendable scratch window until testing is
finished. Input affects the shared desktop's focused application.

Leave the launcher running. **Ctrl-C stops the server, tunnel and gateway and
removes their temporary files.** Sessions expire after 30 minutes by default;
`--minutes` accepts 1 through 120. A reconnect to the local page starts view-only.

The helper extracts matching Krfb/libvncserver packages from the Deck's existing
SteamOS package repositories and checks their hashes against its installed
package database. It does not install system packages. A small process-local
wrapper restricts Krfb to loopback addresses; SSH carries the connection. The
operator computer downloads pinned noVNC and creates a temporary websockify
environment. Credentials, configuration, state and logs stay in private temporary
directories, with no KWallet use or autostart changes.

Focus loss releases keys, and the gateway sends modifier releases before closing
a browser connection. Loss of the SSH connection is a separate failure boundary;
the remote service's time limit remains its backstop. This helper is operator
tooling, not an installed-product prerequisite or an audio stream.

For an explicitly authorized agent to handle an ordinary KDE sharing request
without the operator present:

```sh
python3 tools/deck-browser/consent.py deck@steamdeck snapshot
# Inspect the printed private PNG, identify Share, and use its actual coordinates.
python3 tools/deck-browser/consent.py deck@steamdeck click X Y WIDTH HEIGHT
```

This uses existing Spectacle and the Deck user's existing `/dev/uinput` access;
it changes no permissions. Take a fresh screenshot before clicking and verify
the resulting desktop. Never use it for account, activation or administrator
prompts. Delete the temporary screenshot after inspection. The helper creates
and destroys its input device for each click, with button release before exit.
