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
    def test_display_and_repair_are_focused_subpages_at_desktop_and_deck_widths(self):
        reason = "Close this exact plug-in window before preparing its accessibility trial."
        for width in (1280, 640):
            with self.subTest(width=width), ProductApp("console-display", "--display-fixture",
                    "--state=plugin_ready", "--page=plugins", f"--size={width}x800") as app:
                def reveal(label):
                    x, y = app.center(app.required(prefix="page_scroll"))
                    app.request("/m", k="scroll", x=x, y=y, dy=-10000, dx=0, wait=1)
                    return app.scroll_to(label)

                def usable(label):
                    control = reveal(label)
                    x, _, w, h = control["r"]
                    self.assertGreaterEqual(h, 44)
                    self.assertGreaterEqual(x, 0)
                    self.assertLessEqual(x + w, width)
                    return control

                self.assertIsNone(app.find("Prepare Wine D3D11 · default accessibility"))
                initial_fix = usable("Fix it step by step")["i"]
                app.capture(f"product-detail-{width}")
                mark = app.mark()
                app.mouse("click", usable("Display mode"))
                app.required("Back to plug-in", kind="ActionButton")
                app.capture(f"display-page-{width}")
                record = usable("Record what worked…")
                app.mouse("click", record)
                app.scroll_to("Record this test result", viewport="form_scroll")
                app.key("Escape", wait=True)
                usable("Prepare Wine D3D11 · default accessibility")
                self.assertIsNone(app.find("Use this prepared setup"))
                app.mouse("click", reveal("Choose a prepared setting…"))
                usable("Use this prepared setup")
                self.assertIsNone(app.find("This step is unavailable right now."))
                app.capture(f"prepared-setting-{width}")
                app.mouse("click", usable("Back to plug-in"))
                self.assertIsNone(app.find("Use this prepared setup"))
                app.mouse("click", usable("Fix it step by step"))
                app.capture(f"repair-page-{width}")
                usable("Open on its own")
                self.assertIsNone(app.find("Prepare Wine D3D11 · default accessibility"))
                app.mouse("click", usable("Choose…"))
                usable("Prepare Wine D3D11 · default accessibility")
                app.mouse("click", usable("Back to plug-in"))
                app.mouse("click", usable("Fix it step by step"))
                blocked = usable("Prepare runtime defaults · default accessibility")
                self.assertNotIn("shown above", repr(app.snapshot()))
                app.capture(f"repair-refusal-{width}")
                app.mouse("click", blocked)
                self.assertTrue(any(reason in line for line in app.logs(mark)))
                app.mouse("click", usable("Back to plug-in"))
                self.assertEqual(usable("Fix it step by step")["i"], initial_fix,
                                 "Back must retain the exact product identity")
                app.no_capture(mark)

    def test_restart_confirmation_names_possible_lost_changes_and_cancels(self):
        for width in (1280, 640):
            with self.subTest(width=width), ProductApp("console-restart", "--recovery-fixture",
                    "--state=plugin_ready", "--page=plugins", f"--size={width}x800") as app:
                button = app.scroll_to("Restart this instance")
                x, _, w, h = button["r"]
                self.assertGreaterEqual(h, 44)
                self.assertGreaterEqual(x, 0)
                self.assertLessEqual(x + w, width)
                app.capture(f"restart-detail-{width}")
                mark = app.mark()
                app.mouse("click", button)
                # Blocks draws prose directly; the pure test checks exact warning text.
                # The retained capture is inspected for wrapping and visibility.
                confirm = app.scroll_to("Restart from confirmed state", viewport="form_scroll")
                x, _, w, h = confirm["r"]
                self.assertGreaterEqual(h, 44)
                self.assertGreaterEqual(x, 0)
                self.assertLessEqual(x + w, width)
                app.capture(f"restart-confirmation-{width}")
                app.mouse("click", app.scroll_to("Not now", viewport="form_scroll"))
                self.assertIsNone(app.find("Restart from confirmed state"))
                app.mouse("click", app.required("Activity", kind="ActionButton"))
                app.mouse("click", app.scroll_to("Restart this instance"))
                # Blocks draws prose directly; the pure test checks exact warning text.
                # The retained capture is inspected for wrapping and visibility.
                app.key("Escape", wait=True)
                self.assertIsNone(app.find("Restart from confirmed state"))
                app.no_capture(mark)

    def test_retained_health_fits_during_refresh(self):
        for width in (1280, 640):
            for request in (None, "renewing"):
                args = ["--state=populated", "--page=diagnostics", f"--size={width}x800"]
                if request:
                    args.append(f"--request={request}")
                with self.subTest(width=width, request=request), ProductApp("console-support-refresh", *args) as app:
                    chip = app.required(prefix="chip.health", kind="StatusChip")
                    self.assertNotEqual(chip.get("t"), "Bridge service status unavailable")
                    x, y, w, h = chip["r"]
                    self.assertGreater(w, 100)
                    self.assertGreater(h, 20)
                    self.assertGreaterEqual(x, 0)
                    self.assertLessEqual(x + w, width)
                    if width == 1280:
                        self.assertAlmostEqual(x + w, width - 24, delta=1,
                                               msg="Refresh must preserve the header right anchor")
                    self.assertLessEqual(y + h, app.required(prefix="page_scroll")["r"][1])
                    app.capture(f"support-{width}-{request or 'current'}")

    def test_next_setup_groups_its_exact_actions_at_desktop_and_deck_widths(self):
        for width in (1280, 640):
            with self.subTest(width=width), ProductApp("console-next-setup", "--state=install_app",
                    "--page=home", f"--size={width}x800") as app:
                primary = app.required("Install plug-ins with Example Hub", kind="ActionButton")
                opened = app.required("Open this setup", kind="ActionButton")
                px, py, pw, ph = primary["r"]
                ox, oy, ow, oh = opened["r"]
                self.assertEqual(py, oy, "Actions that fit belong on the same row")
                self.assertGreaterEqual(ox, px + pw)
                self.assertLessEqual(ox + ow, width)
                self.assertGreaterEqual(min(ph, oh), 44)
                self.assertNotEqual(primary["i"], opened["i"])
                app.capture(f"setup-actions-{width}")

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
