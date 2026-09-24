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

    def __init__(self, group=None, proc_root='/proc', cgroup_root='/sys/fs/cgroup', supervisor=None, installer_operation=None, renderer_operation=None, dependency_operation=None, daw_operation=None):
        self.proc_root = pathlib.Path(proc_root)
        self.cgroup_root = pathlib.Path(cgroup_root)
        self.supervisor = os.getpid() if supervisor is None else supervisor
        actual = self.group_of(self.supervisor)
        self.group = actual if group is None else group
        suffix='/linux-vst-bridge-vendor-arturia-software-center.service'
        if installer_operation is not None:
            if not isinstance(installer_operation,str) or len(installer_operation)!=32 or any(c not in '0123456789abcdef' for c in installer_operation):fail('installer operation identity')
            suffix='/linux-vst-bridge-installer-'+installer_operation+'.service'
        if renderer_operation is not None:
            if installer_operation is not None or not isinstance(renderer_operation,str) or len(renderer_operation)!=32 or any(c not in '0123456789abcdef' for c in renderer_operation):fail('renderer operation identity')
            suffix='/linux-vst-bridge-renderer-'+renderer_operation+'.service'
        if dependency_operation is not None:
            if installer_operation is not None or renderer_operation is not None or daw_operation is not None or not isinstance(dependency_operation,str) or len(dependency_operation)!=32 or any(c not in '0123456789abcdef' for c in dependency_operation):fail('dependency operation identity')
            suffix='/linux-vst-bridge-dependency-'+dependency_operation+'.service'
        if daw_operation is not None:
            if any(v is not None for v in (installer_operation,renderer_operation,dependency_operation)) or not isinstance(daw_operation,str) or len(daw_operation)!=32 or any(c not in '0123456789abcdef' for c in daw_operation):fail('DAW operation identity')
            suffix='/linux-vst-bridge-daw-fl-'+daw_operation+'.service'
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
        self.records={}; self.dropped=0; self.unattributed_waits=0; self.unattributed_exits=[]
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
            else:
                self.unattributed_waits+=1
                if len(self.unattributed_exits)<64:self.unattributed_exits.append({'pid':info.si_pid,'status':code,'domain':'linux_wait','observed_ns':self.clock(),'ownership':'unverified'})
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
                'unattributed_adopted_exits':self.unattributed_waits,'unattributed_exit_observations':self.unattributed_exits,
                'dropped_unattributed_exit_observations':max(0,self.unattributed_waits-len(self.unattributed_exits)),'first_failure':self.first_failure,
                'persistence_failures':self.persistence_failures}

# NAD1 qualified Linux lead and generation census.
"""NAD1 read-only Linux-generation census. Names admit leads, never ownership.

No Windows PID joins, signalling, service requests, socket probes, or environment
publication. Callers must keep `private` outside public receipts.
"""
import collections
import hashlib
import os
import pathlib
import re
import stat
import time

NAME = 'NTKDaemon.exe'
LIMITS = {'stat': (4096, 1), 'comm': (256, 1), 'cmdline': (65536, 4096),
          'maps': (2 * 1024 * 1024, 32768), 'environ': (262144, 8192)}
CATEGORIES = frozenset({'stat_unavailable', 'command_unavailable', 'maps_unavailable',
    'environment_unavailable', 'prefix_relation_unavailable', 'image_unavailable',
    'process_reused', 'extent_exhausted', 'record_invalid'})

class Extent(ValueError):
    pass


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def bounded(path, kind):
    maximum, records = LIMITS[kind]
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    chunks = []; total = 0; count = 0
    separator = b'\0' if kind in ('cmdline', 'environ') else b'\n'
    try:
        while True:
            part = os.read(fd, min(8192, maximum + 1 - total))
            if not part:
                break
            chunks.append(part); total += len(part); count += part.count(separator)
            if total > maximum or count > records:
                raise Extent(kind)
        return b''.join(chunks)
    finally:
        os.close(fd)


