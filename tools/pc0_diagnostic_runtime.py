"""PC0-D1 only: exact deployed inputs, separately observed Steam bookkeeping.

The historical WR0 verifier and its accepted snapshot remain untouched. This
verifier never restores Steam files or reports the old snapshot as observed.
"""
from __future__ import annotations
import copy
import hashlib
import json
import pathlib
import re
import runpy
import stat

BASELINE = '2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547'
SCHEMA = 'linux-vst-bridge-pc0-diagnostic-runtime/v1'
APPS = {
    '4628710': ('24867889', 'Proton 11.0', '4628711', '3114679013132291065', '1445063147'),
    '4183110': ('24599767', 'SteamLinuxRuntime_4', '4183111', '78117001432799844', '672162947'),
}
BOOKKEEPING = {'LastOwner', 'LastUpdated', 'LastPlayed', 'UpdateResult',
               'BytesToDownload', 'BytesDownloaded', 'BytesToStage', 'BytesStaged',
               'TargetBuildID', 'AutoUpdateBehavior', 'AllowOtherDownloadsWhileRunning',
               'ScheduledAutoUpdate', 'StagingSize', 'DownloadType'}

def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(',', ':'))+'\n').encode()

def digest(value):
    return hashlib.sha256(canonical(value)).hexdigest()

def parse_vdf(raw):
    if len(raw) > 16384:
        raise RuntimeError('Steam manifest exceeds diagnostic bound')
    text=raw.decode('utf-8', 'strict')
    token=re.compile(r'\s*(?:"((?:\\["\\]|[^"\\])*)"|([{}]))')
    items=[]; pos=0
    while pos<len(text):
        if not text[pos:].strip(): break
        match=token.match(text,pos)
        if not match: raise RuntimeError('Malformed Steam manifest')
        items.append((match.group(1),match.group(2))); pos=match.end()
    pos=0
    def object_(depth=0):
        nonlocal pos
        if depth>4: raise RuntimeError('Steam manifest nesting exceeds bound')
        result={}
        while pos<len(items):
            key,brace=items[pos];pos+=1
            if brace=='}':
                if not depth: raise RuntimeError('Unexpected Steam manifest closing brace')
                return result
            if key is None or key in result or pos>=len(items):
                raise RuntimeError('Duplicate or malformed Steam manifest key')
            value,brace=items[pos];pos+=1
            if brace=='{': value=object_(depth+1)
            elif value is None: raise RuntimeError('Malformed Steam manifest value')
            result[key]=value
        if depth: raise RuntimeError('Unclosed Steam manifest object')
        return result
    result=object_()
    if pos!=len(items) or set(result)!={'AppState'} or not isinstance(result['AppState'],dict):
        raise RuntimeError('Steam manifest root differs')
    return result['AppState']

def application(raw, appid):
    app=parse_vdf(raw)
    build,directory,depot,manifest,size=APPS[appid]
    known={'appid','universe','name','StateFlags','installdir','SizeOnDisk','buildid',
           'InstalledDepots','UserConfig','MountedConfig'} | BOOKKEEPING
    if set(app)-known: raise RuntimeError('Unknown Steam manifest field requires classification')
    expected={'appid':appid,'universe':'1','buildid':build,'installdir':directory,
              'SizeOnDisk':size,'InstalledDepots':{depot:{'manifest':manifest,'size':size}},
              'UserConfig':{},'MountedConfig':{}}
    if any(app.get(k)!=v for k,v in expected.items()):
        raise RuntimeError('Installed Steam application/build/depot/install/config differs')
    if app.get('name') != ('Proton 11.0' if appid=='4628710' else 'Steam Linux Runtime 4.0'):
        raise RuntimeError('Steam application name differs')
    # Installed or installed+update-available only; never downloading/staging.
    if app.get('StateFlags') not in {'4','6'}:
        raise RuntimeError('Steam installation is not quiescent')
    for k in BOOKKEEPING:
        if k in app and (not isinstance(app[k],str) or re.fullmatch(r'[0-9]{1,20}',app[k]) is None):
            raise RuntimeError('Steam bookkeeping value is not a bounded integer')
    for k in ('BytesDownloaded','BytesStaged','BytesToStage','StagingSize','UpdateResult'):
        if app.get(k,'0')!='0': raise RuntimeError('Steam update/staging activity requires resolution')
    return {'selection':expected, 'bookkeeping':{
        'state_flags':app['StateFlags'], 'target_build':app.get('TargetBuildID'),
        'scheduled_update':app.get('ScheduledAutoUpdate'),
        'pending_download_bytes':app.get('BytesToDownload','0')}}

def lock_api():
    path=pathlib.Path.cwd()/'tools/wr0-proton-bootstrap/launch.py'
    api=runpy.run_path(str(path),run_name='pc0_readonly_wr0_lock')
    if api['expected_lock_digest']()!=BASELINE:
        raise RuntimeError('Historical runtime contract differs')
    return api

