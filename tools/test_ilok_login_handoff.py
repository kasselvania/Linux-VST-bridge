import importlib.util
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import unittest
import warnings
from unittest import mock


PATH = pathlib.Path(__file__).with_name("ilok-login-handoff.py")
SPEC = importlib.util.spec_from_file_location("ilok_login_handoff", PATH)
HANDOFF = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = HANDOFF
SPEC.loader.exec_module(HANDOFF)


class FakeRun:
    def __init__(self, window="83886085", pid=1089040):
        self.window = window
        self.pid = pid
        self.calls = []
        self.type_code = 0
        self.before_type = None
        self.before_owner = None

    def __call__(self, arguments, **keywords):
        self.calls.append((list(arguments), keywords.copy()))
        output = b""
        code = 0
        if arguments[:2] == [HANDOFF.XDOTOOL, "getactivewindow"]:
            output = (self.window + "\n").encode()
        elif arguments[:2] == [HANDOFF.XDOTOOL, "getwindowpid"]:
            if self.before_owner:
                self.before_owner()
            output = (str(self.pid) + "\n").encode()
        elif arguments[:2] == [HANDOFF.XDOTOOL, "type"]:
            if self.before_type:
                self.before_type()
            code = self.type_code
        return subprocess.CompletedProcess(arguments, code, output)


class HandoffTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        root = pathlib.Path(self.temporary.name)
        self.runtime = root / "runtime"
        self.runtime.mkdir(mode=0o700)
        self.proc = root / "proc"
        self.pid = 1089040
        process = self.proc / str(self.pid)
        process.mkdir(parents=True)
        self.start = "998877"
        fields = ["S"] + ["0"] * 18 + [self.start]
        (process / "stat").write_text(f"{self.pid} (iLok License Manager) "
                                      + " ".join(fields) + "\n")
        self.cgroup = "/user.slice/user-1000.slice/user@1000.service/app.slice/ilok.scope"
        (process / "cgroup").write_text("0::" + self.cgroup + "\n")
        self.binding = {"schema": 1, "window": "83886085", "pid": self.pid,
                        "start_ticks": self.start, "unit": "ilok.scope",
                        "cgroup": self.cgroup, "display": ":0",
                        "xauthority": "/run/user/1000/xauth"}
        self._write(self.runtime / "binding.json", self.binding)
        self.runner = FakeRun(pid=self.pid)
        self.now = 1000.0
        self.ctx = HANDOFF.Context(self.runtime, self.proc, PATH, os.getuid(),
                                   self.runner, lambda: self.now)

    def tearDown(self):
        self.temporary.cleanup()

    @staticmethod
    def _write(path, value):
        path.write_text(json.dumps(value) + "\n")
        path.chmod(0o600)

    def _prepare(self, username="sample-user", password="sample-password"):
        values = iter([username, password])
        return HANDOFF.prepare(self.ctx, lambda: True, lambda _message: next(values))

    def test_prepare_refuses_non_tty_without_prompting(self):
        prompted = []
        with self.assertRaisesRegex(HANDOFF.Refusal, "interactive terminal required"):
            HANDOFF.prepare(self.ctx, lambda: False, lambda text: prompted.append(text))
        self.assertEqual(prompted, [])
        self.assertFalse(self.ctx.credentials.exists())

    def test_prepare_refuses_getpass_echo_fallback(self):
        def fallback(_message):
            warnings.warn("fallback", HANDOFF.getpass.GetPassWarning)
            return "must-not-store"
        with self.assertRaisesRegex(HANDOFF.Refusal, "credential input rejected"):
            HANDOFF.prepare(self.ctx, lambda: True, fallback)
        self.assertFalse(self.ctx.credentials.exists())

    def test_insecure_permissions_and_symlinks_are_refused(self):
        self.runtime.chmod(0o755)
        with self.assertRaises(HANDOFF.Refusal):
            HANDOFF.verify(self.ctx)
        self.runtime.chmod(0o700)
        self.ctx.binding.unlink()
        target = self.runtime / "outside.json"
        self._write(target, self.binding)
        self.ctx.binding.symlink_to(target)
        with self.assertRaises(HANDOFF.Refusal):
            HANDOFF.verify(self.ctx)
        self.ctx.binding.unlink()
        self._write(self.ctx.binding, self.binding)
        target.chmod(0o644)
        self.ctx.credentials.symlink_to(target)
        with self.assertRaises(HANDOFF.Refusal):
            HANDOFF.verify(self.ctx)

    def test_sequence_uses_stdin_and_password_is_deleted_before_failed_send(self):
        self.assertEqual(self._prepare(), "ready-for-user")
        self.assertTrue(any(call[0][0] == HANDOFF.SYSTEMD_RUN for call in self.runner.calls))
        self.assertEqual(HANDOFF.status(self.ctx), "ready-for-user")
        self.assertEqual(HANDOFF.fill_user(self.ctx), "ready-for-password")
        self.assertEqual(HANDOFF.status(self.ctx), "ready-for-password")
        typed = [call for call in self.runner.calls
                 if call[0][:2] == [HANDOFF.XDOTOOL, "type"]]
        self.assertEqual(typed[0][1]["input"], b"sample-user")
        joined = " ".join(typed[0][0]) + " " + " ".join(typed[0][1]["env"].values())
        self.assertNotIn("sample-user", joined)
        self.assertNotIn("sample-password", joined)
        self.assertEqual(typed[0][1]["stdout"], subprocess.DEVNULL)
        self.assertEqual(typed[0][1]["stderr"], subprocess.DEVNULL)
        self.assertEqual(typed[0][1]["timeout"], 15)
        self.assertEqual(typed[0][1]["env"]["LC_ALL"], "C.UTF-8")
        self.runner.type_code = 1
        self.runner.before_type = lambda: self.assertFalse(self.ctx.credentials.exists())
        with self.assertRaises(HANDOFF.Refusal):
            HANDOFF.fill_password(self.ctx)
        self.assertFalse(self.ctx.credentials.exists())
        count = len([call for call in self.runner.calls
                     if call[0][:2] == [HANDOFF.XDOTOOL, "type"]])
        with self.assertRaises(HANDOFF.Refusal):
            HANDOFF.fill_password(self.ctx)
        self.assertEqual(count, len([call for call in self.runner.calls
                                    if call[0][:2] == [HANDOFF.XDOTOOL, "type"]]))

    def test_expiration_nonce_cannot_delete_replacement(self):
        self._prepare()
        old = json.loads(self.ctx.credentials.read_text())["nonce"]
        replacement = {"schema": 1, "nonce": "ab" * 16,
                       "expires_monotonic": self.now + 900, "next": "password",
                       "password": "replacement"}
        HANDOFF._replace_secret(self.ctx, replacement)
        with mock.patch.object(HANDOFF.fcntl, "flock", wraps=HANDOFF.fcntl.flock) as flock:
            self.assertEqual(HANDOFF.expire(self.ctx, old), "none")
        self.assertEqual(flock.call_args_list[0].args[1], HANDOFF.fcntl.LOCK_EX)
        self.assertEqual(json.loads(self.ctx.credentials.read_text())["nonce"], "ab" * 16)
        HANDOFF.expire(self.ctx, "ab" * 16)
        self.assertFalse(self.ctx.credentials.exists())

    def test_target_mismatch_refuses_without_typing_and_erases_input(self):
        self._prepare()
        before = len([call for call in self.runner.calls
                      if call[0][:2] == [HANDOFF.XDOTOOL, "type"]])
        self.runner.window = "83886086"
        with self.assertRaises(HANDOFF.Refusal):
            HANDOFF.fill_user(self.ctx)
        self.assertFalse(self.ctx.credentials.exists())
        after = len([call for call in self.runner.calls
                     if call[0][:2] == [HANDOFF.XDOTOOL, "type"]])
        self.assertEqual(before, after)

    def test_expiry_is_rechecked_after_target_validation(self):
        self._prepare()
        self.runner.before_owner = lambda: setattr(self, "now", self.now + HANDOFF.TTL)
        with self.assertRaises(HANDOFF.Refusal):
            HANDOFF.fill_user(self.ctx)
        self.assertFalse(self.ctx.credentials.exists())
        self.assertFalse(any(call[0][:2] == [HANDOFF.XDOTOOL, "type"]
                             for call in self.runner.calls))


if __name__ == "__main__":
    unittest.main()
