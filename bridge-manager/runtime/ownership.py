"""AP9 stat-only identity tracking and bounded positive cleanup.

Extracted from pc0_diagnostic_primitives; installed playback has no proof-harness
or git imports. PID/start-time identity is never replaced with process names.
"""
import os,signal,subprocess,time
from collections import deque
from typing import Any
CLEANUP_SECONDS=10.0
POLL_SECONDS=0.05
def fail(message): raise RuntimeError(message)

def process_identities(proc_root="/proc"):
    """Fresh stat-only census shared by ongoing tracking and cleanup."""
    records = []
    with os.scandir(proc_root) as entries:
        for entry in entries:
            if not entry.name.isdigit():
                continue
            try:
                with open(entry.path + "/stat", encoding="utf-8", errors="replace") as handle:
                    raw = handle.read()
                pid, separator, _ = raw.partition(" (")
                if not separator or int(pid) != int(entry.name):
                    continue
                # Field 2 is parenthesized and may itself contain ')' or spaces.
                fields = raw.rsplit(")", 1)[1].split()
                records.append({"pid": int(pid), "ppid": int(fields[1]),
                                "pgrp": int(fields[2]), "session": int(fields[3]),
                                "start_ticks": int(fields[19])})
            except (FileNotFoundError, ProcessLookupError, PermissionError, ValueError, IndexError):
                continue
    return records


def descendant_identities(root_pid, records=None):
    """Discover the complete parent tree afresh; never cache PID identities."""
    by_parent = {}
    for record in (process_identities() if records is None else records):
        by_parent.setdefault(record["ppid"], []).append(record)
    result, seen, queue = [], set(), deque([root_pid])
    while queue:
        for child in by_parent.get(queue.popleft(), []):
            if child["pid"] in seen:
                continue
            seen.add(child["pid"])
            result.append(child)
            queue.append(child["pid"])
    return result


def cleanup_process(root: subprocess.Popen[bytes], owned: list[tuple[int, int]]) -> dict[str, Any]:
    # Cleanup ownership is an observed PID/start identity or this root's
    # process group/session, never a stage-shaped string in another command.
    deadline = time.monotonic() + CLEANUP_SECONDS
    live_owned = {
        (record["pid"], record["start_ticks"]) for record in process_identities()
    } & set(owned)
    if root.poll() is None or live_owned:
        try:
            os.killpg(root.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
    while time.monotonic() < deadline - 3:
        live_owned = {
            (record["pid"], record["start_ticks"]) for record in process_identities()
        } & set(owned)
        if root.poll() is not None and not live_owned:
            break
        time.sleep(POLL_SECONDS)
    if root.poll() is None or live_owned:
        try:
            os.killpg(root.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
    try:
        root.wait(timeout=max(0.1, deadline - time.monotonic()))
    except subprocess.TimeoutExpired:
        fail("Runtime root did not terminate inside cleanup bound")
    remaining = []
    for record in process_identities():
        if ((record["pid"], record["start_ticks"]) in owned or
                (record["pgrp"] == root.pid and record["session"] == root.pid)):
            remaining.append(record["pid"])
    if remaining:
        fail(f"owned descendants survived cleanup: {remaining}")
    return {"owned_descendants_zero": True, "process_group_empty": True}