def verify_diagnostic_runner():
    api=lock_api(); baseline=api['expected_lock_manifest']()
    observed=copy.deepcopy(baseline); apps={}
    for record,spec in zip(observed['files'],api['LOCK_FILES'],strict=True):
        base=api['lock_base'](spec.base)
        api['require_no_symlink_ancestors'](base,allow_absent_leaf=False,label='diagnostic runtime root')
        path=base/spec.relative
        api['require_contained'](path,base,label='diagnostic locked file')
        info=path.lstat()
        if not stat.S_ISREG(info.st_mode) or format(stat.S_IMODE(info.st_mode),'04o')!=spec.mode:
            raise RuntimeError('Diagnostic runtime file type/mode differs: '+record['safe_path'])
        if spec.base=='steamapps':
            if info.st_size>16384: raise RuntimeError('Steam manifest exceeds diagnostic bound')
            raw=path.read_bytes()
            appid=spec.relative.removeprefix('appmanifest_').removesuffix('.acf')
            apps[appid]=application(raw,appid)
            record['size']=len(raw);record['sha256']=hashlib.sha256(raw).hexdigest()
        elif info.st_size!=spec.size or api['sha256_file'](path)!=spec.sha256:
            raise RuntimeError('Deployed runtime input differs: '+record['safe_path'])
    selection={'runner':baseline['runner'],'runtime':baseline['runtime'],
               'files':[r for r in baseline['files'] if not r['safe_path'].startswith('steamapps/')],
               'applications':{k:v['selection'] for k,v in apps.items()}}
    result={'schema':SCHEMA,'baseline_contract_sha256':BASELINE,
            'launch_critical_manifest_sha256':digest(observed),
            'declared_inputs_sha256':digest(selection), 'observed_manifest':observed,
            'applications':apps}
    return result

def validate_runtime_observation(value):
    if not isinstance(value,dict) or set(value)!={'schema','baseline_contract_sha256',
            'launch_critical_manifest_sha256','declared_inputs_sha256','observed_manifest','applications'}:
        raise RuntimeError('Diagnostic runtime observation shape differs')
    baseline=lock_api()['expected_lock_manifest']()
    if value['schema']!=SCHEMA or value['baseline_contract_sha256']!=BASELINE:
        raise RuntimeError('Diagnostic runtime contract differs')
    manifest=value['observed_manifest']
    if set(manifest)!=set(baseline) or any(manifest[k]!=baseline[k] for k in ('schema','runner','runtime')):
        raise RuntimeError('Diagnostic runtime declarations differ')
    if len(manifest['files'])!=len(baseline['files']): raise RuntimeError('Runtime file roster differs')
    for actual,expected in zip(manifest['files'],baseline['files'],strict=True):
        if not expected['safe_path'].startswith('steamapps/'):
            if actual!=expected: raise RuntimeError('Deployed runtime record differs')
        elif (set(actual)!=set(expected) or any(actual[k]!=expected[k] for k in ('safe_path','type','mode'))
              or type(actual['size']) is not int or not 0<actual['size']<=16384
              or re.fullmatch('[0-9a-f]{64}',str(actual['sha256'])) is None):
            raise RuntimeError('Observed Steam metadata record differs')
    if digest(manifest)!=value['launch_critical_manifest_sha256']:
        raise RuntimeError('Observed runtime snapshot digest differs')
    if set(value['applications'])!=set(APPS): raise RuntimeError('Application roster differs')
    for appid,observed in value['applications'].items():
        build,directory,depot,depot_manifest,size=APPS[appid]
        expected={'appid':appid,'universe':'1','buildid':build,'installdir':directory,'SizeOnDisk':size,
                  'InstalledDepots':{depot:{'manifest':depot_manifest,'size':size}},'UserConfig':{},'MountedConfig':{}}
        if set(observed)!={'selection','bookkeeping'} or observed['selection']!=expected:
            raise RuntimeError('Observed application selection differs')
        b=observed['bookkeeping']
        if (set(b)!={'state_flags','target_build','scheduled_update','pending_download_bytes'}
            or b['state_flags'] not in {'4','6'} or any(v is not None and
                (not isinstance(v,str) or re.fullmatch('[0-9]{1,20}',v) is None) for v in b.values())):
            raise RuntimeError('Observed bookkeeping shape differs')
    selection={'runner':baseline['runner'],'runtime':baseline['runtime'],
               'files':[r for r in baseline['files'] if not r['safe_path'].startswith('steamapps/')],
               'applications':{k:v['selection'] for k,v in value['applications'].items()}}
    if digest(selection)!=value['declared_inputs_sha256']: raise RuntimeError('Declared runtime input digest differs')
    return value['launch_critical_manifest_sha256']


