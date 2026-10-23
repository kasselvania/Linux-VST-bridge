#!/usr/bin/env python3
"""Native rendered preview regressions; run after building the release binary.

Uses only the app's localhost remote API. Every test owns a hidden process,
uses its returned widget geometry, and quits and reaps that process. Logs and
the final app-only capture remain in a temporary directory outside the repo.
"""

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import time
import unittest
from urllib.parse import urlencode
from urllib.request import urlopen


ROOT = Path(__file__).resolve().parents[3]
BINARY = ROOT / "crates/window/target/release/bridge-window-preview"
ARTIFACTS = None
FOCUS = re.compile(r"\[window\] focus (.+?)(?: “(.*)”(?: \(disclosure\))?| \(text field\))$")


def wait_until(check, description, timeout=10):
    """Bounded readiness polling, never a sleep used as an assertion oracle."""
    deadline = time.monotonic() + timeout
    while True:
        value = check()
        if value:
            return value
        if time.monotonic() >= deadline:
            raise AssertionError(f"Timed out waiting for {description}")
        time.sleep(0.02)


class App:
    ready_control = "Next sample"

    def __init__(self, name, *args):
        self.directory = Path(tempfile.mkdtemp(prefix=name + "-", dir=ARTIFACTS))
        self.stdout = self.directory / "app.log"
        self.port = None
        self.process = None
        self.args = args

    def __enter__(self):
        try:
            with self.stdout.open("wb") as log:
                self.process = subprocess.Popen(
                    [str(BINARY), "--remote", *self.args], cwd=ROOT,
                    env={**os.environ, "MAKEPAD_HIDE_WINDOWS": "1"},
                    stdout=log, stderr=subprocess.STDOUT,
                )

            def listening():
                self.alive()
                match = re.search(r"listening on 127\.0\.0\.1:(\d+)", self.stdout.read_text(errors="replace"))
                return int(match[1]) if match else None

            self.port = wait_until(listening, "the owned app's remote port", 30)
            status = self.request("/s")
            assert status["pid"] == self.process.pid, "Remote server is not the process this test launched"
            size = next((arg.removeprefix("--size=") for arg in self.args if arg.startswith("--size=")), None)
            if size is not None:
                expected = [int(value) for value in size.split("x")]
                wait_until(lambda: any(window["sz"] == expected for window in self.request("/s")["w"]),
                           f"the requested {size} window size", 30)
            wait_until(lambda: self.find(self.ready_control, kind="ActionButton") and self.find(prefix="page_scroll"),
                       "the first rendered preview controls", 30)
            return self
        except BaseException:
            self.close()
            raise

    def __exit__(self, kind, value, traceback):
        if kind is not None and self.port is not None:
            try:
                widgets = self.snapshot()
                safe = [{"type": w["ty"], "label": w.get("t", ""), "rect": w["r"]} for w in widgets]
                (self.directory / "failure-widgets.json").write_text(json.dumps(safe, indent=2))
            except Exception:
                pass
        try:
            self.close()
        except Exception as cleanup:
            if value is None:
                raise
            value.add_note(f"Cleanup: {cleanup}")
        if value is not None:
            value.add_note(f"Native app log and captures: {self.directory}")

    def close(self):
        if self.process is None:
            return
        failure = None
        if self.process.poll() is None:
            try:
                if self.port is None:
                    raise RuntimeError("The app never advertised a remote port")
                capture = self.request("/gq", timeout=15)
                (self.directory / "final-capture.json").write_text(json.dumps(capture, indent=2))
            except Exception as error:
                failure = error
            try:
                self.process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                self.process.terminate()
                try:
                    self.process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    self.process.kill()
                    self.process.wait()
                failure = failure or RuntimeError("Owned app did not exit after /gq")
        else:
            self.process.wait()
        if failure is not None:
            raise RuntimeError(f"Could not quit the owned app normally: {failure}")
        if self.process.returncode != 0:
            raise AssertionError(f"Owned app exited with status {self.process.returncode}; log: {self.stdout}")

    def alive(self):
        if self.process.poll() is not None:
            raise AssertionError(f"Owned app exited early ({self.process.returncode}); log: {self.stdout}")

    def request(self, route, timeout=8, **parameters):
        self.alive()
        query = "?" + urlencode(parameters) if parameters else ""
        with urlopen(f"http://127.0.0.1:{self.port}{route}{query}", timeout=timeout) as response:
            result = json.load(response)
        if "err" in result:
            raise AssertionError(f"Remote {route}: {result['err']}")
        return result

    def snapshot(self, **parameters):
        return self.request("/snap", **parameters)["s"]

    def mark(self):
        return self.request("/log", n=1)["n"]

    def logs(self, mark):
        return self.request("/log", since=mark, n=500)["l"]

    def find(self, label=None, *, prefix=None, kind=None):
        matches = [w for w in self.snapshot()
                   if (label is None or w.get("t") == label)
                   and (prefix is None or w["i"].startswith(prefix))
                   and (kind is None or w["ty"] == kind)]
        if not matches:
            return None
        return matches[0]

    def required(self, label=None, **parameters):
        result = self.find(label, **parameters)
        if result is None:
            raise AssertionError(f"No visible widget: {label or parameters}")
        return result

    @staticmethod
    def center(widget):
        x, y, width, height = widget["r"]
        assert width > 0 and height > 0, "Widget has no visible click target"
        return x + width / 2, y + height / 2

    def mouse(self, kind, widget, wait=True):
        x, y = self.center(widget)
        return self.request("/m", k=kind, x=x, y=y, **({"wait": 1} if wait else {}))

    def click(self, label=None, **parameters):
        return self.mouse("click", self.required(label, **parameters))

    def key(self, code, *, wait=False, **modifiers):
        return self.request("/k", k="press", c=code, **modifiers, **({"wait": 1} if wait else {}))

    def tab(self, reverse=False):
        mark = self.mark()
        self.key("Tab", shift=int(reverse))

        def focused():
            records = [FOCUS.search(line) for line in self.logs(mark)]
            return [match.groups() for match in records if match]

        return wait_until(focused, "Tab to move focus to a rendered control", 3)[-1]

    def scroll_to(self, label, *, viewport="page_scroll", prefix=None, direction=1):
        for _ in range(60):
            widget = self.find(label, prefix=prefix)
            # These runs use the default 44-point controls. A clipped strip
            # is not evidence that a musician can see and use the button.
            if widget is not None and widget["r"][3] >= 43:
                return widget
            view = self.required(prefix=viewport)
            x, y = self.center(view)
            self.request("/m", k="scroll", x=x, y=y, dy=direction * 180, dx=0, wait=1)
        raise AssertionError(f"Could not scroll {label!r} into {viewport}")

    def no_capture(self, mark):
        assert not any("Captured, not sent" in line for line in self.logs(mark)), "Unexpected manager action capture"

    def capture(self, name):
        capture = self.request("/g", scale=0.5)
        (self.directory / (name + ".json")).write_text(json.dumps(capture, indent=2))
        return capture["png"]