def generation(raw, pid):
    match = re.fullmatch(rb'([0-9]+) \((.*)\) (.*)\n?', raw, re.DOTALL)
    if not match or int(match[1]) != pid:
        raise ValueError('stat_identity')
    fields = match[3].split()
    if len(fields) < 20 or not fields[19].isdigit():
        raise ValueError('stat_extent')
    return int(fields[19]), match[2]


def token_lead(raw):
    # Only complete NUL-delimited argv entries, never free-form substring/prose.
    if raw and not raw.endswith(b'\0'):
        raise ValueError('command_termination')
    for token in raw.split(b'\0'):
        if token == NAME.encode() or re.fullmatch(
                rb'(?:/[^\0\r\n]+/|[A-Za-z]:\\[^\0\r\n]+\\)NTKDaemon\.exe', token):
            return True
    return False


def mapped_images(raw):
    found = set()
    for line in raw.splitlines():
        fields = line.split(None, 5)
        if len(fields) == 6:
            location = fields[5]
            deleted = location.endswith(b' (deleted)')
            if deleted:
                location = location[:-10]
            if location.startswith(b'/') and location.rsplit(b'/', 1)[-1] == NAME.encode():
                found.add((os.fsdecode(location), deleted))
    return found


def image_identity(path):
    p = pathlib.Path(path)
    if not p.is_absolute() or p.resolve() != p:
        raise ValueError('image_alias')
    fd = os.open(p, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        before = os.fstat(fd)
        if not stat.S_ISREG(before.st_mode) or before.st_nlink != 1 or not 64 <= before.st_size <= 256*1024*1024:
            raise ValueError('image_extent_or_alias')
        header = os.read(fd, 64)
        if header[:2] != b'MZ':
            raise ValueError('not_pe')
        offset = int.from_bytes(header[60:64], 'little')
        if offset + 26 > before.st_size:
            raise ValueError('pe_extent')
        os.lseek(fd, offset, os.SEEK_SET); pe = os.read(fd, 26)
        if len(pe) != 26 or pe[:4] != b'PE\0\0' or pe[4:6] not in (b'\x4c\x01', b'\x64\x86'):
            raise ValueError('pe_identity')
        os.lseek(fd, 0, os.SEEK_SET); h = hashlib.sha256(); left = before.st_size
        while left:
            data = os.read(fd, min(left, 65536))
            if not data: raise ValueError('image_short')
            left -= len(data); h.update(data)
        def stamp(s): return s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns
        if stamp(before) != stamp(os.fstat(fd)) or stamp(before) != stamp(p.lstat()):
            raise ValueError('image_changed')
        return {'sha256': h.hexdigest(), 'size': before.st_size,
                'architecture': 'x86' if pe[4:6] == b'\x4c\x01' else 'x64'}
    finally:
        os.close(fd)


def prefix_relation(raw, prefix):
    if raw and not raw.endswith(b'\0'):
        raise ValueError('environment_termination')
    values = [v[11:] for v in raw.split(b'\0') if v.startswith(b'WINEPREFIX=')]
    if len(values) != 1 or not values[0]:
        raise ValueError('prefix_unavailable')
    p = pathlib.Path(os.fsdecode(values[0]))
    if not p.is_absolute() or '..' in p.parts:
        raise ValueError('prefix_unavailable')
    if not p.exists(): return 'deleted'
    if p.resolve() != p: raise ValueError('prefix_alias')
    return 'same' if p == pathlib.Path(prefix).resolve() else 'foreign'


def census(prefix, admitted=None, proc=pathlib.Path('/proc'), *, reader=bounded):
    """`admitted` is an internal exact artifact, never an operator selector.

    A full no-lead scan establishes absence under the generated-proven lead law.
    Unreadable unled tasks are ignored; exhausting a readable lead-search extent
    is explicit incompleteness because a lead could lie beyond the bound.
    """
    candidates=[]; failures=[]; private=[]; unled_unreadable=0; visited=0
    deadline=time.monotonic()+30; search_exhausted=0
    with os.scandir(proc) as entries:
        for entry in entries:
            if not entry.name.isdigit(): continue
            visited+=1
            if visited>32768 or time.monotonic()>deadline:
                search_exhausted+=1; break
            p=pathlib.Path(entry.path); pid=int(entry.name); values={}; errors={}; leads=[]
            # Linux comm/stat, argv, and mapped semantic image are independent
            # bounded lead sources. Failure without a positive lead has no role.
            for kind in ('stat','comm','cmdline','maps'):
                try: values[kind]=reader(p/kind,kind)
                except Extent: errors[kind]='extent_exhausted'
                except (OSError,ValueError): errors[kind]='unavailable'
            before=None
            try:
                if 'stat' in values:
                    before,comm=generation(values['stat'],pid)
                    if comm==NAME.encode(): leads.append('linux_stat_comm_exact')
            except ValueError: errors['stat']='unavailable'
            if values.get('comm',b'').rstrip(b'\n')==NAME.encode(): leads.append('linux_comm_exact')
            try:
                if token_lead(values.get('cmdline',b'')): leads.append('nul_command_token_exact')
            except ValueError: errors['cmdline']='unavailable'
            images=mapped_images(values.get('maps',b''))
            if images: leads.append('mapped_pe_basename_exact')
            if not leads:
                if 'extent_exhausted' in errors.values(): search_exhausted+=1
                elif errors: unled_unreadable+=1
                continue
            generation_hash=sha((str(pid)+':'+str(before)).encode())
            failure=None; stage=None; relation='unavailable'; identity=None; exact=False
            names={'stat':'stat_unavailable','cmdline':'command_unavailable','maps':'maps_unavailable'}
            for kind in ('stat','cmdline','maps'):
                if kind in errors:
                    stage=kind;failure='extent_exhausted' if errors[kind]=='extent_exhausted' else names[kind];break
            if before is None and failure is None:stage='stat';failure='stat_unavailable'
            if failure is None:
                stage='environment'
                try: environment=reader(p/'environ','environ')
                except Extent: failure='extent_exhausted'
                except (OSError,ValueError): failure='environment_unavailable'
            if failure is None:
                stage='prefix_relation'
                try: relation=prefix_relation(environment,prefix)
                except (OSError,ValueError): failure='prefix_relation_unavailable'
            if failure is None:
                stage='image'
                try:
                    if len(images)!=1: raise ValueError('image_count')
                    path,deleted=next(iter(images))
                    if deleted: raise ValueError('image_deleted')
                    identity=image_identity(path)
                    if admitted is None or identity!=admitted: raise ValueError('image_not_admitted')
                    exact=True
                except (OSError,ValueError): failure='image_unavailable'
            # Even failed detailed reads must not attach to a recycled generation.
            try:
                after,_=generation(reader(p/'stat','stat'),pid)
                if before is None or after!=before: failure='process_reused';stage='generation_recheck';exact=False
            except Extent: failure='extent_exhausted';stage='generation_recheck';exact=False
            except (OSError,ValueError): failure='stat_unavailable';stage='generation_recheck';exact=False
            row={'ordinal':len(candidates)+1,'generation_sha256':generation_hash,
                 'lead_authorities':sorted(leads),'prefix_relation':relation,
                 'exact':exact,'image':identity}
            candidates.append(row)
            if failure:
                assert failure in CATEGORIES
                item={'generation_sha256':generation_hash,'lead_authorities':sorted(leads),
                      'failure_stage':stage,'category':failure}
                failures.append(item)
                private.append(dict(item,linux_pid=pid,start_ticks=before,
                                    input_sha256={k:sha(v) for k,v in values.items()}))
            else:
                private.append({'generation_sha256':generation_hash,'linux_pid':pid,'start_ticks':before,'prefix_relation':relation,'exact':exact})
    counts=dict(sorted(collections.Counter(x['category'] for x in failures).items()))
    return {'candidates':candidates,'unavailable':len(failures)+search_exhausted,
            'visited':visited,'unled_unreadable':unled_unreadable,
            'search_extent_exhausted':search_exhausted,'failure_counts':counts,
            'failure_hashes':[sha(__import__('json').dumps(x,sort_keys=True).encode()) for x in failures],
            'private':private}
