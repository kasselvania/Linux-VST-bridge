"""AP9 stat-only identity tracking and bounded positive cleanup.

Extracted from pc0_diagnostic_primitives; installed playback has no proof-harness
or git imports. PID/start-time identity is never replaced with process names.
"""
import os,pathlib,signal,subprocess,time
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


class ProcessTracker:
    """Follow this launch's kernel child lists, including non-main threads.

    Known descendants remain roots after reparenting. Every visit checks the
    PID/start-time pair afresh; recycled PIDs never acquire ownership. The full
    stat census is retained at admission and for positive cleanup.
    """
    def __init__(self, root_pid, proc_root="/proc"):
        self.proc_root = proc_root
        census = process_identities(proc_root)
        roots = [r for r in census if r['pid'] == root_pid]
        self.owned = {(r['pid'], r['start_ticks'])
                      for r in roots + descendant_identities(root_pid, census)}

    def identity(self, pid):
        try:
            with open(f"{self.proc_root}/{pid}/stat", encoding="utf-8", errors="replace") as f:
                raw = f.read()
            actual, separator, _ = raw.partition(" (")
            fields = raw.rsplit(")", 1)[1].split()
            if not separator or int(actual) != pid:
                return None
            return int(fields[19]), int(fields[1])
        except (OSError, ValueError, IndexError):
            return None

    def update(self):
        pending = deque(self.owned)
        visited = set()
        while pending:
            pid, start = pending.popleft()
            if (pid, start) in visited:
                continue
            visited.add((pid, start))
            before = self.identity(pid)
            if before is None or before[0] != start:
                continue
            children = set()
            try:
                with os.scandir(f"{self.proc_root}/{pid}/task") as tasks:
                    for task in tasks:
                        if not task.name.isdigit():
                            continue
                        try:
                            with open(task.path + "/children", encoding="ascii") as f:
                                children.update(int(child) for child in f.read().split())
                        except (FileNotFoundError, ProcessLookupError):
                            continue
            except (FileNotFoundError, ProcessLookupError):
                continue
            # Parent exit/reuse during the scan cannot authorize another tree.
            after = self.identity(pid)
            if after is None or after[0] != start:
                continue
            for child in children:
                identity = self.identity(child)
                if identity is not None and identity[1] == pid:
                    key = (child, identity[0])
                    self.owned.add(key)
                    pending.append(key)
        return self.owned


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



class CompanionCgroup:
    """AP18 companion-only authority: the dedicated systemd unit, not ancestry.

    A process may double-fork or be created by wineserver between samples. It is
    still a member of this unit. Unknown members keep the operation alive too.
    This observer is never used by audio-session supervision.
    """
    MAX_PROCESSES = 4096
    MAX_GROUPS = 64

    def __init__(self, group=None, proc_root='/proc', cgroup_root='/sys/fs/cgroup', supervisor=None):
        self.proc_root = pathlib.Path(proc_root)
        self.cgroup_root = pathlib.Path(cgroup_root)
        self.supervisor = os.getpid() if supervisor is None else supervisor
        actual = self.group_of(self.supervisor)
        self.group = actual if group is None else group
        if (not self.group or self.group != actual or '..' in self.group.split('/')
                or not self.group.endswith('/linux-vst-bridge-vendor-arturia-software-center.service')):
            fail('companion requires its exact dedicated cgroup')
        self.root = self.cgroup_root / self.group.lstrip('/')
        self.root_identity = (self.root.stat().st_dev, self.root.stat().st_ino)

    def group_of(self, pid):
        try:
            lines = (self.proc_root / str(pid) / 'cgroup').read_text().splitlines()
        except (FileNotFoundError, ProcessLookupError):
            return None
        return next((line[3:] for line in lines if line.startswith('0::')), None)

    def identity(self, pid):
        try:
            raw = (self.proc_root / str(pid) / 'stat').read_text()
            head, sep, _ = raw.partition(' ('); fields = raw.rsplit(')', 1)[1].split()
            if not sep or int(head) != pid: fail('companion malformed process identity')
            return {'pid':pid, 'start_ticks':int(fields[19]), 'ppid':int(fields[1]),
                    'state':fields[0], 'exit_code':int(fields[49]) if len(fields)>49 else None}
        except (FileNotFoundError, ProcessLookupError):
            return None

    def contains(self, group):
        return group == self.group or (group is not None and group.startswith(self.group + '/'))

    def members(self):
        current = self.root.stat()
        if (current.st_dev, current.st_ino) != self.root_identity: fail('companion cgroup replaced')
        pids=set(); groups=[self.root]; visited=0
        while groups:
            directory=groups.pop(); visited+=1
            if visited>self.MAX_GROUPS: fail('companion cgroup bound')
            try:
                for value in (directory/'cgroup.procs').read_text().split():
                    pids.add(int(value))
                    if len(pids)>self.MAX_PROCESSES: fail('companion process bound')
                for entry in directory.iterdir():
                    if entry.is_dir() and not entry.is_symlink():groups.append(entry)
            except FileNotFoundError:
                if directory==self.root:raise
        result=[]
        for pid in sorted(pids):
            if pid==self.supervisor:continue
            before=self.identity(pid); group=self.group_of(pid); after=self.identity(pid)
            if before is None or after is None:continue
            if before['start_ticks']!=after['start_ticks'] or not self.contains(group):continue
            after['cgroup']=group
            result.append(after)
        return result

    def signal_members(self, sig):
        # pidfds prevent signalling a recycled PID. Membership is rechecked after
        # opening the pidfd; never signal by session, command substring or name.
        for record in self.members():
            if record['state']=='Z':continue
            try:
                fd=os.pidfd_open(record['pid'])
                try:
                    now=self.identity(record['pid'])
                    if (now and now['start_ticks']==record['start_ticks']
                            and self.contains(self.group_of(record['pid']))):
                        signal.pidfd_send_signal(fd,sig)
                finally:os.close(fd)
            except ProcessLookupError:pass

    def cleanup(self, reap, timeout=CLEANUP_SECONDS):
        deadline=time.monotonic()+timeout
        while True:
            reap(); live=[p for p in self.members() if p['state']!='Z']
            if not live:return True
            if time.monotonic()>=deadline:return False
            self.signal_members(signal.SIGKILL if time.monotonic()>deadline-3 else signal.SIGTERM)
            time.sleep(POLL_SECONDS)
