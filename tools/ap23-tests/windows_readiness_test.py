"""Native Windows SDK ordering and failed-stop containment, without VM pacing."""
import pathlib
import json
import subprocess
import sys
import time

exe = pathlib.Path(sys.argv[1]).resolve()
for args, expected in [([], 0), (["--refuse-stop"], 93), (["--hang"], 93), (["--cancel-restart"], 0)]:
    start = time.monotonic()
    result = subprocess.run([str(exe), *args], capture_output=True, text=True, timeout=12)
    duration = time.monotonic() - start
    print(result.stdout, end="")
    assert "private-vendor-start-exception-marker" not in result.stdout + result.stderr
    print(f"prepared render scenario {args or ['lifecycle']}: exit={result.returncode}, seconds={duration:.3f}")
    if result.returncode != expected:
        raise AssertionError(f"expected {expected}, got {result.returncode}: {result.stderr[-4000:]}")
    assert duration < 9, duration
    if args == ["--hang"]:
        assert duration >= 4.5, duration
    if args == ["--refuse-stop"]:
        records = [json.loads(line) for line in result.stdout.splitlines() if line.startswith("{")]
        failed_stop = [r for r in records if r.get("state") == "ap0_call_completed" and r.get("operation") == "set_processing_false"]
        assert len(failed_stop) == 1 and failed_stop[0]["result"] == 1, failed_stop
        assert not any(r.get("state") == "ap0_call_started" and r.get("operation") == "set_active_false" for r in records)
    if not args:
        records = [json.loads(line) for line in result.stdout.splitlines() if line.startswith("{")]
        failed_starts = [r for r in records if r.get("state") == "ap0_call_started" and r.get("operation") == "set_processing_true" and r.get("returned") is False]
        assert len(failed_starts) == 2, failed_starts
    if args == ["--cancel-restart"]:
        records = [json.loads(line) for line in result.stdout.splitlines() if line.startswith("{")]
        transitions = [r for r in records if r.get("state") == "ap0_call_started" and r.get("operation") in ("set_processing_true", "set_processing_false")]
        assert [r["operation"] for r in transitions] == ["set_processing_true", "set_processing_false"], transitions
print("prepared render ordering, quiescence, failure wake/join and refused-stop containment passed")
