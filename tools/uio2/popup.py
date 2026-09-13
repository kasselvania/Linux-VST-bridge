"""Exact diagnostic window graph and one-use, custodian-issued action capsule.

A title, class hash, position or creation order is never target authority.
Identity includes the admitted session and native editor epoch; HWND/XID alone
cannot authorize an action. No code here launches or recovers a plug-in.
"""
from dataclasses import dataclass, asdict
import hashlib
import json
from pathlib import Path
import struct
import sys
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'uio1'))
from observe import Mapping

CAPACITY = 128
ROW = 192
SLOT = 32 + CAPACITY * ROW
SIZE = 4096 + 2 * SLOT

@dataclass(frozen=True)
class Editor:
    session: str
    native_view: int
    activation: int
    generation: int
    epoch: int
    pid: int
    start: int
    tid: int
    hwnd: int
    xid: int

    def validate(self):
        if (len(self.session) != 32 or any(c not in '0123456789abcdef' for c in self.session) or
            any(type(v) is not int or not 0 < v < 2**64 for k, v in asdict(self).items() if k != 'session')):
            raise RuntimeError('incomplete editor identity')

class WindowGraph(Mapping):
    def __init__(self, path, editor):
        editor.validate()
        super().__init__(path, SIZE)
        h = struct.unpack_from('<4sIII7Q', self.map)
        if h[:4] != (b'UIO2', 2, SIZE, ROW) or h[4:8] != (editor.pid, editor.tid, editor.start, editor.hwnd):
            self.close(); raise RuntimeError('window graph identity/schema mismatch')
        self.editor = editor

    def snapshot(self, seconds=.4):
        if not 0 < seconds <= 1: raise ValueError('graph request bound')
        began = time.monotonic_ns()
        request = self.read(48) + 1
        self.write(48, request)
        end = time.monotonic() + seconds
        while self.read(56) != request:
            if time.monotonic() >= end: raise RuntimeError('window graph unavailable')
            time.sleep(.002)
        for _ in range(3):
            n = self.read(64); at = 4096 + (n % 2) * SLOT
            if not n or self.read(at) != n: continue
            copy = bytes(self.map[at:at + SLOT])
            if self.read(at) != n or self.read(64) != n: continue
            commit, qpc, count, incomplete, _ = struct.unpack_from('<QQIIQ', copy)
            if commit != n or count > CAPACITY or incomplete: raise RuntimeError('window graph incomplete')
            rows = []
            for i in range(count):
                r = struct.unpack_from('<6Q8I12i8Q', copy, 32 + i * ROW)
                keys = ('hwnd', 'parent', 'owner', 'root', 'root_owner', 'xid', 'pid', 'tid',
                        'visible', 'enabled', 'minimized', 'dpi', 'style', 'exstyle')
                row = dict(zip(keys, r[:14]))
                row.update(rect=list(r[14:18]), client=list(r[18:22]), work=list(r[22:26]))
                row.update(zip(('class_atom','class_hash','focus','active','capture','previous','next','pointer_hwnd'),r[26:34]))
                rows.append(row)
            if len({r['hwnd'] for r in rows}) != len(rows): raise RuntimeError('duplicate window identity')
            return dict(editor=asdict(self.editor), windows=rows, qpc=qpc,
                        interval_ns=[began, time.monotonic_ns()], commit=n)
        raise RuntimeError('window graph changing; no complete snapshot')


def owned(row, rows, e):
    if (row['pid'] != e.pid or row['tid'] != e.tid or row['root_owner'] != e.hwnd or
        row['hwnd'] == e.hwnd or row['root'] != row['hwnd'] or not row['owner']):
        return False
    # Complete exact owner chain, not merely the root-owner scalar.
    seen = {row['hwnd']}; owner = row['owner']
    for _ in range(CAPACITY):
        if owner == e.hwnd: return True
        parent = rows.get(owner)
        if (not parent or owner in seen or parent['pid'] != e.pid or parent['tid'] != e.tid): return False
        seen.add(owner); owner = parent['owner']
    return False


