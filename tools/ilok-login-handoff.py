#!/usr/bin/python3
"""One-shot, target-bound iLok sign-in field handoff."""

from __future__ import annotations

import contextlib
import dataclasses
import fcntl
import getpass
import json
import math
import os
import pathlib
import re
import resource
import secrets
import stat
import subprocess
import sys
import time
import warnings
from typing import Any, Callable, Iterator

MAX_FILE = 16 * 1024
MAX_INPUT = 512
TTL = 15 * 60
XDOTOOL = "/usr/bin/xdotool"
SYSTEMCTL = "/usr/bin/systemctl"
SYSTEMD_RUN = "/usr/bin/systemd-run"
PYTHON = "/usr/bin/python3"


class Refusal(Exception):
    pass


@dataclasses.dataclass(frozen=True)
class Context:
    directory: pathlib.Path
    proc: pathlib.Path
    helper: pathlib.Path
    uid: int
    run: Callable[..., Any] = subprocess.run
    monotonic: Callable[[], float] = time.monotonic

    @property
    def binding(self) -> pathlib.Path:
        return self.directory / "binding.json"

    @property
    def credentials(self) -> pathlib.Path:
        return self.directory / "credentials.json"

    @property
    def lock(self) -> pathlib.Path:
        return self.directory / ".lock"


def default_context() -> Context:
    uid = os.getuid()
    return Context(pathlib.Path(f"/run/user/{uid}/lvb-ilok-input"), pathlib.Path("/proc"),
                   pathlib.Path(__file__).resolve(), uid)


def _require(ok: bool, message: str) -> None:
    if not ok:
        raise Refusal(message)


def _directory(ctx: Context) -> None:
    try:
        value = os.lstat(ctx.directory)
    except OSError as error:
        raise Refusal("secure runtime directory unavailable") from error
    _require(stat.S_ISDIR(value.st_mode) and value.st_uid == ctx.uid
             and stat.S_IMODE(value.st_mode) == 0o700,
             "secure runtime directory unavailable")


def _secure_metadata(value: os.stat_result, uid: int, message: str) -> None:
    _require(stat.S_ISREG(value.st_mode) and value.st_uid == uid
             and stat.S_IMODE(value.st_mode) == 0o600 and value.st_nlink == 1
             and value.st_size <= MAX_FILE, message)


@contextlib.contextmanager
def _locked(ctx: Context, blocking: bool = False) -> Iterator[None]:
    _directory(ctx)
    flags = os.O_RDWR | os.O_CREAT | os.O_CLOEXEC | getattr(os, "O_NOFOLLOW", 0)
    try:
        descriptor = os.open(ctx.lock, flags, 0o600)
        _secure_metadata(os.fstat(descriptor), ctx.uid, "credential handoff busy")
        mode = fcntl.LOCK_EX if blocking else fcntl.LOCK_EX | fcntl.LOCK_NB
        fcntl.flock(descriptor, mode)
    except (OSError, Refusal) as error:
        try:
            os.close(descriptor)
        except (OSError, UnboundLocalError):
            pass
        raise Refusal("credential handoff busy") from error
    try:
        yield
    finally:
        fcntl.flock(descriptor, fcntl.LOCK_UN)
        os.close(descriptor)


def _read_file(ctx: Context, path: pathlib.Path, message: str) -> bytes:
    flags = os.O_RDONLY | os.O_CLOEXEC | getattr(os, "O_NOFOLLOW", 0)
    try:
        descriptor = os.open(path, flags)
        _secure_metadata(os.fstat(descriptor), ctx.uid, message)
        chunks: list[bytes] = []
        remaining = MAX_FILE + 1
        while remaining:
            chunk = os.read(descriptor, remaining)
            if not chunk:
                break
            chunks.append(chunk)
            remaining -= len(chunk)
        raw = b"".join(chunks)
        _require(len(raw) <= MAX_FILE, message)
        return raw
    except (OSError, Refusal) as error:
        raise Refusal(message) from error
    finally:
        try:
            os.close(descriptor)
        except (OSError, UnboundLocalError):
            pass


def _read_json(ctx: Context, path: pathlib.Path, message: str) -> dict[str, Any]:
    try:
        value = json.loads(_read_file(ctx, path, message))
    except (json.JSONDecodeError, UnicodeDecodeError) as error:
        raise Refusal(message) from error
    _require(type(value) is dict, message)
    return value


