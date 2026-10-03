"""Child isolation proves both controlled cancellation and hung-vendor bounds."""
import pathlib
import subprocess
import sys
import time

exe = pathlib.Path(sys.argv[1]).resolve()
for args, expected in [([], 0), (["--hang"], 93)]:
    start = time.monotonic()
    child = subprocess.run([str(exe), *args], capture_output=True, text=True, timeout=12)
    duration = time.monotonic() - start
    print(child.stdout, end="")
    print(f"owner scenario {args or ['cancellable']}: exit={child.returncode}, seconds={duration:.3f}")
    if child.returncode != expected:
        raise AssertionError(f"expected {expected}, got {child.returncode}; 96 identifies std::terminate before owned shutdown")
    if args:
        assert 4.5 <= duration < 9, duration
print("production SDK owner exception and bounded hung-worker containment passed")
