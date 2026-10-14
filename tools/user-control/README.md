# Portable program fixture

This original Win32 fixture displays build A or B in its title and window,
reads UTF-8 `resource.txt` beside its executable, and displays launch arguments.
The title-bar Close action and `WM_CLOSE` exit normally with status zero.
It has no installer, registration, licensing, network access or persistent writes.
A missing, empty, invalid UTF-8 or oversized resource fails visibly; keep it
below 2 KiB.

From the repository root, build both versions with an existing MinGW compiler:

```sh
fixture_build=/tmp/lvb-user-control-fixture
mkdir -p "$fixture_build/a" "$fixture_build/b"
x86_64-w64-mingw32-gcc -std=c11 -Wall -Wextra -Werror -municode -mwindows -DFIXTURE_BUILD=1 tools/user-control/portable_fixture.c -o "$fixture_build/a/portable-fixture.exe" -luser32
x86_64-w64-mingw32-gcc -std=c11 -Wall -Wextra -Werror -municode -mwindows -DFIXTURE_BUILD=2 tools/user-control/portable_fixture.c -o "$fixture_build/b/portable-fixture.exe" -luser32
cp tools/user-control/resource.txt "$fixture_build/a/resource.txt"
cp tools/user-control/resource.txt "$fixture_build/b/resource.txt"
```

With an existing Zig installation, use the same input and output paths with:

```sh
zig cc -target x86_64-windows-gnu -std=c11 -Wall -Wextra -Werror -Wno-unused-command-line-argument -municode -Wl,--subsystem,windows -DFIXTURE_BUILD=1 tools/user-control/portable_fixture.c -o "$fixture_build/a/portable-fixture.exe" -luser32
zig cc -target x86_64-windows-gnu -std=c11 -Wall -Wextra -Werror -Wno-unused-command-line-argument -municode -Wl,--subsystem,windows -DFIXTURE_BUILD=2 tools/user-control/portable_fixture.c -o "$fixture_build/b/portable-fixture.exe" -luser32
```

Outputs belong outside Git;
on the homelab, use a folder under `/mnt/Storage2/LinuxPluginBridge` instead of
the `/tmp` example after confirming the mount is present.

Through the installed manager's program chooser, remember A's executable with
its folder. This first chooser slice launches without extra arguments. Run, focus
and close it; verify the displayed version and resource. After it
closes, replace only A's executable with B's executable at the same selected
path. The manager should notice the change; use its adoption action, restart
the manager, then run and close the remembered program again. The window must
now identify B and still display the sibling resource. Source compilation alone
does not establish that this installed GUI workflow works.
