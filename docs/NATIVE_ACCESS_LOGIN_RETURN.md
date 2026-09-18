# Native Access browser return

The browser authenticated the user, then Linux had no handler for the vendor's
`native-access:` return. A successful web page did not prove application login.
The repair registers a scheme-only desktop handler and a default association,
preserving unrelated associations and refusing an existing foreign default.
Desktop-specific preferences may take precedence; deployment verifies the actual
selected handler before another human sign-in.

The handler admits only the current exact registered Native Access operation,
installed software generation, bound application root and live supervisor socket.
It cannot start a new application session. The already-running Windows adapter
retains the exact executable and primary process handles. It creates a verified
secondary instance of that same executable with the unchanged renderer mode and
one opaque URI argument. This uses the application's ordinary second-instance
handoff; it neither implements nor interprets the vendor's authorization protocol.
A secondary exit zero means dispatched, not authenticated.

The URI has a closed 2 KiB extent, is passed through an ephemeral same-user socket
and inherited pipe, and is never written into a request, result or log. The manager
callback process disables core dumps and Linux dumpability. Before delivery, raw
application capture and Windows diagnostic parsing stop for the remainder of the
operation: vendor output could echo a code without a recognizable URI prefix.
The result explicitly marks renderer evidence incomplete. Exact original root,
Linux process custody, SCM retirement and cleanup remain independently available.
Vendor-owned account state is not copied or exported.

The primary operation has no independent elapsed-time deadline. Normal close or
exact manager Stop owns retirement; SCM stop and cgroup cleanup remain bounded.
Returns refuse if no admitted session is live, if identities change, if the URI is
malformed or the receiver is outside the exact cgroup. Delivery is bounded and not
retried automatically. A refused/expired return requires a fresh human sign-in.

Generated qualification uses fake return bytes and a disposable Windows fixture,
never the user's account. Real Native Access login completion remains a separate
human observation. No sandbox, renderer flag, runner or dependency change is part
of this repair.
