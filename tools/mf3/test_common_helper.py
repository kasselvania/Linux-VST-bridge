"""Build-time exact pairing refusals; no compiler or marker tool is executed."""
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("kit", ROOT / "tools/mf3/prebuilt_kit.py")
kit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(kit)


class PairingTests(unittest.TestCase):
    def test_wrong_tool_refuses_before_any_subprocess_and_recipe_digest_is_exact(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            marker = root / "winebuild"
            marker.write_bytes(b"different executable must never run")
            declaration = root / "wine-build.json"
            names = ("linux_wait.h", "linux_wait.rs", "shim.cpp")
            value = dict(schema=1, kind="wine_build", helper_abi=1, wine_revision="c" * 40,
                helper_sources={"tools/direct-audio-handoff/" + name:
                    kit.sha((ROOT / "tools/direct-audio-handoff" / name).read_bytes()) for name in names},
                winebuild_sha256="0" * 64)
            declaration.write_text(json.dumps(value))
            arguments = ["kit", "--sdk", tmp, "--windows-package", tmp, "--output", str(root / "kit.zip"),
                         "--winebuild", str(marker), "--runtime-build", str(declaration)]
            with patch.object(sys, "argv", arguments), patch.object(kit, "git", return_value=""), \
                 patch.object(kit.subprocess, "run") as execute:
                with self.assertRaisesRegex(ValueError, "marker tool binding"):
                    kit.main()
                execute.assert_not_called()
            value["winebuild_sha256"] = kit.sha(marker.read_bytes())
            declaration.write_text(json.dumps(value))
            _, digest, executable = kit.common_helper_binding(declaration, marker)
            self.assertEqual(digest, kit.sha(declaration.read_bytes()))
            self.assertEqual(executable, marker.read_bytes())
            value["helper_sources"]["tools/direct-audio-handoff/linux_wait.h"] = "0" * 64
            declaration.write_text(json.dumps(value))
            with self.assertRaisesRegex(ValueError, "source/ABI binding"):
                kit.common_helper_binding(declaration, marker)


if __name__ == "__main__":
    unittest.main()
