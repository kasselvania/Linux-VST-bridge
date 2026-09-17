"""Reclassify immutable IS1/IS2 records. Never equate Linux and Windows PIDs."""
import collections
import decimal
import json
import re
from common import require, digest

ROLES = {'unknown', 'gpu', 'renderer', 'utility', 'network'}
# Anchored Chromium source-location AND exact diagnostic grammar. Arbitrary prose
# and accessibility messages never acquire these categories.
PATTERNS = (
    ('gpu_exit', r'gpu_process_host\.cc', r'GPU process exited unexpectedly: exit_code=(-?\d{1,10})\.?'),
    ('gpu_unusable', r'gpu_data_manager_impl_private\.cc', r"GPU process isn't usable\. Goodbye\."),
    ('sandbox_child', r'(?:gpu_process_host|child_process_launcher_helper|child_process_launcher_helper_win|sandbox_win)\.cc', r'(?:GPU process launch failed:|Failed to launch child:) error_code=39\.?'),
    ('network_crash', r'network_service_instance_impl\.cc', r'Network service crashed, restarting service\.'),
    ('renderer_gone', r'(?:web_contents|electron_api_web_contents)\.cc', r'render-process-gone: (?:crashed|oom|launch-failed)'),
    ('graphics_initialization', r'(?:gl_factory|gl_surface_egl|gl_display|angle_platform_impl)\.cc', r'.{0,180}(?:initialization failed|Failed to initialize|Initialization failed).{0,180}'),
)
CHROMIUM = re.compile(r'^\[([0-9]{1,10}):([0-9]{1,10}):([0-9]{4}/[0-9]{6}\.[0-9]{3,6}):(ERROR|FATAL):([a-z_]+\.cc)\([0-9]{1,6}\)\] (.{1,512})$')
WINE = re.compile(r'^(\d+\.\d+):([0-9a-fA-F]+):([0-9a-fA-F]+):(trace|warn|err|fixme):([a-z0-9_]+):([A-Za-z0-9_]+) (.*)$')


def integer(x, low=0, high=2**32-1):
    require(type(x) is int and low <= x <= high, 'process_scalar')
    return x


def timestamp(x):
    require(isinstance(x, str) and re.fullmatch(r'[0-9]{1,12}\.[0-9]{1,9}', x), 'process_timestamp')
    return decimal.Decimal(x)


def generations(transaction, operation, root_binding):
    require(transaction.get('schema') == 1 and transaction.get('operation') == operation, 'transaction_binding')
    trace = transaction['windows_trace']; rows = trace['processes']
    require(isinstance(rows, list) and len(rows) <= 512, 'process_count')
    require(trace['launch_binding']['status'] == 'bound', 'unbound_root')
    for k in ('operation', 'epoch', 'token_sha256', 'artifact_sha256', 'size', 'root_ordinal'):
        require(trace['launch_binding'][k] == root_binding[k], 'trace_root_mismatch')
    byid = {}; previous_epoch = 0; bypid = collections.defaultdict(list)
    for index, r in enumerate(rows, 1):
        epoch = integer(r['epoch'], 1, 2); ordinal = integer(r['creation_ordinal'], 1, 512)
        require(ordinal == index and epoch >= previous_epoch, 'reordered_or_duplicate_generation')
        previous_epoch = epoch
        pid = integer(r['windows_pid'], 1)
        require(type(r['target_tree']) is bool and type(r['target_root']) is bool, 'process_boolean')
        start = None if r['created_timestamp'] is None else timestamp(r['created_timestamp'])
        end = None
        if r.get('self_exit') is not None:
            e = r['self_exit']
            require(e['domain'] == 'wine_self_exit_observation' and e['source'] == 'Wine NtTerminateProcess self pseudo-handle'
                    and type(e['process_exiting']) is int and e['process_exiting'] == 1, 'exit_authority')
            integer(e['status']); end = timestamp(e['timestamp'])
            require(start is None or end >= start, 'reversed_exit')
        require(not r.get('exit_conflict'), 'conflicting_exit')
        if r['target_root']:
            require(ordinal == root_binding['root_ordinal'] and epoch == 2 and r['target_tree']
                    and r.get('root_authority') == 'verified_launch_adapter_exact_handle_image_and_creation_time'
                    and r['image_identity']['sha256'] == root_binding['artifact_sha256']
                    and r['image_identity']['size'] == root_binding['size'], 'root_authority')
        elif r['target_tree']:
            parent = byid.get(r['parent_ordinal'])
            require(parent is not None and parent['epoch'] == epoch and parent['target_tree'] and start is not None, 'parent_generation')
            ps = parent['created_timestamp']; pe = parent.get('self_exit')
            require((ps is None or timestamp(ps) <= start) and (pe is None or timestamp(pe['timestamp']) >= start), 'retired_parent')
        for old in bypid[epoch, pid]:
            require(old.get('self_exit') is not None and start is not None
                    and timestamp(old['self_exit']['timestamp']) < start, 'overlapping_pid_generations')
        bypid[epoch, pid].append(r); byid[ordinal] = r
    require(sum(r['target_root'] for r in rows) == 1, 'root_count')
    return rows


