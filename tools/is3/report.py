"""Bounded source-owned oracle. Never upgrades a vendor installation claim."""
import re

PATTERN = re.compile(r'IS3_CAP_V1 mode=(baseline|unavailable|restored) stage=(side_effect|capability|empty_query|fallback) launched=([01]) exited=([01]) status=([0-9]+) side_effect=([01]) tick=([0-9]+)')
MODES = ('baseline', 'unavailable', 'restored')
STAGES = ('side_effect', 'capability', 'empty_query', 'fallback')

def summarize(lines):
    if len(lines) != 12 or any(len(line) > 256 for line in lines):
        raise ValueError('incomplete_or_unbounded_oracle')
    result = {}
    last = 0
    for line, expected in zip(lines, ((m, s) for m in MODES for s in STAGES)):
        match = PATTERN.fullmatch(line)
        if not match or match.group(1, 2) != expected:
            raise ValueError('oracle_order_or_identity')
        mode, stage, launched, exited, status, effect, tick = match.groups()
        row = dict(launched=launched == '1', exited=exited == '1', status=int(status), side_effect=effect == '1')
        if int(status) > 0xffffffff or int(tick) < last or (row['exited'] and not row['launched']):
            raise ValueError('malformed_oracle')
        last = int(tick)
        result.setdefault(mode, {})[stage] = row
    for mode, rows in result.items():
        side, cap, query = (rows[k] for k in STAGES[:3])
        if side['launched'] and side['exited'] and side['status'] == 37 and side['side_effect']:
            behavior = 'script_side_effect_and_exit_verified'
        elif side['launched'] and side['exited'] and side['status'] == 0 and not side['side_effect']:
            behavior = 'false_success'
        elif not side['launched'] or (side['exited'] and side['status'] != 0):
            behavior = 'launch_or_execution_refused'
        else:
            behavior = 'unavailable_observation'
        fallback_required = not cap['launched'] or (cap['exited'] and cap['status'] != 0)
        f = rows['fallback']
        if fallback_required != f['launched'] or (f['launched'] and not (f['exited'] and f['status'] == 43)):
            raise ValueError('fallback_contract_incomplete')
        rows['script_behavior'] = behavior
        rows['empty_query_correct'] = query['launched'] and query['exited'] and query['status'] == 1
        rows['source_owned_fallback_executed'] = f['launched']
    return {'schema': 1, 'authority': 'source_owned_consumer_only', 'modes': result,
            'vendor_fallback': 'not_observed', 'genuine_managed_interpreter': 'not_tested',
            'historical_native_access_cause': 'not_formally_proved'}
