"""AP1-only Proton 11.0-2c binding explicitly authorized by the operator.

The historical WR0/PC0/AP0 baseline is unchanged. These exact measured records
replace only this AP1 selection; the shared verifier still rejects every drift.
"""
import copy
import pc0_diagnostic_runtime as runtime

RUNNER = {'build_id': '25118279',
 'depot_manifest': '2978628887517351791',
 'directory_name': 'Proton 11.0',
 'required_tool_app_id': '4183110',
 'steam_app_id': '4628710',
 'version': '1788504981 proton-11.0-2c-x86_64'}
FILES = {'runner/files/lib/wine/x86_64-windows/cmd.exe': {'mode': '0555',
                                                  'safe_path': 'runner/files/lib/wine/x86_64-windows/cmd.exe',
                                                  'sha256': '69b0eb75577a430db0efc30b6aa10b6891cbe58cbe40aa54f59645570b7adfa0',
                                                  'size': 1210529,
                                                  'type': 'regular_file'},
 'runner/files/share/default_pfx/system.reg': {'mode': '0755',
                                               'safe_path': 'runner/files/share/default_pfx/system.reg',
                                               'sha256': '9517868f66bb33c72a49a6e40c082aae4990ecfd0cac73ca40d7f42186f23ced',
                                               'size': 3874913,
                                               'type': 'regular_file'},
 'runner/files/share/default_pfx/user.reg': {'mode': '0755',
                                             'safe_path': 'runner/files/share/default_pfx/user.reg',
                                             'sha256': '71697ce829ea0394d52c6dbcf5f452b1bf3ef7d4942465206ae36e2f8f2ff681',
                                             'size': 27885,
                                             'type': 'regular_file'},
 'runner/files/share/default_pfx/userdef.reg': {'mode': '0755',
                                                'safe_path': 'runner/files/share/default_pfx/userdef.reg',
                                                'sha256': '5335cec8099762150f1d247d6a780030e7c4343eb7f75d9422d23ecf4a519da0',
                                                'size': 4190,
                                                'type': 'regular_file'},
 'runner/files/steampipe_fixups_mtime': {'mode': '0644',
                                         'safe_path': 'runner/files/steampipe_fixups_mtime',
                                         'sha256': '153ef8ca2f0e5bcd56f3ce3bafec0af7327a115125d022f160b0f008e1f14eb9',
                                         'size': 19,
                                         'type': 'regular_file'},
 'runner/proton': {'mode': '0755',
                   'safe_path': 'runner/proton',
                   'sha256': '787504a79bacf6b303984a9a846cf47463f36599248e8306b26d5faf78267aad',
                   'size': 93135,
                   'type': 'regular_file'},
 'runner/steampipe_fixups.json': {'mode': '0755',
                                  'safe_path': 'runner/steampipe_fixups.json',
                                  'sha256': 'c13c7c7e4ac94d30d976747edffed32099ad4b87f3efea09a7e79d401193011a',
                                  'size': 96309,
                                  'type': 'regular_file'},
 'runner/version': {'mode': '0755',
                    'safe_path': 'runner/version',
                    'sha256': '85597f4c274a7c4a6815805d65265b60e6d393a8bec858ffb032a162c40ec5e4',
                    'size': 33,
                    'type': 'regular_file'},
 'steamapps/appmanifest_4628710.acf': {'mode': '0755',
                                       'safe_path': 'steamapps/appmanifest_4628710.acf',
                                       'sha256': '27cb5724f8d7cb050b38d8b5bbf90b53614c7fea2b0e567e285525a8408dc25e',
                                       'size': 700,
                                       'type': 'regular_file'}}
APPS = {'4183110': ('24599767', 'SteamLinuxRuntime_4', '4183111', '78117001432799844', '672162947'),
 '4628710': ('25118279', 'Proton 11.0', '4628711', '2978628887517351791', '1445063149')}

def baseline():
    value = copy.deepcopy(runtime.lock_api()['expected_lock_manifest']())
    value['runner'] = copy.deepcopy(RUNNER)
    value['files'] = [copy.deepcopy(FILES.get(r['safe_path'], r)) for r in value['files']]
    return value

def identity():
    return runtime.digest(baseline())

def declared_inputs():
    return runtime.declared_runtime_inputs(baseline=baseline(), applications=APPS)

def verify_runtime():
    return runtime.verify_diagnostic_runner(baseline=baseline(), applications=APPS, allow_completed_update=True)

def validate_observation(value):
    return runtime.validate_runtime_observation(value, baseline=baseline(), applications=APPS)