def normalize_windows(path):
    if not isinstance(path, str) or '\0' in path or '..' in path.replace('\\', '/').split('/'):
        return None
    return path.replace('/', '\\').casefold()


def role(body):
    # Only the command-line field of an observed create REQUEST. This does not
    # make arbitrary text a create completion or mapped image identity.
    m = re.search(r' cmdline L"((?:[^"\\]|\\.)*)", inherit ', body)
    if not m:
        return 'unknown'
    try:
        command = json.loads('"'+m[1]+'"')
    except ValueError:
        return 'unknown'
    values = re.findall(r'(?:^|\s)--type=([a-z-]+)(?=\s|$)', command)
    if len(values) != 1:
        return 'unknown'
    result = {'gpu-process': 'gpu', 'renderer': 'renderer', 'utility': 'utility'}.get(values[0], 'unknown')
    sub = re.findall(r'(?:^|\s)--utility-sub-type=([^\s]+)', command)
    if result == 'utility' and sub == ['network.mojom.NetworkService']:
        result = 'network'
    return result


def parse(transaction, result, logs, identity, windows_image):
    op = result['operation']; binding = result['transaction']['launch_binding']
    rows = generations(transaction, op, binding)
    require(isinstance(logs, dict) and len(logs) <= 34 and sum(len(x) for x in logs.values()) <= 2*1024*1024, 'logs_bound')
    exe = identity['files']['Native Access.exe']['sha256']
    candidates = [r for r in rows if r['epoch'] == 2 and r['target_tree'] and
                  normalize_windows(r.get('image_request')) == normalize_windows(windows_image)]
    # Retained digest at request is weaker than mapped-image custody but sufficient
    # for this explicitly labelled request-generation diagnostic association.
    exact = {r['creation_ordinal']: r for r in candidates if r.get('image_identity') is not None
             and r['image_identity'].get('sha256') == exe
             and r['image_identity'].get('authority') in ('same_open_file_at_launch_request_not_mapped', 'mapped_device_inode_same_open_digest')}
    facts = []; seen = set(); pending = {}; roles = {}; ambiguous_requests = set(); ignored = 0
    unbound = collections.Counter()
    for logid, data in sorted(logs.items()):
        require(re.fullmatch('[0-9a-f]{64}', logid) and isinstance(data, bytes), 'private_log_identity')
        pending.clear()  # No pairing between independent files or truncated tails.
        lines = data.splitlines(keepends=True)
        for raw in lines:
            if not raw.endswith(b'\n') or len(raw) > 4096:
                ignored += 1; continue
            line = raw.decode('utf-8', errors='replace').rstrip('\r\n')
            match = WINE.fullmatch(line)
            if match:
                ts, pid, tid, level, channel, function, body = match.groups()
                key = (int(pid, 16), int(tid, 16))
                if channel == 'process' and function == 'CreateProcessInternalW' and body.startswith('app '):
                    if key in pending:
                        ambiguous_requests.add(key); pending.pop(key)
                    elif key not in ambiguous_requests:
                        pending[key] = (role(body), digest(raw))
                elif channel == 'process' and function == 'CreateProcessInternalW':
                    child = re.fullmatch(r'started process pid ([0-9a-fA-F]+) tid ([0-9a-fA-F]+)', body)
                    if child:
                        req = pending.pop(key, None)
                        matches = [r for r in exact.values() if r['windows_pid'] == int(child[1], 16)
                                   and r['creator_windows_pid'] == key[0] and r['creator_windows_tid'] == key[1]
                                   and r['created_timestamp'] == ts]
                        if req and len(matches) == 1 and req[0] != 'unknown':
                            oid = matches[0]['creation_ordinal']
                            require(oid not in roles or roles[oid] == req, 'role_conflict')
                            roles[oid] = req
                # A Wine GDI diagnostic supplies its own Windows PID domain.
                if channel == 'gdi' and level == 'err' and re.fullmatch(r'out of GDI object handles(?:[.!])?', body, re.I):
                    matches = [r for r in exact.values() if r['windows_pid'] == key[0]
                               and r['created_timestamp'] is not None and timestamp(r['created_timestamp']) <= timestamp(ts)
                               and (r.get('self_exit') is None or timestamp(r['self_exit']['timestamp']) >= timestamp(ts))]
                    if len(matches) == 1:
                        facts.append(fact(op, matches[0], 'gdi_exhaustion', digest(raw), logid, 'wine_windows_generation'))
            c = CHROMIUM.fullmatch(line)
            if not c:
                continue
            pid, _, _, severity, source, body = c.groups()
            category = None
            for name, filename, message in PATTERNS:
                if re.fullmatch(filename, source) and re.fullmatch(message, body):
                    category = name; break
            if category is None:
                continue
            # Chromium headers alone are NOT proof of a Windows/Linux generation.
            # Keep canonical operation-pipe diagnostics separately; only one exact
            # candidate request generation, no PID reuse in either epoch, permits
            # the Windows diagnostic association. Linux rows are never consulted.
            matches = [r for r in exact.values() if r['windows_pid'] == int(pid)]
            if len(matches) != 1 or sum(r['windows_pid'] == int(pid) for r in rows) != 1:
                unbound[category] += 1
                ignored += 1; continue
            if category == 'gpu_exit' and not (-(2**31) <= int(re.search(r'exit_code=(-?\d+)', body)[1]) < 2**32 and int(re.search(r'exit_code=(-?\d+)', body)[1]) != 0):
                continue
            k = (logid, digest(raw))
            if k in seen:
                continue
            seen.add(k)
            facts.append(fact(op, matches[0], category, digest(raw), logid, 'chromium_windows_unique_request_generation'))
    processes = []
    for r in candidates:
        ordinal = r['creation_ordinal']; rr, request_hash = roles.get(ordinal, ('unknown', None))
        exit_fact = r.get('self_exit')
        processes.append({'domain': 'windows', 'epoch': r['epoch'], 'ordinal': ordinal,
                          'image_digest_at_request': (r.get('image_identity') or {}).get('sha256'),
                          'role': rr, 'role_request_sha256': request_hash,
                          'exit_domain': exit_fact['domain'] if exit_fact else None,
                          'exit_status': exit_fact['status'] if exit_fact else None})
        if ordinal in exact and rr in ('gpu', 'renderer') and exit_fact and exit_fact['status'] != 0:
            facts.append(fact(op, r, rr+'_abnormal_exit', digest(json.dumps(exit_fact, sort_keys=True).encode()),
                              digest(json.dumps(transaction, sort_keys=True).encode()), 'wine_windows_generation'))
    return {'schema': 1, 'operation': op, 'application_sha256': exe,
            'root_bound': True, 'processes': processes, 'facts': facts,
            'unattributed_or_incomplete_lines': ignored, 'unbound_diagnostic_categories': dict(unbound),
            'runner_loss': transaction['diagnostics']['runner'],
            'windows_dropped_observations': transaction['windows_trace']['dropped_observations'],
            'linux_windows_join_performed': False}


def fact(op, row, category, record_hash, source_hash, authority):
    return {'operation': op, 'domain': 'windows', 'epoch': row['epoch'], 'ordinal': row['creation_ordinal'],
            'category': category, 'record_sha256': record_hash, 'source_sha256': source_hash,
            'authority': authority, 'image_sha256': row['image_identity']['sha256']}
