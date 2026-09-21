import os
import pathlib
import stat
import subprocess
import tempfile
import unittest


ROOT = pathlib.Path(__file__).parents[1]
SCRIPT = ROOT / "mixer-state.sh"


class MixerStateTest(unittest.TestCase):
    def test_apply_uses_only_declared_wm8731_controls(self):
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
                    "-c sndrpiproto cset name=Line Bypass Switch off",
                    "-c sndrpiproto cset name=HiFi Playback Switch on",
                    "-c sndrpiproto cset name=Mic Sidetone Switch off",
                    "-c sndrpiproto cset name=Input Mux Line In",
                    "-c sndrpiproto cset name=Master Playback Volume 100,100",
                    "-c sndrpiproto cset name=Master Playback ZC Switch on",
                    "-c sndrpiproto cset name=Capture Volume 23,23",
                    "-c sndrpiproto cset name=Line Capture Switch on,on",
                    "-c sndrpiproto cset name=Mic Boost Volume 0",
                    "-c sndrpiproto cset name=Mic Capture Switch off",
                    "-c sndrpiproto cset name=Sidetone Playback Volume 0",
                    "-c sndrpiproto cset name=ADC High Pass Filter Switch on",
                    "-c sndrpiproto cset name=Store DC Offset Switch off",
                    "-c sndrpiproto cset name=Playback Deemphasis Switch off",
                ],
            )


if __name__ == "__main__":
    unittest.main()
