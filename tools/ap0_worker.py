#!/usr/bin/env python3
"""Class-specific entrypoint; verified shared helpers own execution and retention."""
import json
import hashlib
import pathlib
import sys
import types

_SUPPORT_SOURCES = {}
sys.path.insert(0, str(pathlib.Path.cwd() / "tools/wf0-factory-census"))
for name, source in _SUPPORT_SOURCES.items():
    identity = hashlib.sha256(source.encode()).hexdigest()
    if name in sys.modules:
        if getattr(sys.modules[name], "_pc0_source_sha256", None) != identity:
            raise RuntimeError("worker helper module identity differs")
        continue
    module = types.ModuleType(name)
    module.__file__ = "<" + name + ">"
    module._pc0_source_sha256 = identity
    sys.modules[name] = module
    exec(compile(source, module.__file__, "exec"), module.__dict__)
from pc0_execution_worker import main
main(json.loads(bytes.fromhex(sys.argv[3]))["execution_class"])