def _binding(ctx: Context) -> dict[str, Any]:
    message = "secure binding unavailable"
    value = _read_json(ctx, ctx.binding, message)
    _require(set(value) == {"schema", "window", "pid", "start_ticks", "unit",
                            "cgroup", "display", "xauthority"}, message)
    _require(value["schema"] == 1 and type(value["pid"]) is int and value["pid"] > 0,
             message)
    _require(type(value["window"]) is str
             and re.fullmatch(r"[1-9][0-9]{0,19}", value["window"]) is not None, message)
    _require(type(value["start_ticks"]) is str
             and re.fullmatch(r"[1-9][0-9]*", value["start_ticks"]) is not None, message)
    _require(type(value["unit"]) is str and len(value["unit"]) <= 255
             and re.fullmatch(r"[A-Za-z0-9_.@:\\-]+", value["unit"]) is not None, message)
    for key, limit in (("cgroup", 4096), ("display", 128), ("xauthority", 4096)):
        item = value[key]
        _require(type(item) is str and len(item) <= limit
                 and all(character.isprintable() for character in item), message)
    _require(value["cgroup"].startswith("/") and bool(value["display"]), message)
    _require(not value["xauthority"] or pathlib.Path(value["xauthority"]).is_absolute(), message)
    return value


def _secret_ok(value: Any) -> bool:
    return type(value) is str and 0 < len(value) <= MAX_INPUT \
        and all(character.isprintable() for character in value)


def _credential(ctx: Context) -> dict[str, Any]:
    message = "secure credential input unavailable"
    value = _read_json(ctx, ctx.credentials, message)
    common = {"schema", "nonce", "expires_monotonic", "next", "password"}
    _require(value.get("schema") == 1 and type(value.get("nonce")) is str
             and re.fullmatch(r"[0-9a-f]{32}", value["nonce"]) is not None, message)
    expiry = value.get("expires_monotonic")
    _require(type(expiry) in (int, float) and math.isfinite(expiry) and expiry > 0, message)
    _require(value.get("next") in ("user", "password") and _secret_ok(value.get("password")),
             message)
    if value["next"] == "user":
        _require(set(value) == common | {"username"} and _secret_ok(value.get("username")),
                 message)
    else:
        _require(set(value) == common, message)
    return value


def _present(path: pathlib.Path) -> bool:
    try:
        os.lstat(path)
        return True
    except FileNotFoundError:
        return False
    except OSError as error:
        raise Refusal("secure credential storage unavailable") from error


def _fsync_directory(ctx: Context) -> None:
    descriptor = os.open(ctx.directory, os.O_RDONLY | os.O_CLOEXEC | getattr(os, "O_DIRECTORY", 0))
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def _encode(value: dict[str, Any]) -> bytes:
    raw = (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()
    _require(len(raw) <= MAX_FILE, "credential input rejected")
    return raw


def _write_all(descriptor: int, raw: bytes) -> None:
    written = 0
    while written < len(raw):
        count = os.write(descriptor, raw[written:])
        _require(count > 0, "secure credential storage unavailable")
        written += count


def _create_secret(ctx: Context, value: dict[str, Any]) -> None:
    raw = _encode(value)
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC \
        | getattr(os, "O_NOFOLLOW", 0)
    created = False
    descriptor = None
    try:
        descriptor = os.open(ctx.credentials, flags, 0o600)
        created = True
        _secure_metadata(os.fstat(descriptor), ctx.uid, "secure credential storage unavailable")
        _write_all(descriptor, raw)
        os.fsync(descriptor)
        os.close(descriptor)
        descriptor = None
        _fsync_directory(ctx)
    except FileExistsError as error:
        raise Refusal("pending input exists; run clear") from error
    except (OSError, Refusal) as error:
        if created:
            try:
                os.unlink(ctx.credentials)
            except OSError:
                pass
        raise Refusal("secure credential storage unavailable") from error
    finally:
        if descriptor is not None:
            try:
                os.close(descriptor)
            except OSError:
                pass


def _replace_secret(ctx: Context, value: dict[str, Any]) -> None:
    raw = _encode(value)
    temporary = ctx.directory / (".credentials-" + secrets.token_hex(16))
    descriptor = None
    try:
        descriptor = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC
                             | getattr(os, "O_NOFOLLOW", 0), 0o600)
        _write_all(descriptor, raw)
        os.fsync(descriptor)
        os.close(descriptor)
        descriptor = None
        os.replace(temporary, ctx.credentials)
        _fsync_directory(ctx)
    except OSError as error:
        raise Refusal("secure credential storage unavailable") from error
    finally:
        if descriptor is not None:
            os.close(descriptor)
        try:
            os.unlink(temporary)
        except FileNotFoundError:
            pass


