"""Cross-session decision. Configuration intent alone cannot prove absence."""
from environment import validate
from report import summarize
from identity import canonical,validate_identity,hashed

def compare(proofs):
    if [p['session_mode'] for p in proofs]!=['baseline','unix_override','restored']:raise ValueError('session_order')
    for p in proofs:
        if p.get('schema')!=2:raise ValueError('unsealed_session_schema')
        validate_identity(p['identity'])
    sealed=canonical(proofs[0]['identity'])
    if any(canonical(p['identity'])!=sealed for p in proofs) or len({p['binding']['operation'] for p in proofs})!=3:raise ValueError('session_identity')
    for p in proofs:
        if p['outer_exit']!=0 or not p['cleanup_confirmed'] or p['owned_survivors'] or not p['scratch_prefix_removed'] or p['binding']['status']!='bound':raise ValueError('session_custody')
        identity=p['identity']
        if p['binding']['artifact_sha256']!=identity['payload']['sha256'] or p['binding'].get('size')!=identity['payload']['size']:raise ValueError('root_artifact')
        if p['payload_sha256']!=identity['payload']['sha256'] or p['runner_id']!=identity['runner']['id'] or p['staged_runtime_sha256']!=identity['sources']['session.py']:raise ValueError('legacy_alias_drift')
        if canonical(p['powershell_images'])!=canonical(identity['powershell_images']):raise ValueError('image_alias_drift')
        if p['installed_artifacts']!={k:v['sha256'] for k,v in identity['installed']['artifacts'].items() if k!='operator_frontend'}:raise ValueError('installed_alias_drift')
        if p['source_sha256']!={k:identity['sources'][k] for k in ('run.py','report.py','capability.cpp')}:raise ValueError('source_alias_drift')
        validate(p['windows_environment_lines'])
        p['comparison']=summarize(p['oracle_lines'])
        u=p['unix_environment']
        if u['operation']!=p['binding']['operation'] or u['fixture_sha256']!=p['payload_sha256'] or u['phase']!='target_runner_after_prefix_initialization' or u['mode']!=p['session_mode']:raise ValueError('unix_identity')
    for p in (proofs[0],proofs[2]):
        if p['unix_environment']['present']:raise ValueError('baseline_not_normal')
        delivery=validate(p['windows_environment_lines'])
        expected=p['identity']['baseline_windows_environment']
        for mode in ('baseline','restored'):
            for origin in ('expected','child'):
                if canonical(delivery['rows'][mode][origin])!=canonical(expected):raise ValueError('outer_windows_baseline_drift')
        if any(m['script_behavior']!='false_success' or m['source_owned_fallback_executed'] for m in p['comparison']['modes'].values()):raise ValueError('baseline_behavior_drift')
    mid=proofs[1];u=mid['unix_environment']
    if not u['present'] or u['setting']!='powershell.exe=' or u['sha256_utf8']!='a156d13bd0ad6129fa13a0a7b76672634c09956c9b090e7c89edf60078979c63' or u['value_bytes']!=15:raise ValueError('unix_request')
    absence=all(all(not m[k]['launched'] and not m[k]['exited'] and m[k]['status']==126 and not m[k]['side_effect'] for k in ('side_effect','capability','empty_query')) and m['source_owned_fallback_executed'] for m in mid['comparison']['modes'].values())
    return {'schema':2,'comparison_identity_sha256':hashed(proofs[0]['identity']),'disposition':'IS3_OPERATION_SCOPED_HONEST_ABSENCE_PROVED' if absence else 'IS3_UNIX_UNAVAILABILITY_NOT_ESTABLISHED',
        'windows_environment_delivery':'verified','unix_behavior':'CreateProcessW_refused_126' if absence else 'not_selected',
        'source_owned_fallback':'launched_exit_43' if absence else 'not_established',
        'vendor_fallback':'not_tested','genuine_interpreter':'deferred_requirement_not_established',
        'registry_comparison':'unnecessary' if absence else 'permitted_separate_fresh_prefix',
        'compatibility_requirement':'sufficient_for_source_owned_fallback_contract_only',
        'native_access_historical_cause':'not_formally_proved'}
