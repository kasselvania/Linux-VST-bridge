#!/usr/bin/env python3
"""Hidden rendered Support journey; fixture file operations never touch disk."""
from pathlib import Path
import tempfile
import unittest
import remote_ui as ui
from console_layout import ProductApp


class SupportReport(unittest.TestCase):
    def test_inspect_paginate_save_and_finish_at_both_widths(self):
        for width in (1280, 640):
            with self.subTest(width=width), ProductApp("support-report", "--support-report-fixture",
                    f"--size={width}x800") as app:
                read = app.scroll_to("Read the contents")
                save = app.required("Save report…", kind="ActionButton")
                for button in (read, save):
                    x, _, w, h = button["r"]
                    self.assertGreaterEqual(h, 44)
                    self.assertGreaterEqual(x, 0)
                    self.assertLessEqual(x + w, width)
                self.assertEqual(read["r"][1], save["r"][1])
                app.capture(f"ready-{width}")
                app.mouse("click", read)
                app.capture(f"contents-{width}")
                previous = app.required("Previous page", kind="ActionButton")
                mark = app.mark()
                app.mouse("click", previous)
                self.assertTrue(any("first page" in line for line in app.logs(mark)))
                app.click("Next page")
                app.capture(f"next-page-{width}")
                app.click("Previous page")
                app.click("Back to report")
                app.mouse("click", app.scroll_to("Save report…"))
                app.required("Done", kind="ActionButton")
                self.assertIsNone(app.find("Save report…", kind="ActionButton"))
                app.capture(f"saved-{width}")
                app.click("Read the contents")
                app.required("Back to report", kind="ActionButton")
                app.click("Back to report")
                app.click("Done")
                self.assertIsNone(app.find("Done", kind="ActionButton"))


if __name__ == "__main__":
    ui.ARTIFACTS = Path(tempfile.mkdtemp(prefix="bridge-support-native-"))
    print("Native captures:", ui.ARTIFACTS, flush=True)
    unittest.main(verbosity=2)
