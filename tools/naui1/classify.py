"""Closed disposition owner. Missing observations never supply negative evidence."""
import re
from common import require

PREFIX = 'NAUI1_'
CATEGORIES = {'gpu_exit', 'gpu_unusable', 'gpu_abnormal_exit', 'sandbox_child', 'gdi_exhaustion',
              'renderer_gone', 'renderer_abnormal_exit', 'network_crash', 'graphics_initialization'}
FACT_KEYS = {'operation', 'domain', 'epoch', 'ordinal', 'category', 'record_sha256',
             'source_sha256', 'authority', 'image_sha256'}


def classify(identity, evidence):
    if identity['application_identity'] != 'established':
        return PREFIX+'APPLICATION_IDENTITY_UNRESOLVED'
    if identity['renderer_family'] != 'electron_chromium':
        return PREFIX+'OTHER_RENDERER_CAUSE_UNRESOLVED'
    require(set(evidence) == {'schema', 'operation', 'application_sha256', 'root_bound', 'processes', 'facts',
            'unattributed_or_incomplete_lines', 'unbound_diagnostic_categories', 'runner_loss', 'windows_dropped_observations', 'linux_windows_join_performed'}, 'evidence_schema')
    require(evidence['schema'] == 1 and evidence['root_bound'] is True and evidence['linux_windows_join_performed'] is False, 'evidence_authority')
    require(evidence['application_sha256'] == identity['files']['Native Access.exe']['sha256'], 'evidence_application')
    categories = set()
    require(len(evidence['facts']) <= 1024, 'fact_bound')
    for f in evidence['facts']:
        require(set(f) == FACT_KEYS, 'fact_privacy_or_schema')
        require(f['operation'] == evidence['operation'] and f['domain'] == 'windows' and type(f['epoch']) is int and f['epoch'] == 2
                and type(f['ordinal']) is int and 1 <= f['ordinal'] <= 512, 'fact_binding')
        require(f['image_sha256'] == evidence['application_sha256'] and f['category'] in CATEGORIES
                and f['authority'] in ('wine_windows_generation', 'chromium_windows_unique_request_generation'), 'fact_authority')
        require(all(isinstance(f[k], str) and re.fullmatch('[0-9a-f]{64}', f[k]) for k in ('record_sha256', 'source_sha256')), 'fact_digest')
        require(any(p['domain'] == 'windows' and p['epoch'] == f['epoch'] and p['ordinal'] == f['ordinal']
                    and p['image_digest_at_request'] == f['image_sha256'] for p in evidence['processes']), 'fact_generation')
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
