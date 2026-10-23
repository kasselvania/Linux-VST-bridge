"""Fixed host bootstrap embedded in the product, never imported from a package."""
import sys

if sys.implementation.name != "cpython" or not (3, 13) <= sys.version_info[:2] < (3, 16):
    raise SystemExit("supervisor requires host CPython >=3.13,<3.16")
if sys.argv[1:] == ["--probe"]:
    print("cpython-source-v1")
    raise SystemExit(0)

import hashlib
import pathlib
import types

session_path, session_sha, ownership_path, ownership_sha = sys.argv[1:5]
def verified(path, expected):
    source = pathlib.Path(path)
    if not source.is_absolute() or source.resolve() != source:
        raise SystemExit("supervisor source path changed")
    data = source.read_bytes()
    if hashlib.sha256(data).hexdigest() != expected:
        raise SystemExit("supervisor source changed")
    return data

# Read and authenticate both before executing either; compile these same bytes.
session = verified(session_path, session_sha)
ownership = verified(ownership_path, ownership_sha)
module = types.ModuleType("ownership")
module.__file__ = ownership_path
module.__package__ = ""
sys.modules["ownership"] = module
exec(compile(ownership, ownership_path, "exec"), module.__dict__)
sys.argv = [session_path, *sys.argv[5:]]
main = types.ModuleType("__main__")
main.__file__ = session_path
main.__package__ = None
sys.modules["__main__"] = main
exec(compile(session, session_path, "exec"), main.__dict__)
