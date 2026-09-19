# Enter iLok credentials from SSH

This temporary operator helper lets you provide credentials directly on the Deck
while an authorized GUI worker operates the already-open iLok sign-in form.
The helper is bound to that exact supervised iLok process and window.

After the helper and login window have been prepared, run this on the Deck over
an interactive SSH connection:

```sh
~/.local/bin/lvb-ilok-login
```

Both the User ID and password prompts hide what you type. Paste each value and
press Enter. Wait for the ready message, then tell the assistant the handoff is
ready. Do not put either value in the SSH command or send it in chat.

The helper stores the pending input with owner-only permissions under the Deck's
temporary runtime directory. It expires after 15 minutes. The GUI worker selects
each field and invokes a fixed helper command; credentials travel to keyboard
input through stdin, not the clipboard, command arguments or tool output. The
password handoff is deleted before its single typing attempt. The worker leaves
“Remember User ID and Password on this machine” unchecked.

To cancel pending input:

```sh
~/.local/bin/lvb-ilok-login clear
```

To check the handoff without showing its contents:

```sh
~/.local/bin/lvb-ilok-login status
```

If the iLok process/window changes or closes, the helper refuses input. Have the
window prepared again instead of changing its binding yourself. This helper
does not activate a license or establish plug-in compatibility.

For the authorized GUI worker: use `verify` before input, select User ID and
invoke `fill-user`, then select Password and invoke `fill-password`. These
commands print status only. Never read the temporary credential file. Keep the
remember option unchecked; sign-in submission is a separate authorized GUI
action. A refused or uncertain typing attempt is not a reason to replay input
blindly. Source: [handoff helper](../../tools/ilok-login-handoff.py).
