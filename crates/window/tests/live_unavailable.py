#!/usr/bin/env python3
"""Native live startup outside an installed generation, with no sample fallback."""
import argparse
from pathlib import Path
import sys
import tempfile
import unittest
import remote_ui as remote


class LiveApp(remote.App):
    ready_control = "Home"


class LiveUnavailableTests(unittest.TestCase):
    def test_refusal_is_rendered_and_navigation_remains_responsive(self):
        with LiveApp("live-unavailable", "--size=1280x800") as app:
            # Blocks draw prose directly; the remote widget tree exposes the
            # resulting status chip and request controls. The app-only capture
            # also retains the precise refusal text for visual verification.
            remote.wait_until(lambda: app.find("Bridge service status unavailable", kind="StatusChip")
                              and app.find("Try again", kind="ActionButton"),
                              "the client's unavailable status and request feedback")
            self.assertIsNone(app.find("Next sample"))
            self.assertIsNone(app.find("Previous sample"))
            for label in ("Plug-ins", "Activity", "Support", "Home"):
                tab = app.required(label, kind="ActionButton")
                app.mouse("click", tab)
                self.assertTrue(app.required(prefix="page_scroll")["r"][3] > 0)
                self.assertIsNotNone(app.find("Try again", kind="ActionButton"))
            self.assertIsNone(app.find("Dismiss", kind="ActionButton"))
            app.capture("failed-initial-read")
            app.mouse("click", app.required("Try again", kind="ActionButton"))
            remote.wait_until(lambda: app.find("Try again", kind="ActionButton")
                              and app.find("Bridge service status unavailable", kind="StatusChip"),
                              "the retry control to remain visible after the read request")
            # A keyboard focus traversal remains possible after worker completion.
            identity, _ = app.tab()
            self.assertTrue(identity)
            app.capture("unavailable")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path,
                        default=remote.ROOT / "crates/window/target/release/linux-audio-compatibility-manager")
    options, tests = parser.parse_known_args()
    remote.BINARY = options.binary.resolve()
    if not remote.BINARY.is_file():
        parser.error("Build the live release binary first")
    remote.ARTIFACTS = Path(tempfile.mkdtemp(prefix="bridge-live-native-"))
    print(f"Native UI artifacts: {remote.ARTIFACTS}", flush=True)
    unittest.main(argv=[sys.argv[0], *tests], verbosity=2)
