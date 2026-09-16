"""Check the retained, source-owned two-ended lifecycle fixture; no UI authority."""


def check(case):
    if case['case'] not in ('normal', 'activation_only', 'wrong_sequence'):
        raise ValueError('unknown fixture case')
    if case['exit'] != 0 or case['cohort_empty'] is not True:
        raise ValueError('fixture exit or cleanup incomplete')
    rows = case['rows']
    received = [r for r in rows if r.get('state') == 'lc1_receive']
    summaries = [r for r in rows if r.get('state') == 'ap3_processing_summary']
    closed = [r for r in rows if r.get('state') == 'ap1_endpoint_closed']
    if any(r.get('session_match') is not True for r in received):
        raise ValueError('session mismatch')
    wrong = [r for r in received if r['expected_kind'] != r['actual_kind'] or r['expected_sequence'] != r['actual_sequence']]
    starts = [r for r in received if r['actual_kind'] == 10]
    if [(r['actual_sequence'], r['expected_epoch'], r['actual_epoch']) for r in starts] != [(104684, 1, 1), (104687, 2, 2)]:
        raise ValueError('restart sequence or epoch differs')
    if case['case'] == 'wrong_sequence':
        if len(wrong) != 1 or (wrong[0]['expected_kind'], wrong[0]['actual_kind'], wrong[0]['expected_sequence'], wrong[0]['actual_sequence']) != (14, 14, 104688, 104689) or closed:
            raise ValueError('malformed lifecycle refusal not established')
        return 'exact_wrong_sequence_refused'
    if wrong or len(closed) != 1 or closed[0].get('mapping_unmapped') is not True:
        raise ValueError('correlated normal closure missing')
    blocks = [r['processed_blocks'] for r in summaries]
    if blocks != ([3, 0, 1] if case['case'] == 'activation_only' else [3, 1]):
        raise ValueError('processing intervals differ')
    if case['case'] == 'activation_only':
        intermediate = [r for r in received if r['actual_kind'] == 14 and r['actual_sequence'] == 104687]
        if len(intermediate) != 2 or summaries[1]['intervals'] != 0:
            raise ValueError('activation-only interval not established')
    return 'exact_restart_and_close_passed'
