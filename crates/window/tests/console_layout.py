#!/usr/bin/env python3
"""Rendered Console composition checks against the approved journey proof.

Runs hidden native sample windows. These check layout and navigation only;
service behavior and physical touch are qualified on the installed Linux route.
"""
import tempfile
import unittest
from pathlib import Path
import remote_ui as ui


class ProductApp(ui.App):
    ready_control = "Home"

    def __init__(self, name, *args):
        super().__init__(name, "--presentation-only", *args)


class ConsoleLayout(unittest.TestCase):
    def test_long_collection_rows_grow_without_clipping_and_preserve_held_actions(self):
        name = ("Layout fixture: a very long orchestral instrument with multiple microphone "
                "positions and an extended performance edition")
        pending = "Owner is checking the selected plug-in and waiting for its window to finish closing."
        heights = {}
        for width in (1280, 640):
            with self.subTest(width=width), ProductApp(
                "console-long-rows", "--state=populated", "--page=home", "--long-row-fixture",
                f"--size={width}x900",
            ) as app:
                rows = [app.required(label, kind="ActionButton") for label in
                        (name, "Unavailable fixture row", pending)]
                heights[width] = rows[0]["r"][3]
                for row in rows:
                    x, y, w, h = row["r"]
                    self.assertGreater(w, width - 100, "The full row must remain the hit target")
                    self.assertGreaterEqual(h, 44)
                    self.assertGreaterEqual(x, 0)
                    self.assertLessEqual(x + w, width)
                for before, after in zip(rows, rows[1:]):
                    self.assertGreaterEqual(after["r"][1], before["r"][1] + before["r"][3],
                                            "Wrapped content must move the following row down")
                long_status = app.required(
                    "Not tried with this DAW and operating system; compatibility has not been established",
                    kind="StatusChip")
                rx, ry, rw, rh = rows[0]["r"]
                sx, sy, sw, sh = long_status["r"]
                self.assertGreaterEqual(sx, rx)
                self.assertGreaterEqual(sy, ry)
                self.assertLessEqual(sx + sw, rx + rw)
                self.assertLessEqual(sy + sh, ry + rh)
                if width == 640:
                    self.assertGreater(sh, 35, "A long status must wrap, not clip on one line")
                    self.assertGreater(rows[1]["r"][3], 100,
                                       "The disabled row must make room for its reason and consequence")
                app.capture(f"long-rows-{width}")
                mark = app.mark()
                if width == 640:
                    app.mouse("click", rows[1])
                    self.assertTrue(any("Waiting for an exact owner readback before this action can be used."
                                        in line for line in app.logs(mark)))
                else:
                    app.mouse("click", rows[2])
                    self.assertTrue(any("is already waiting" in line for line in app.logs(mark)))
                app.no_capture(mark)
        self.assertGreater(heights[640], heights[1280],
                           "Narrow layout must reflow real text into a taller interactive row")

    def test_private_link_is_not_exported_and_plain_search_remains_readable(self):
        canary = "synthetic-private-link-canary-504"
        with ProductApp("console-password", "--state=available", "--page=setup",
                        "--update=no_link", "--size=1280x900") as app:
            field = app.scroll_to(None, prefix="field.beta-link")
            app.mouse("click", field)
            app.request("/t", t=canary, wait=1)
            snapshot = app.snapshot()
            self.assertNotIn(canary, repr(snapshot))
            secret = next(w for w in snapshot if w["ty"] == "TextInput"
                          and w["i"].startswith("field.beta-link"))
            self.assertIsNone(secret.get("val"))
            self.assertIsNone(secret.get("t"))
            app.click("Plug-ins")
            app.click(prefix="field.search")
            app.request("/t", t="ordinary search", wait=1)
            self.assertEqual(app.required(prefix="field.search").get("val"), "ordinary search")

    def test_deck_header_leaves_the_library_visible_and_rows_open_exact_details(self):
        with ProductApp("console-library", "--state=populated", "--page=home", "--size=1280x746") as app:
            viewport = app.required(prefix="page_scroll")["r"]
            self.assertLess(viewport[1], 130, "Product chrome consumes too much Deck height")
            kontakt = app.required("Kontakt 8", kind="ActionButton")
            serum = app.required("Serum 2 FX", kind="ActionButton")
            self.assertGreater(kontakt["r"][2], 1100, "The whole collection row must be clickable")
            self.assertLess(kontakt["r"][1], 230)
            self.assertLess(serum["r"][1] + serum["r"][3], 600)
            for chip in [w for w in app.snapshot() if w["ty"] == "StatusChip"]:
                self.assertLessEqual(chip["r"][0] + chip["r"][2], 1280)
            app.capture("home")
            app.mouse("click", kontakt)
            self.assertIsNone(app.find("Serum 2 FX", kind="ActionButton"))
            self.assertIsNotNone(app.find("Show all plug-ins", kind="ActionButton"))
            app.capture("exact-product")
            app.click("Show all plug-ins")
            self.assertIsNotNone(app.find("Kontakt 8", kind="ActionButton"))
            app.capture("collection")

    def test_narrow_window_preserves_navigation_and_row_targets(self):
        with ProductApp("console-narrow", "--state=populated", "--page=home", "--size=640x720") as app:
            for label in ("Home", "Plug-ins", "Activity", "Support"):
                button = app.required(label, prefix="local.")
                x, y, w, h = button["r"]
                self.assertGreaterEqual(h, 43)
                self.assertGreaterEqual(x, 0)
                self.assertLessEqual(x + w, 640)
            row = app.required("Kontakt 8", kind="ActionButton")
            self.assertGreater(row["r"][2], 540)
            app.mouse("click", row)
            app.capture("narrow-product")


if __name__ == "__main__":
    ui.ARTIFACTS = Path(tempfile.mkdtemp(prefix="bridge-console-layout-"))
    print("Native captures:", ui.ARTIFACTS, flush=True)
    unittest.main()
