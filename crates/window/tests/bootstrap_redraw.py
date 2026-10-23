#!/usr/bin/env python3
"""A queued bootstrap reply must survive Draw and repaint without user input.

Build bridge-window-preview with --features samples,live before running this
test. The fixture is unavailable in production builds and starts no worker,
client or package task. All evidence before the decisive draw comes from the
owned process's log; even remote status/snapshot calls wait until after it.
"""
from pathlib import Path
import argparse
import re
import sys
import tempfile
import unittest

import remote_ui as ui


class BootstrapApp(ui.App):
    ready_control = None

    def before_remote(self):
        def drawn():
            self.alive()
            text = self.stdout.read_text(errors="replace")
            match = re.search(r"bootstrap page_drawn revision=(\d+) phase=progress_panel", text)
            return text if match else None

        text = ui.wait_until(drawn, "bootstrap progress to draw without a remote event", 30)
        patterns = (
            r"bootstrap page_drawn revision=(\d+) phase=waiting_for_worker",
            r"bootstrap fixture_queued_after_draw revision=(\d+)",
            r"bootstrap fixture_draw_return revision=(\d+) phase=waiting_for_worker",
            r"bootstrap ui_applied revision=(\d+) phase=progress_panel",
            r"bootstrap page_drawn revision=(\d+) phase=progress_panel",
        )
        markers = [re.search(pattern, text) for pattern in patterns]
        assert all(markers), "Missing actual waiting draw, queued reply, deferred pump or progress draw"
        positions = [marker.start() for marker in markers]
        assert positions == sorted(positions), "The reply must be applied only after Draw returns"
        revisions = [int(marker[1]) for marker in markers]
        assert revisions[0] == revisions[1] == revisions[2], "Draw consumed the queued bootstrap reply"
        assert revisions[3] == revisions[4] > revisions[2], "The new model revision was not actually drawn"
        assert text.count("bootstrap fixture_queued_after_draw") == 1, "Fixture must queue only one reply"


class BootstrapRedraw(unittest.TestCase):
    def test_reply_after_waiting_draw_repaints_on_ui_wake(self):
        for width in (1280, 640):
            with self.subTest(width=width), BootstrapApp("bootstrap-redraw",
                    "--bootstrap-draw-fixture", "--presentation-only", f"--size={width}x800") as app:
                self.assertIsNone(app.find("Update Bridge", kind="ActionButton"))
                app.capture(f"bootstrap-progress-{width}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ui.BINARY)
    parser.add_argument("--artifacts", type=Path)
    options, tests = parser.parse_known_args()
    ui.BINARY = options.binary.resolve()
    if not ui.BINARY.is_file():
        parser.error(f"Build the preview with samples,live; no binary at {ui.BINARY}")
    ui.ARTIFACTS = options.artifacts.resolve() if options.artifacts else Path(tempfile.mkdtemp(prefix="bridge-bootstrap-redraw-"))
    if ui.ARTIFACTS.is_relative_to(ui.ROOT):
        parser.error("Keep logs and captures outside the Git checkout")
    ui.ARTIFACTS.mkdir(parents=True, exist_ok=True)
    print("Native captures:", ui.ARTIFACTS, flush=True)
    unittest.main(argv=[sys.argv[0], *tests], verbosity=2)
