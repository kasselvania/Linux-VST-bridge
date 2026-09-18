# First application launch after dependency recovery

The human launched software rendering once. Operation
`06a22a3c22fc2c1922a756104f573d0d` proved exact dependency readiness, then
created the exact application root. The human saw no Native Access window.
The launcher exited 2; service retirement and process cleanup passed without
forced cleanup. The bridge and two keepers recovered. Protected records changed
only by the expected current application-operation selection.

The application log records renderer `launch-failed`, exit code 72, and
`ERR_FAILED (-2)` loading its renderer during this startup interval. This shared
vendor log is a diagnostic source, not an exact Windows-generation join. The
operation-owned Wine stream has zero drops and no admitted causal facts.

The failed stream also contains 93 Fontconfig errors, 62 saying out of memory;
the prior content-visible B stream contains none. Those messages concern config
loading at `/etc/fonts/fonts.conf`; they do not establish host RAM exhaustion.
The exact executable additionally contains version markers Electron/43.2.0 and
Chrome/150.0.7871.129. Upstream Chromium names sandbox launch error 72
`SBOX_ERROR_DISABLING_APPHELP`, returned when querying/writing a suspended
child process's PEB fails. This directs a source-owned API reproduction; it does
not authorize disabling the sandbox or assert the exact failed system call yet.

No second application launch, renderer switch, reinstall or production mutation
was performed during diagnosis.