def _unlink_secret(ctx: Context) -> None:
    try:
        os.unlink(ctx.credentials)
    except FileNotFoundError:
        return
    _fsync_directory(ctx)


def _run(ctx: Context, arguments: list[str], **keywords: Any) -> Any:
    try:
        return ctx.run(arguments, check=False, **keywords)
    except Exception as error:
        raise Refusal("target validation failed") from error


def _x_environment(binding: dict[str, Any]) -> dict[str, str]:
    environment = {"DISPLAY": binding["display"], "LC_ALL": "C.UTF-8"}
    if binding["xauthority"]:
        environment["XAUTHORITY"] = binding["xauthority"]
    return environment


def _target(ctx: Context, binding: dict[str, Any]) -> None:
    message = "target validation failed"
    try:
        process = ctx.proc / str(binding["pid"])
        stat_text = (process / "stat").read_text(encoding="utf-8")
        closing = stat_text.rfind(")")
        fields = stat_text[closing + 2:].split()
        _require(closing > 0 and len(fields) > 19 and fields[19] == binding["start_ticks"], message)
        cgroups = [(parts[0], parts[1], parts[2]) for line in
                   (process / "cgroup").read_text(encoding="utf-8").splitlines()
                   if len(parts := line.split(":", 2)) == 3]
        unified = [path for hierarchy, controllers, path in cgroups
                   if hierarchy == "0" and controllers == ""]
        _require(unified == [binding["cgroup"]], message)
    except (OSError, UnicodeError, Refusal) as error:
        raise Refusal(message) from error
    active = _run(ctx, [SYSTEMCTL, "--user", "is-active", "--quiet", binding["unit"]],
                  stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                  stderr=subprocess.DEVNULL, timeout=5)
    _require(active.returncode == 0, message)
    environment = _x_environment(binding)
    window = _run(ctx, [XDOTOOL, "getactivewindow"], stdin=subprocess.DEVNULL,
                  stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=5,
                  env=environment)
    _require(window.returncode == 0 and window.stdout.decode("ascii", "strict").strip()
             == binding["window"], message)
    owner = _run(ctx, [XDOTOOL, "getwindowpid", binding["window"]], stdin=subprocess.DEVNULL,
                 stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=5,
                 env=environment)
    _require(owner.returncode == 0 and owner.stdout.decode("ascii", "strict").strip()
             == str(binding["pid"]), message)


def _type(ctx: Context, binding: dict[str, Any], value: str) -> None:
    result = _run(ctx, [XDOTOOL, "type", "--window", binding["window"],
                        "--clearmodifiers", "--delay", "1", "--file", "-"],
                  input=value.encode("utf-8"), stdout=subprocess.DEVNULL,
                  stderr=subprocess.DEVNULL, timeout=15, env=_x_environment(binding))
    _require(result.returncode == 0, "credential handoff failed")


def _schedule(ctx: Context, nonce: str) -> None:
    result = _run(ctx, [SYSTEMD_RUN, "--user", "--quiet", "--collect",
                        f"--unit=lvb-ilok-input-expire-{nonce}", "--on-active=15m",
                        PYTHON, str(ctx.helper), "expire", nonce],
                  stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                  stderr=subprocess.DEVNULL, timeout=15)
    _require(result.returncode == 0, "expiration scheduling failed")


