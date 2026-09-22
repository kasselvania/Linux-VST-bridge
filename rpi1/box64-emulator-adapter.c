#define _XOPEN_SOURCE 700

#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#ifndef O_NOFOLLOW
#define O_NOFOLLOW 0
#endif

static int fail(const char *message)
{
    fprintf(stderr, "lvb-box64-emulator-adapter: %s\n", message);
    return 64;
}

static int box64_path(const char *argv0, char output[PATH_MAX])
{
    char resolved[PATH_MAX];
    char *slash;
    int written;

    if (realpath(argv0, resolved) == NULL) {
        fprintf(stderr,
                "lvb-box64-emulator-adapter: cannot resolve self: %s\n",
                strerror(errno));
        return -1;
    }

    slash = strrchr(resolved, '/');
    if (slash == NULL) {
        return -1;
    }
    *slash = '\0';
    written = snprintf(output, PATH_MAX, "%s/bin/box64", resolved);
    if (written < 0 || written >= PATH_MAX) {
        return -1;
    }
    return 0;
}

static int is_x86_64_loader(const char *path)
{
    const char *name = strrchr(path, '/');
    name = name == NULL ? path : name + 1;
    return strcmp(name, "ld-linux-x86-64.so.2") == 0;
}

static int is_proton_python(const char *path)
{
    static const char shebang[] = "#!/usr/bin/env python3\n";
    const char *name = strrchr(path, '/');
    char header[sizeof(shebang) - 1];
    struct stat info;
    ssize_t got;
    int descriptor;

    name = name == NULL ? path : name + 1;
    if (strcmp(name, "proton") != 0) {
        return 0;
    }
    descriptor = open(path, O_RDONLY | O_CLOEXEC | O_NOFOLLOW);
    if (descriptor < 0) {
        return 0;
    }
    if (fstat(descriptor, &info) != 0 || !S_ISREG(info.st_mode)) {
        close(descriptor);
        return 0;
    }
    got = read(descriptor, header, sizeof(header));
    close(descriptor);
    return got == (ssize_t)sizeof(header)
        && memcmp(header, shebang, sizeof(header)) == 0;
}

int main(int argc, char **argv)
{
    char emulator[PATH_MAX];
    int target = 1;

    if (argc < 2) {
        return fail("missing emulated executable");
    }
    if (box64_path(argv[0], emulator) != 0) {
        return fail("cannot derive sibling bin/box64 path");
    }

    if (is_x86_64_loader(argv[1])) {
        target = 2;
        if (target < argc && strcmp(argv[target], "--library-path") == 0) {
            if (target + 2 >= argc || argv[target + 1][0] == '\0') {
                return fail("malformed loader --library-path vector");
            }
            if (setenv("BOX64_LD_LIBRARY_PATH", argv[target + 1], 1) != 0) {
                return fail("cannot set BOX64_LD_LIBRARY_PATH");
            }
            target += 2;
        }
        if (target >= argc) {
            return fail("loader vector has no executable");
        }
        if (strncmp(argv[target], "--", 2) == 0) {
            return fail("unsupported loader option");
        }
    }

    if (target == 1 && is_proton_python(argv[1])) {
        char **forwarded = calloc((size_t)argc + 2, sizeof(*forwarded));
        int index;
        if (forwarded == NULL) {
            return fail("cannot allocate Proton Python vector");
        }
        forwarded[0] = emulator;
        forwarded[1] = "/usr/bin/python3";
        for (index = 1; index < argc; ++index) {
            forwarded[index + 1] = argv[index];
        }
        execv(emulator, forwarded);
        fprintf(stderr,
                "lvb-box64-emulator-adapter: cannot execute Proton Python through pinned Box64: %s\n",
                strerror(errno));
        free(forwarded);
        return 126;
    }

    argv[target - 1] = emulator;
    execv(emulator, &argv[target - 1]);
    fprintf(stderr,
            "lvb-box64-emulator-adapter: cannot execute pinned Box64: %s\n",
            strerror(errno));
    return 126;
}