def sanitized_supervision_error(error):
    """Preserve a useful bounded exception without arguments or private state."""
    if error is None:
        return None
    text=type(error).__name__+": "+str(error)
    text=re.sub(r"Command .*", "Command <arguments omitted>", text)
    text=re.sub(r"(?i)(password|token|secret|cookie|authorization)[=:]\s*\S+", r"\1=<redacted>", text)
    text=re.sub(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\b", "<account>", text)
    text=re.sub(r"\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b", "<address>", text)
    text=re.sub(r"(?:[A-Za-z]:[\\/]|/)[^\s\"']+", "<path>", text)
    text=re.sub(r"(?i)\b(?:pid|ppid)[=: ]+[0-9]+", "<process>", text)
    text=re.sub(r"[\x00-\x1f\x7f]", " ", text)
    return text[:768]


# Independent of the product normalizer and its validators. Only these scalar
# protocol fields can enter a troubleshooting checkpoint; no process identities,
# command lines, environment, factory account metadata or paths are retained.
CHECKPOINT_KEYS = frozenset("""
rejected origin thread_role enclosing_attempt_sequence enclosing_operation reference_count output_null
event sequence attempt_sequence operation interface ordinal tier return_kind
state disposition object_quiescence component_state primary_blocker
result_u32_hex win32_error_u32_hex i32_result u32_result bool_result output_nonnull
host_reference_count object_role requested_interface media_type direction index
audio_index symbolic_size audio_interface_quiescence audio_processor_state
callback_ledger_unchanged release_reference_count pointer_cleared
result blocks comparison maximum_absolute_error samples_compared ap0_call_started ap0_call_completed block gain frames sample_rate process_mode sample_format process_result worker_thread owner_thread distinct_from_owner joined processing_stopped worker_exception input_silence_flags output_silence_flags input_bits output_bits
processing_contract schema lifecycle_state counts count buses name_utf8
channel_count bus_type flags_u32_hex default_active control_voltage
speaker_arrangement bits_u64_hex recognized_layout sample_sizes tresult_i32
tresult_u32_hex supported call_count complete mutation_call_count
raw_exit classification blocker secondary_cleanup_blocker last_lifecycle
last_in_flight_operation audio_processor_observer_state inherited_shutdown
operations clean_in_process_shutdown physical_containment_only cleanup
owned_descendants_zero process_group_empty stdout_sha256 stderr_sha256 stderr_bytes
records run_id component_session
terminate_component release_component release_factory_3 release_factory_2
release_factory_base exit_dll free_library source
""".split())


def checkpoint_projection(value, depth=0):
    if depth > 10:
        return "<depth bound>"
    if isinstance(value, dict):
        return {k: checkpoint_projection(v, depth+1) for k,v in value.items()
                if k in CHECKPOINT_KEYS}
    if isinstance(value, list):
        return [checkpoint_projection(v, depth+1) for v in value[-256:]]
    if value is None or type(value) in (bool, int):
        return value
    if type(value) is float and value == value and abs(value) < 1e6:
        return value
    if isinstance(value, str):
        # Protocol strings and AGain bus names, never arbitrary free-form text.
        if len(value) <= 128 and re.fullmatch(r"[A-Za-z0-9_ .:+-]*", value):
            return value
        return "<redacted string>"
    return "<unsupported value>"


def exception_detail(error):
    """Bounded chain, repository locations and sanitized messages; no locals."""
    chain=[]; seen=set()
    while error is not None and id(error) not in seen and len(chain)<4:
        seen.add(id(error)); frames=[]; tb=error.__traceback__
        while tb is not None:
            filename=tb.tb_frame.f_code.co_filename
            if '/tools/' in filename:
                module='tools/'+filename.split('/tools/',1)[1]
            elif filename.startswith(('<pc0_','<ap0_')):
                module='tools/'+filename[1:-1]+'.py'
            else:
                module='<external>'
            frames.append({'module':module, 'function':tb.tb_frame.f_code.co_name[:80],
                           'line':tb.tb_lineno})
            tb=tb.tb_next
        chain.append({'type':type(error).__name__, 'detail':sanitized_supervision_error(error),
                      'frames':frames[-6:]})
        error=error.__cause__ if error.__cause__ is not None else error.__context__
    return chain


def declared_runtime_inputs():
    """Compute the pinned selection/input identity, excluding Steam bookkeeping."""
    baseline = lock_api()['expected_lock_manifest']()
    applications = {}
    for appid, (build, directory, depot, manifest, size) in APPS.items():
        applications[appid] = {'appid':appid, 'universe':'1', 'buildid':build,
            'installdir':directory, 'SizeOnDisk':size,
            'InstalledDepots':{depot:{'manifest':manifest,'size':size}},
            'UserConfig':{}, 'MountedConfig':{}}
    return digest({'runner':baseline['runner'], 'runtime':baseline['runtime'],
        'files':[r for r in baseline['files'] if not r['safe_path'].startswith('steamapps/')],
        'applications':applications})
