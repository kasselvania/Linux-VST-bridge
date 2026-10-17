"""Recipe identity changes when same-revision patches or build flags change."""
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest.mock import patch

SOURCE = Path(__file__).with_name("build-identity.py")
spec = importlib.util.spec_from_file_location("build_identity", SOURCE)
identity = importlib.util.module_from_spec(spec)
spec.loader.exec_module(identity)


class RecipeTests(unittest.TestCase):
    def test_flags_and_patch_order_bind_recipe_but_changed_unpinned_patch_refuses(self):
        recipe = json.loads((identity.HERE / "source.json").read_bytes())
        names = {"tools/runtime/" + name for name in identity.RECIPE_FILES}
        names.update("tools/" + item["path"] for item in [*recipe["patches"], *recipe["fixtures"]])
        names.update("tools/runtime/patches/" + name for name in
                     ("proton-redist-inputs.patch", "proton-audio-composition.patch"))
        original = identity.recipe_identity()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for name in names:
                target = root / name
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(identity.ROOT / name, target)
            with patch.object(identity, "ROOT", root), patch.object(identity, "HERE", root / "tools/runtime"):
                self.assertEqual(identity.recipe_identity(), original)
                invocation = root / "tools/runtime/build-inner.sh"
                data = invocation.read_bytes()
                self.assertIn(b"make -j3 J=3", data)
                invocation.write_bytes(data.replace(b"make -j3 J=3", b"make -j3 J=3 CFLAGS=-O0", 1))
                self.assertNotEqual(identity.recipe_identity(), original)
                invocation.write_bytes(data)
                inputs = root / "tools/runtime/source.json"
                data = inputs.read_bytes()
                recipe["patches"][:2] = reversed(recipe["patches"][:2])
                inputs.write_text(json.dumps(recipe))
                self.assertNotEqual(identity.recipe_identity(), original)
                inputs.write_bytes(data)
                source_patch = root / "tools" / recipe["patches"][0]["path"]
                source_patch.write_bytes(source_patch.read_bytes() + b"\n")
                with self.assertRaisesRegex(ValueError, "Pinned recipe input differs"):
                    identity.recipe_identity()


if __name__ == "__main__":
    unittest.main()