def prepare(ctx: Context, is_tty: Callable[[], bool] = sys.stdin.isatty,
            prompt: Callable[[str], str] = getpass.getpass) -> str:
    _require(is_tty(), "interactive terminal required")
    with _locked(ctx):
        if _present(ctx.credentials):
            try:
                _secure_metadata(os.lstat(ctx.credentials), ctx.uid,
                                 "secure credential storage unavailable")
            except OSError as error:
                raise Refusal("secure credential storage unavailable") from error
            raise Refusal("pending input exists; run clear")
        binding = _binding(ctx)
        _target(ctx, binding)
        try:
            with warnings.catch_warnings():
                warnings.simplefilter("error", getpass.GetPassWarning)
                username = prompt("iLok User ID: ")
                password = prompt("iLok Password: ")
        except (EOFError, KeyboardInterrupt, getpass.GetPassWarning) as error:
            raise Refusal("credential input rejected") from error
        _require(_secret_ok(username) and _secret_ok(password), "credential input rejected")
        nonce = secrets.token_hex(16)
        _create_secret(ctx, {"schema": 1, "nonce": nonce,
                             "expires_monotonic": ctx.monotonic() + TTL, "next": "user",
                             "username": username, "password": password})
        try:
            _schedule(ctx, nonce)
        except Refusal:
            _unlink_secret(ctx)
            raise
    return "ready-for-user"


def status(ctx: Context) -> str:
    with _locked(ctx):
        if not _present(ctx.credentials):
            return "none"
        record = _credential(ctx)
        if record["expires_monotonic"] <= ctx.monotonic():
            return "expired"
        return "ready-for-" + record["next"]


def fill_user(ctx: Context) -> str:
    with _locked(ctx):
        try:
            record = _credential(ctx)
            _require(record["next"] == "user"
                     and record["expires_monotonic"] > ctx.monotonic(),
                     "credential handoff failed")
            binding = _binding(ctx)
            _target(ctx, binding)
            _require(record["expires_monotonic"] > ctx.monotonic(),
                     "credential handoff failed")
            _type(ctx, binding, record["username"])
            _replace_secret(ctx, {"schema": 1, "nonce": record["nonce"],
                                  "expires_monotonic": record["expires_monotonic"],
                                  "next": "password", "password": record["password"]})
        except Exception as error:
            _unlink_secret(ctx)
            raise Refusal("credential handoff failed") from error
    return "ready-for-password"


def fill_password(ctx: Context) -> str:
    with _locked(ctx):
        try:
            record = _credential(ctx)
            _require(record["next"] == "password"
                     and record["expires_monotonic"] > ctx.monotonic(),
                     "credential handoff failed")
            binding = _binding(ctx)
            _target(ctx, binding)
            _require(record["expires_monotonic"] > ctx.monotonic(),
                     "credential handoff failed")
            password = record["password"]
            _unlink_secret(ctx)
            _type(ctx, binding, password)
        except Exception as error:
            _unlink_secret(ctx)
            raise Refusal("credential handoff failed") from error
    return "none"


def clear(ctx: Context) -> str:
    with _locked(ctx):
        _unlink_secret(ctx)
    return "none"


def expire(ctx: Context, nonce: str) -> str:
    _require(re.fullmatch(r"[0-9a-f]{32}", nonce) is not None, "expiration request rejected")
    with _locked(ctx, blocking=True):
        if _present(ctx.credentials):
            record = _credential(ctx)
            if record["nonce"] == nonce:
                _unlink_secret(ctx)
    return "none"


def verify(ctx: Context) -> str:
    with _locked(ctx):
        binding = _binding(ctx)
        if _present(ctx.credentials):
            _secure_metadata(os.lstat(ctx.credentials), ctx.uid,
                             "secure credential storage unavailable")
        _target(ctx, binding)
    return "binding-valid"


def _disable_core_dumps() -> None:
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))


def main(arguments: list[str]) -> int:
    try:
        _disable_core_dumps()
        ctx = default_context()
        command = arguments[0] if arguments else "prepare"
        rest = arguments[1:] if arguments else []
        _require((command == "expire" and len(rest) == 1)
                 or (command in {"prepare", "status", "fill-user", "fill-password",
                                 "clear", "verify"} and not rest), "command rejected")
        if command == "prepare":
            result = prepare(ctx)
        elif command == "status":
            result = status(ctx)
        elif command == "fill-user":
            result = fill_user(ctx)
        elif command == "fill-password":
            result = fill_password(ctx)
        elif command == "clear":
            result = clear(ctx)
        elif command == "verify":
            result = verify(ctx)
        else:
            result = expire(ctx, rest[0])
        if command == "prepare":
            print("Credentials ready for Luna. They expire in 15 minutes.")
        else:
            print(result)
        return 0
    except Refusal as error:
        print(str(error), file=sys.stderr)
        return 1
    except (Exception, KeyboardInterrupt):
        print("credential handoff failed", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
