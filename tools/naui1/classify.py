"""Closed disposition owner. Missing observations never supply negative evidence."""
import re
from common import require
from records import integer, ROLES

PREFIX = 'NAUI1_'
CATEGORIES = {'gpu_exit', 'gpu_unusable', 'gpu_abnormal_exit', 'sandbox_child', 'gdi_exhaustion',
              'renderer_gone', 'renderer_abnormal_exit', 'network_crash', 'graphics_initialization'}
FACT_KEYS = {'operation', 'domain', 'epoch', 'ordinal', 'category', 'record_sha256',
             'source_sha256', 'authority', 'image_sha256'}

PROCESS_KEYS = {'domain', 'epoch', 'ordinal', 'image_digest_at_request', 'role',
                'role_request_sha256', 'exit_domain', 'exit_status'}
ROLE_EXITS = {'gpu_abnormal_exit': 'gpu', 'renderer_abnormal_exit': 'renderer'}


def sha256(value):
    return isinstance(value, str) and re.fullmatch('[0-9a-f]{64}', value) is not None


def process_generations(processes):
    """Closed normalized history; gaps in ordinals are allowed, reuse is not."""
    require(isinstance(processes, list) and len(processes) <= 512, 'process_count')
    by_generation = {}
    previous = 0
    for p in processes:
        require(isinstance(p, dict) and set(p) == PROCESS_KEYS, 'process_privacy_or_schema')
        require(p['domain'] == 'windows' and type(p['epoch']) is int and p['epoch'] == 2, 'process_domain')
        ordinal = integer(p['ordinal'], 1, 512)
        require(ordinal > previous, 'reordered_or_duplicate_generation')
        previous = ordinal
        require(p['image_digest_at_request'] is None or sha256(p['image_digest_at_request']), 'process_image')
        require(isinstance(p['role'], str) and p['role'] in ROLES, 'process_role')
        require((p['role'] == 'unknown' and p['role_request_sha256'] is None)
                or (p['role'] != 'unknown' and sha256(p['role_request_sha256'])), 'process_role_request')
        if p['exit_domain'] is None:
            require(p['exit_status'] is None, 'process_exit_pair')
        else:
            require(p['exit_domain'] == 'wine_self_exit_observation', 'process_exit_domain')
            integer(p['exit_status'])
        by_generation[p['epoch'], ordinal] = p
    return by_generation


def classify(identity, evidence):
    if identity['application_identity'] != 'established':
        return PREFIX+'APPLICATION_IDENTITY_UNRESOLVED'
    if identity['renderer_family'] != 'electron_chromium':
        return PREFIX+'OTHER_RENDERER_CAUSE_UNRESOLVED'
    require(set(evidence) == {'schema', 'operation', 'application_sha256', 'root_bound', 'processes', 'facts',
            'unattributed_or_incomplete_lines', 'unbound_diagnostic_categories', 'runner_loss', 'windows_dropped_observations', 'linux_windows_join_performed'}, 'evidence_schema')
    require(evidence['schema'] == 1 and evidence['root_bound'] is True and evidence['linux_windows_join_performed'] is False, 'evidence_authority')
    require(evidence['application_sha256'] == identity['files']['Native Access.exe']['sha256'], 'evidence_application')
    drops = integer(evidence['windows_dropped_observations'])
    processes = process_generations(evidence['processes'])
    categories = set()
    require(isinstance(evidence['facts'], list) and len(evidence['facts']) <= 1024, 'fact_bound')
    for f in evidence['facts']:
        require(isinstance(f, dict) and set(f) == FACT_KEYS, 'fact_privacy_or_schema')
        require(f['operation'] == evidence['operation'] and f['domain'] == 'windows' and type(f['epoch']) is int and f['epoch'] == 2
                and type(f['ordinal']) is int and 1 <= f['ordinal'] <= 512, 'fact_binding')
        require(f['image_sha256'] == evidence['application_sha256'] and isinstance(f['category'], str)
                and f['category'] in CATEGORIES, 'fact_authority')
        authority = ('wine_exact_role_and_self_exit_generation' if f['category'] in ROLE_EXITS else
                     'wine_pid_lifetime_complete_trace' if f['category'] == 'gdi_exhaustion' else
                     'chromium_pid_unique_complete_trace')
        require(f['authority'] == authority, 'fact_authority')
        require(all(isinstance(f[k], str) and re.fullmatch('[0-9a-f]{64}', f[k]) for k in ('record_sha256', 'source_sha256')), 'fact_digest')
        p = processes.get((f['epoch'], f['ordinal']))
        require(p is not None and p['image_digest_at_request'] == f['image_sha256'], 'fact_generation')
        if f['category'] in ROLE_EXITS:
            require(p['role'] == ROLE_EXITS[f['category']] and sha256(p['role_request_sha256'])
                    and p['exit_domain'] == 'wine_self_exit_observation'
                    and p['exit_status'] != 0, 'fact_role_exit')
        # Independently enforce completeness even on evidence bypassing parse().
        # Runner-tail loss is separate and does not invalidate exact positive
        # authority. All current routes, including retained self-exit mapping,
        # depend on the Windows trace's generation census being complete.
        if drops == 0:
            categories.add(f['category'])
    specific = []
    if categories & {'gpu_exit', 'gpu_unusable', 'gpu_abnormal_exit'}:
        specific.append('ELECTRON_GPU_FAILURE_SELECTED')
    if 'sandbox_child' in categories:
        specific.append('ELECTRON_SANDBOX_CHILD_FAILURE_SELECTED')
    if 'gdi_exhaustion' in categories:
        specific.append('FONT_GDI_FAILURE_SELECTED')
    # Multiple specific failure families need a comparison, not arbitrary priority.
    if len(specific) > 1:
        return PREFIX+'ELECTRON_IDENTITY_ONLY_CAUSE_UNRESOLVED'
    if specific:
        return PREFIX+specific[0]
    if categories & {'renderer_gone', 'renderer_abnormal_exit'}:
        return PREFIX+'RENDERER_PROCESS_FAILURE_SELECTED'
    return PREFIX+'ELECTRON_IDENTITY_ONLY_CAUSE_UNRESOLVED'
