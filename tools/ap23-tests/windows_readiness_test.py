"""Native Windows SDK ordering and failed-stop containment, without VM pacing."""
import pathlib
import subprocess
import sys
import time

exe = pathlib.Path(sys.argv[1]).resolve()
for args, expected in [([], 0), (["--refuse-stop"], 93)]:
    start = time.monotonic()
    result = subprocess.run([str(exe), *args], capture_output=True, text=True, timeout=12)
    duration = time.monotonic() - start
    print(result.stdout, end="")
    print(f"prepared render scenario {args or ['lifecycle']}: exit={result.returncode}, seconds={duration:.3f}")
    if result.returncode != expected:
        raise AssertionError(f"expected {expected}, got {result.returncode}: {result.stderr[-4000:]}")
    assert duration < 9, duration
print("prepared render ordering, quiescence, failure wake/join and refused-stop containment passed")
