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


def cleanup_process(root: subprocess.Popen[bytes], owned: list[tuple[int, int]], during_cleanup=None) -> dict[str, Any]:
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
        if during_cleanup is not None:
            during_cleanup()
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

    def __init__(self, group=None, proc_root='/proc', cgroup_root='/sys/fs/cgroup', supervisor=None, installer_operation=None):
        self.proc_root = pathlib.Path(proc_root)
        self.cgroup_root = pathlib.Path(cgroup_root)
        self.supervisor = os.getpid() if supervisor is None else supervisor
        actual = self.group_of(self.supervisor)
        self.group = actual if group is None else group
        suffix='/linux-vst-bridge-vendor-arturia-software-center.service'
        if installer_operation is not None:
            if not isinstance(installer_operation,str) or len(installer_operation)!=32 or any(c not in '0123456789abcdef' for c in installer_operation):fail('installer operation identity')
            suffix='/linux-vst-bridge-installer-'+installer_operation+'.service'
        if (not self.group or self.group != actual or '..' in self.group.split('/')
                or not self.group.endswith(suffix)):
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

class InstallerLedger:
    """Installer-only, private cgroup/PID-start custody. No name-based ownership.

    Linux cgroup.procs omits zombies. Known identities and WNOWAIT children are
    therefore revalidated against /proc/PID/cgroup before reap as well. A child
    already reaped by its own parent has an explicitly unavailable exit status.
    Windows DWORD exits are not inferred from Linux's eight-bit exit status.
    """
    LIMIT = 512

    def __init__(self, scope, persist, clock=time.monotonic_ns):
        self.scope=scope; self.persist=persist; self.clock=clock
        self.records={}; self.dropped=0; self.unattributed_waits=0
        self.first_failure=None; self.cancelled=False; self.launchers={}
        self.persistence_failures=0; self.phase='environment_bootstrap'

    def checked(self, pid):
        before=self.scope.identity(pid); group=self.scope.group_of(pid); after=self.scope.identity(pid)
        if not before or not after or before['start_ticks']!=after['start_ticks'] or not self.scope.contains(group):return None
        return dict(after,cgroup=group)

    def add(self, observed, phase=None):
        phase=phase or self.phase
        key=(observed['pid'],observed['start_ticks']); now=self.clock()
        row=self.records.get(key)
        if row is None:
            if len(self.records)>=self.LIMIT:self.dropped+=1;return None
            parent=self.checked(observed['ppid']);child_after=self.checked(key[0])
            if not child_after or child_after['start_ticks']!=key[1] or parent and child_after['ppid']!=parent['pid']:parent=None
            row={'pid':key[0],'start_ticks':key[1],'parent':None if parent is None else {'pid':parent['pid'],'start_ticks':parent['start_ticks']},
                 'first_ns':now,'last_ns':now,'cgroup':observed['cgroup'],'state':observed['state'],
                 'relationship':'direct_launcher' if observed['pid'] in self.launchers else 'descendant','phase':phase,'role':'unknown','role_evidence':None,
                 'linux_exit':None,'windows_exit':None,'exit_availability':'unavailable',
                 'images':[],'image_observation':'unavailable'}
            self.records[key]=row
        row['last_ns']=now; row['state']=observed['state']
        if observed['state']=='Z' and observed.get('exit_code') is not None:
            self.exit(row,os.waitstatus_to_exitcode(observed['exit_code']),'proc_zombie_wait_status')
        return row

    def launcher(self, child, phase):
        observed=self.checked(child.pid)
        if observed is None:raise RuntimeError('installer launcher cgroup identity unavailable')
        self.launchers[child.pid]=(child,observed['start_ticks']);self.phase=phase
        row=self.add(observed,phase)
        if row is None:raise RuntimeError('installer launcher ledger full')
        row['relationship']='direct_launcher';row['phase']=phase
        self.launchers[child.pid]=(child,observed['start_ticks'])
        self.commit()

    def exit(self,row,code,source):
        if row['linux_exit'] is not None:
            if row['linux_exit']['status']!=code:raise RuntimeError('installer exit custody conflict')
            if source=='waitpid':row['linux_exit']['reaped']=True
            return
        row['linux_exit']={'domain':'linux_wait','status':code,'source':source,'observed_ns':self.clock(),'reaped':source=='waitpid'}
        row['exit_availability']='observed';row['state']='exited'
        if code!=0 and not self.cancelled and self.first_failure is None:
            self.first_failure={'pid':row['pid'],'start_ticks':row['start_ticks'],'role':row['role'],
                'phase':row['phase'],'relationship':row['relationship'],'domain':'linux_wait','status':code,
                'observed_ns':row['linux_exit']['observed_ns'],'cause':'unestablished'}

    def commit(self):
        try:self.persist(self.value())
        except (OSError,ValueError):self.persistence_failures+=1

    def sample(self):
        members=self.scope.members();seen=set()
        for observed in members:
            key=(observed['pid'],observed['start_ticks']);seen.add(key);self.add(observed)
        # Includes zombies omitted from cgroup.procs. Never acquire a recycled PID.
        for key,row in list(self.records.items()):
            if key in seen:continue
            current=self.checked(key[0])
            if current and current['start_ticks']==key[1]:
                self.add(current);members.append(current)
            elif row['linux_exit'] is None:
                row['state']='disappeared';row['exit_availability']='unavailable_parent_reaped_or_unobserved'
        return members

    def harvest(self):
        self.sample()
        # WNOWAIT preserves /proc identity, cgroup and zombie status until custody
        # has been durably attempted. Never Popen.poll()/wait before this owner.
        for _ in range(self.LIMIT):
            try:info=os.waitid(os.P_ALL,0,os.WEXITED|os.WNOHANG|os.WNOWAIT)
            except ChildProcessError:break
            if info is None or info.si_pid==0:break
            current=self.checked(info.si_pid)
            row=self.add(current) if current else None
            code=info.si_status if info.si_code==os.CLD_EXITED else -info.si_status
            if row:self.exit(row,code,'waitid_wnowait')
            else:self.unattributed_waits+=1
            self.commit() # first failure survives subsequent cancellation/reaping
            pid,status=os.waitpid(info.si_pid,os.WNOHANG)
            if pid:
                actual=os.waitstatus_to_exitcode(status)
                if row:self.exit(row,actual,'waitpid')
                owner=self.launchers.get(pid)
                if owner and current and owner[1]==current['start_ticks']:owner[0].returncode=actual
        remaining=self.sample();self.commit();return remaining

    def cleanup(self,timeout=CLEANUP_SECONDS):
        deadline=time.monotonic()+timeout
        while True:
            members=self.harvest()
            if not members:return True
            if time.monotonic()>=deadline:return False
            self.scope.signal_members(signal.SIGKILL if time.monotonic()>deadline-3 else signal.SIGTERM)
            time.sleep(POLL_SECONDS)

    def value(self):
        return {'schema':1,'processes':list(self.records.values()),'dropped_process_observations':self.dropped,
                'unattributed_adopted_exits':self.unattributed_waits,'first_failure':self.first_failure,
                'persistence_failures':self.persistence_failures}