class NativeUiTests(unittest.TestCase):
    def test_confirmation_traps_tab_and_escape_restores_navigation(self):
        with App("confirmation", "--page=setup", "--state=setup_removal", "--reveal", "--size=1280x800") as app:
            app.mouse("click", app.scroll_to("Uninstall, remove or delete"))
            opener = app.scroll_to("Delete setup", prefix="offer.")
            background = {w["i"] for w in app.snapshot(all=1)}
            mark = app.mark()
            app.mouse("click", opener)
            modal = {w["i"] for w in app.snapshot(all=1) if w["i"] not in background}
            self.assertTrue(modal, "Confirmation must add rendered controls")
            seen = set()
            for reverse in (False, True):
                for _ in range(12):
                    identity, label = app.tab(reverse)
                    self.assertTrue(identity in modal, f"Modal focus escaped to {label or 'a background field'}")
                    seen.add(label)
            self.assertIn("Cancel", seen)
            self.assertIn("Delete setup", seen)
            app.no_capture(mark)
            app.key("Escape", wait=True)
            identity, label = app.tab()
            self.assertTrue(identity not in modal, f"Focus stayed on dismissed modal control {label}")
            app.no_capture(mark)

    def test_result_form_scroll_and_keyboard_at_both_sizes(self):
        for size in ("1280x800", "640x720"):
            with self.subTest(size=size), App("result-" + size, "--page=plugins", "--state=guided", "--size=" + size) as app:
                app.mouse("click", app.scroll_to("Help and testing"))
                app.mouse("click", app.scroll_to("Record a test result or problem"))
                app.click("Report a problem")
                app.mouse("click", app.scroll_to("Other problem", viewport="form_scroll"))
                submit = app.scroll_to("Record this test result", viewport="form_scroll")
                cancel = app.required("Cancel")
                self.assertGreater(submit["r"][2], 20)
                self.assertGreater(cancel["r"][2], 20)
                self.assertGreaterEqual(cancel["r"][3], 43)
                app.capture("result-form-footer")
                mark = app.mark()
                app.mouse("click", submit)
                self.assertTrue(any("Not passed on:" in line and "via Pointer" in line for line in app.logs(mark)),
                                "Incomplete result must refuse pointer activation")
                app.no_capture(mark)
                mark = app.mark()
                app.key("Enter", wait=True)
                self.assertTrue(any("Not passed on:" in line and "via Keyboard" in line for line in app.logs(mark)),
                                "Incomplete result must refuse keyboard activation")
                app.no_capture(mark)
                seen = set()
                for _ in range(80):
                    _, label = app.tab()
                    seen.add(label)
                    if {"Record this test result", "Cancel"} <= seen:
                        break
                self.assertTrue({"Record this test result", "Cancel"} <= seen,
                                "Both form footer controls must be reachable by keyboard")
                app.capture("keyboard-footer")
                for label in ("Record this test result", "Cancel"):
                    self.assertGreaterEqual(app.required(label)["r"][3], 43,
                                            f"Keyboard-reached {label} must be fully visible")
                app.mouse("click", app.scroll_to("Cancel", viewport="form_scroll"))
                self.assertIsNone(app.find("Report a problem (chosen)"), "Cancel must close the result form")

    def test_held_rename_does_not_retarget_after_sample_change(self):
        with App("held-rename", "--page=setup", "--state=setup_many", "--reveal", "--size=1280x800") as app:
            app.mouse("click", app.scroll_to("Setup details and options"))
            rename = app.scroll_to("Rename", prefix="offer.")
            mark = app.mark()
            app.mouse("down", rename)
            app.key("]", wait=True, **({"cmd": 1} if sys.platform == "darwin" else {"ctrl": 1}))
            self.assertTrue(any("Sample install_running for setup" in line for line in app.logs(mark)),
                            "Sample shortcut must replace the target while the pointer is held")
            app.mouse("up", rename, wait=False)
            app.capture("after-held-release")
            self.assertFalse(any("rename form opened" in line for line in app.logs(mark)),
                             "Held pointer was reassigned to a different Rename target")
            self.assertIsNone(app.find(prefix="field.rename"))
            app.no_capture(mark)

    def test_all_navigation_tabs_change_the_rendered_page(self):
        for size in ("1280x800", "640x720"):
            with self.subTest(size=size), App("navigation-" + size, "--page=setup", "--state=populated", "--size=" + size) as app:
                def page_controls():
                    viewport = app.required(prefix="page_scroll")["r"]
                    return {(w["i"], w.get("t", "")) for w in app.snapshot()
                            if w["ty"] in ("ActionButton", "Disclosure", "TextInput") and w["r"][1] >= viewport[1]}

                for label in ("Plug-ins", "Activity", "Support", "Home"):
                    before = page_controls()
                    mark = app.mark()
                    tab = app.required(label, prefix="local.")
                    self.assertGreater(tab["r"][2], 45, f"{label} tab is clipped")
                    self.assertGreaterEqual(tab["r"][3], 43, f"{label} tab is clipped")
                    app.mouse("click", tab)
                    self.assertTrue(any(f"Tab {label} via Pointer" in line for line in app.logs(mark)),
                                    f"{label} tab did not activate")
                    self.assertTrue(app.required(prefix="page_scroll")["r"][3] > 0)
                    self.assertTrue(page_controls() != before, f"{label} did not replace the rendered page controls")
                    app.no_capture(mark)
                    if label == "Home":
                        app.capture("home-" + size)

    def test_clear_search_removal_does_not_strand_tab_focus(self):
        with App("clear-search", "--page=plugins", "--state=populated", "--size=1280x800") as app:
            app.click("Show all plug-ins")
            app.click(prefix="field.search")
            app.request("/t", t="no-such-plugin-regression", wait=1)
            clear = app.required("Clear search")
            app.mouse("click", clear)
            self.assertIsNone(app.find("Clear search"))
            identity, label = app.tab()
            self.assertTrue(identity != clear["i"], f"Focus stayed on the removed {label} control")
            self.assertEqual(app.required(prefix="field.search").get("val"), "")

    def test_search_field_density_geometry_and_edge_hit_area(self):
        for density, height in (("comfortable", 44), ("compact", 32)):
            for size in ("1280x800", "640x720"):
                with self.subTest(density=density, size=size), App(
                    "search-target-" + density + "-" + size, "--page=plugins",
                    "--state=populated", "--density=" + density, "--size=" + size,
                ) as app:
                    app.click("Plug-ins", prefix="local.")
                    field = app.required(prefix="field.search", kind="TextInput")
                    self.assertAlmostEqual(field["r"][3], height, delta=0.5,
                                           msg="Rendered search height must follow the selected density")
                    self.assertGreaterEqual(field["r"][2], 44)
                    # Focus another control first, then edit from both ends of
                    # the field's rendered hit box. A taller decorative wrapper
                    # around a shorter TextInput would not pass this check.
                    for edge in ("top", "bottom"):
                        app.click("Plug-ins", prefix="local.")
                        x, y, width, actual_height = app.required(prefix="field.search")["r"]
                        hit_y = y + 1 if edge == "top" else y + actual_height - 1
                        app.request("/m", k="click", x=x + width / 2, y=hit_y, wait=1)
                        app.key("KeyA", **({"cmd": 1} if sys.platform == "darwin" else {"ctrl": 1}))
                        value = "edge-" + edge
                        app.request("/t", t=value, wait=1)
                        self.assertEqual(app.required(prefix="field.search").get("val"), value,
                                         f"The {edge} of the field must focus and accept text")
                    app.capture("search-field-" + density + "-" + size)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=BINARY)
    parser.add_argument("--artifacts", type=Path)
    options, tests = parser.parse_known_args()
    BINARY = options.binary.resolve()
    if not BINARY.is_file():
        parser.error(f"Build the release preview first; no binary at {BINARY}")
    ARTIFACTS = options.artifacts.resolve() if options.artifacts else Path(tempfile.mkdtemp(prefix="bridge-window-preview-native-"))
    if ARTIFACTS.is_relative_to(ROOT):
        parser.error("Keep logs and captures outside the Git checkout")
    ARTIFACTS.mkdir(parents=True, exist_ok=True)
    print(f"Native UI artifacts: {ARTIFACTS}", flush=True)
    unittest.main(argv=[sys.argv[0], *tests], verbosity=2)