def bind(snapshot, editor, current, previous=None):
    editor.validate()
    if editor != current or snapshot['editor'] != asdict(editor): raise RuntimeError('stale editor epoch/identity')
    rows = {r['hwnd']: r for r in snapshot['windows']}
    root = rows.get(editor.hwnd)
    if (not root or root['pid'] != editor.pid or root['tid'] != editor.tid or root['xid'] != editor.xid or
        not root['visible'] or not root['enabled'] or root['minimized']):
        raise RuntimeError('editor absent or inactive')
    candidates = [r for r in rows.values() if owned(r, rows, editor) and r['visible'] and r['enabled'] and not r['minimized']]
    if len(candidates) != 1: raise RuntimeError('owned popup absent or ambiguous')
    r = candidates[0]
    if not r['xid'] or not r['dpi'] or r['rect'][2] <= r['rect'][0] or r['rect'][3] <= r['rect'][1]:
        raise RuntimeError('popup geometry/binding unavailable')
    if previous is not None and any(r.get(k)!=v for k,v in previous.items() if k not in ('focus','active','capture','pointer_hwnd','previous','next')): raise RuntimeError('popup changed or disappeared during selection')
    return dict(r)


def inside(rect, point):
    return (len(point) == 2 and all(type(v) is int for v in point) and
            rect[0] <= point[0] < rect[2] and rect[1] <= point[1] < rect[3])


# The GUI executor can choose only the one pre-admitted action. It cannot send
# coordinates, scripts, keys, retries, verdicts, or alter the capsule.
@dataclass(frozen=True)
class Capsule:
    test_id: str
    editor: Editor
    target: dict
    action: str
    point: tuple
    issued_ns: int
    expires_ns: int
    maximum_actions: int = 1
    executor: str = 'luna-max'
    escalation_reason: str = ''
    forbidden: tuple = ('retry', 'relaunch', 'terminal', 'settings', 'project_change', 'verdict', 'unexpected_dialog')
    stop: tuple = ('terminal_record', 'identity_change', 'visibility_loss', 'held_input', 'unexpected_popup')

    def validate(self, current, now):
        self.editor.validate()
        if current != self.editor: raise RuntimeError('stale capsule editor')
        if (self.action not in ('open_menu', 'resize_window', 'resize_choice', 'known_control', 'dismiss_transient_group') or
            type(self.maximum_actions) is not int or self.maximum_actions != 1 or self.executor not in ('luna-max', 'astra') or
            type(self.issued_ns) is not int or type(self.expires_ns) is not int or
            not isinstance(self.test_id,str) or not 1<=len(self.test_id)<=64 or not all(c.isascii() and (c.isalnum() or c in '_-') for c in self.test_id) or
            self.forbidden != Capsule.__dataclass_fields__['forbidden'].default or
            self.stop != Capsule.__dataclass_fields__['stop'].default or
            not 0 <= now-self.issued_ns <= 120_000_000_000 or not self.issued_ns < self.expires_ns <= self.issued_ns+120_000_000_000 or
            now >= self.expires_ns or not inside(self.target['rect'], self.point)):
            raise RuntimeError('capsule authority/extent/expiry')
        if (type(self.escalation_reason) is not str or (self.executor == 'luna-max' and self.escalation_reason) or
            (self.executor == 'astra' and (not 1 <= len(self.escalation_reason.strip()) <= 256 or any(ord(c)<32 for c in self.escalation_reason)))):
            raise RuntimeError('custodian escalation reason required')
        if self.target['pid'] != current.pid or self.target['tid'] != current.tid:
            raise RuntimeError('foreign capsule target')
        return hashlib.sha256(json.dumps(asdict(self), sort_keys=True).encode()).hexdigest()

class Permit:
    def __init__(self, capsule): self.capsule = capsule; self.used = False
    def consume(self, command, current, now):
        if self.used: raise RuntimeError('capsule already consumed; no retries')
        self.used = True  # even a refused attempt cannot silently retry
        if command != {'action': self.capsule.action}: raise RuntimeError('arbitrary executor command refused')
        return self.capsule.validate(current, now)
