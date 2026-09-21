import os
import pathlib
import stat
import subprocess
import tempfile
import unittest


ROOT = pathlib.Path(__file__).parents[1]
SCRIPT = ROOT / "mixer-state.sh"


class MixerStateTest(unittest.TestCase):
    def test_apply_uses_exact_raw_control_names_and_values(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            log = root / "amixer.log"
            amixer = root / "amixer"
            amixer.write_text('#!/bin/sh\nprintf "%s\\n" "$*" >>"$FAKE_AMIXER_LOG"\n')
            amixer.chmod(amixer.stat().st_mode | stat.S_IXUSR)
            result = subprocess.run(
                [str(SCRIPT), "apply"],
                env={
                    **os.environ,
                    "PATH": f"{directory}:/usr/bin:/bin",
                    "FAKE_AMIXER_LOG": str(log),
                },
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(
                log.read_text().splitlines(),
                [
                    "-c SHIELDXL cset name=Master Playback Volume 255,255",
                    "-c SHIELDXL cset name=Master Playback Switch on,on",
                    "-c SHIELDXL cset name=Master Capture Switch on,on",
                    "-c SHIELDXL cset name=Digital Sidetone Switch off",
                    "-c SHIELDXL cset name=Soft Ramp Switch on",
                    "-c SHIELDXL cset name=Zero Cross Switch on",
                    "-c SHIELDXL cset name=De-emphasis filter off",
                    "-c SHIELDXL cset name=Popguard Switch on",
                    "-c SHIELDXL cset name=Auto-Mute Switch on",
                ],
            )


if __name__ == "__main__":
    unittest.main()
