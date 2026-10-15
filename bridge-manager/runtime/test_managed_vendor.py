import copy, hashlib, json, os, pathlib, signal, sys, tempfile, unittest
from unittest.mock import patch
import session as s
import ownership


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()


class ManagedVendorTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.home = pathlib.Path(self.temp.name).resolve()
        self.managed = self.home / '.local/share/linux-vst-bridge/managed'
        self.profile = json.loads((pathlib.Path(__file__).parents[1] / 'src/vendor_profiles/native_access.json').read_text())
        self.home_patch = patch.object(pathlib.Path, 'home', return_value=self.home)
        self.home_patch.start()

    def tearDown(self):
        self.home_patch.stop()
        self.temp.cleanup()

    def write(self, path, value):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(canonical(value))

    def artifact(self, path):
        return {'path': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}

    def image(self, path):
        return {'artifact': self.artifact(path), 'size': path.stat().st_size}

    def operation(self, seed, existing_daemon=False):
        root = self.managed / 'environments' / (seed * 32)
        appdir = root / 'compatdata/pfx/drive_c/Program Files/Chosen App'
        appdir.mkdir(parents=True)
        pe = bytearray(1024)
        pe[:2] = b'MZ'
        pe[60:64] = (128).to_bytes(4, 'little')
        pe[128:132] = b'PE\0\0'
        pe[132:134] = (0x8664).to_bytes(2, 'little')
        pe[150] = 0x22
        pe[-1] = ord(seed)
        program = appdir / 'Native Access.exe'
        program.write_bytes(pe)
        files = {self.profile['executable']: self.image(program)}
        for name in self.profile['resources']:
            path = appdir / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(('resource-' + seed).encode())
            files[name] = self.image(path)
        bundled = appdir / self.profile['dependency']['installer_directory']
        bundled.mkdir(parents=True)
        installer = bundled / ('NTKDaemon unknown-version-' + seed + ' Setup PC.exe')
        installer.write_bytes(pe)
        daemon = root / 'compatdata/pfx/drive_c' / self.profile['dependency']['daemon']
        if existing_daemon:
            daemon.parent.mkdir(parents=True, exist_ok=True)
            daemon.write_bytes(pe)
        imported = self.managed / 'installers' / (hashlib.sha256(pe).hexdigest() + '.exe')
        imported.parent.mkdir(parents=True, exist_ok=True)
        imported.write_bytes(pe)
        sha = self.artifact(imported)['sha256']
        env = {'id': seed * 32, 'root': str(root), 'revision': 1,
            'runner': {'id': 'first-party', 'proton': str(program), 'entry_point': str(program), 'files': [self.artifact(program)]}}
        self.write(root / 'environment.json', env)
        install_op = ('f' if seed == 'a' else 'e') * 32
        onboarding = self.managed / 'onboarding' / env['id']
        self.write(onboarding / 'record.json', {'schema': 1, 'id': env['id'], 'environment': env,
            'installer': sha, 'installation_operation': install_op})
        result = onboarding / (install_op + '-result.json')
        self.write(result, {'schema': 2, 'operation': install_op, 'state': 'completed', 'raw_exit': 0,
            'cleanup_confirmed': True, 'owned_live': 0, 'error': None})
        self.write(self.managed / 'installers' / (sha + '.json'), {'schema': 1, 'id': sha, 'artifact': self.artifact(imported)})
        app = {'schema': 2, 'id': 'selected', 'environment': env, 'executable': self.artifact(program),
            'helpers': [], 'installer_sha256': sha, 'installer_result': self.artifact(result), 'observed_installer_version': 'unqualified-' + seed}
        binding = {'environment': env['id'], 'executable': app['executable'], 'helpers': [],
            'installer': sha, 'installation': app['installer_result']}
        selector = hashlib.sha256(canonical(binding)).hexdigest()
        software = {'manager': self.artifact(pathlib.Path(__file__)),
            'supervisor': self.artifact(pathlib.Path(s.__file__)), 'ownership': self.artifact(pathlib.Path(ownership.__file__)),
            'installer_launch': self.artifact(program)}
        self.write(self.managed / 'software.json', software)
        directory = self.managed / 'vendor-applications' / selector
        self.write(directory / 'application.json', app)
        capability = {'schema': 1, 'profile': self.profile,
            'profile_sha256': hashlib.sha256(canonical(self.profile)).hexdigest(), 'application': selector,
            'resources': files, 'dependency': {'installer': self.image(installer), 'daemon': self.image(daemon) if existing_daemon else None}}
        op = {'application': app, 'report': str(directory / 'operation-result.json'), 'mode': 'normal',
            'operation_id': ('1' if seed == 'a' else '2') * 32, 'launcher': software['installer_launch'],
            'managed': {'capabilities': capability, 'software': software}}
        self.write(directory / 'operation.json', op)
        self.write(pathlib.Path(op['report']), {'schema': 3, 'operation_id': op['operation_id'], 'state': 'unknown'})
        return op

    def persist(self, op):
        self.write(pathlib.Path(op['report']).parent / 'operation.json', op)

    def replacement_operation(self, seed):
        operation=self.operation(seed);app=operation['application'];before=copy.deepcopy(app['environment'])
        after=copy.deepcopy(before);after['revision']+=1;app['environment']=after
        self.write(pathlib.Path(after['root'])/'environment.json',after)
        installer=self.managed/'installers'/(seed+'-replacement.exe')
        installer.write_bytes(pathlib.Path(app['executable']['path']).read_bytes()+b'new installer bytes')
        sha=self.artifact(installer)['sha256'];app['installer_sha256']=sha
        self.write(self.managed/'installers'/(sha+'.json'),{'schema':1,'id':sha,'artifact':self.artifact(installer)})
        owner=('c' if seed=='a' else 'd')*32;directory=self.managed/'operator'/owner
        self.write(directory/'request.json',{'schema':26,'state_token':'accepted',
            'action':{'kind':'environment_installer_start','environment':after['id'],'installer':sha}})
        self.write(directory/'installation.json',{'schema':1,'operation':owner,'installer':sha,
            'before':before,'environment':after,'created_at':1})
        receipt=directory/'installation-result.json'
        self.write(receipt,{'schema':2,'operation':owner,'state':'completed','raw_exit':0,
            'cleanup_confirmed':True,'owned_live':0,'error':None})
        app['installer_result']=self.artifact(receipt)
        binding={'environment':after['id'],'executable':app['executable'],'helpers':app['helpers'],
            'installer':sha,'installation':app['installer_result']}
        selector=hashlib.sha256(canonical(binding)).hexdigest();operation['managed']['capabilities']['application']=selector
        selected=self.managed/'vendor-applications'/selector
        self.write(selected/'application.json',app);operation['report']=str(selected/'operation-result.json')
        self.persist(operation);self.write(pathlib.Path(operation['report']),{'schema':3,'state':'unknown'})
        return operation,directory

    def test_actual_dispatch_accepts_new_launcher_from_typed_replacement_receipt(self):
        for seed,policy in (('a',None),('b',None),('c','inherited'),('d','intentionally_unavailable')):
            operation,directory=self.replacement_operation(seed)
            if policy:
                request=json.loads((directory/'request.json').read_bytes())
                request['schema']=27
                request['action'].update(kind='environment_installer_start_with_policy',powershell=policy)
                self.write(directory/'request.json',request)
            original=self.managed/'onboarding'/operation['application']['environment']['id']/'record.json'
            history=original.read_bytes()
            self.assertEqual(s.selected_installer_origin(self.managed,operation['application']),directory.name)
            with patch.object(s,'renderer_run',return_value=True) as run:
                self.assertTrue(s.vendor_application(operation))
            self.assertEqual(run.call_args.kwargs['managed_contract'],operation['managed']['capabilities'])
            self.assertEqual(original.read_bytes(),history)

    def test_replacement_policy_origin_remains_closed_and_exact(self):
        operation,directory=self.replacement_operation('a')
        request=json.loads((directory/'request.json').read_bytes())
        action=dict(request['action'],kind='environment_installer_start_with_policy',powershell='inherited')
        for changed in (dict(action,powershell='unknown'),dict(action,extra=True),
                        dict(action,environment='other-setup'),dict(action,installer='f'*64)):
            self.write(directory/'request.json',dict(request,action=changed))
            with self.assertRaisesRegex(ValueError,'installation_binding'):
                s.selected_installer_origin(self.managed,operation['application'])

    def test_replacement_receipt_cannot_borrow_other_setup_installer_or_completion(self):
        operation,directory=self.replacement_operation('a');binding=json.loads((directory/'installation.json').read_bytes())
        for field,value in (('installer','0'*64),('operation','b'*32),('environment',dict(binding['environment'],id='b'*32))):
            changed=dict(binding);changed[field]=value;self.write(directory/'installation.json',changed)
            with self.assertRaisesRegex(ValueError,'installation_binding'):
                s.managed_vendor_inputs(operation)
        self.write(directory/'installation.json',binding)
        result=directory/'installation-result.json';completed=json.loads(result.read_bytes())
        for changed in (dict(completed,raw_exit=1),dict(completed,cleanup_confirmed=False),dict(completed,owned_live=None)):
            self.write(result,changed);operation['application']['installer_result']=self.artifact(result)
            with self.assertRaisesRegex(ValueError,'installation_incomplete'):
                s.selected_installer_origin(self.managed,operation['application'])

    def test_actual_operation_dispatch_uses_each_fresh_environment_and_version(self):
        for seed in ('a', 'b'):
            with self.subTest(seed=seed):
                operation = self.operation(seed)
                expected = operation['managed']['capabilities']
                self.assertEqual(s.managed_vendor_inputs(operation), expected)
                with patch.object(s, 'renderer_run', return_value=True) as run, \
                     patch.object(s, 'renderer_validate', side_effect=AssertionError('legacy admission used')), \
                     patch.object(s, 'nad1_session_admitted', side_effect=AssertionError('Deck receipt used')):
                    self.assertTrue(s.vendor_application(operation))
                spec = run.call_args.args[0]
                self.assertEqual(spec['application']['environment'], operation['application']['environment'])
                self.assertEqual(spec['application']['files'], expected['resources'])
                self.assertEqual(run.call_args.kwargs, {'managed_contract': expected})

    def test_definition_hash_is_data_not_application_version_allowlist(self):
        self.assertEqual(hashlib.sha256(canonical(self.profile)).hexdigest(), s.MANAGED_NATIVE_ACCESS_PROFILE_SHA256)
        self.assertNotIn('sha256', canonical(self.profile).decode())
        self.assertNotIn('environment', self.profile)

    def owner(self, operation):
        with patch.object(s, 'renderer_run', return_value=True) as run:
            self.assertTrue(s.vendor_application(operation))
        spec = run.call_args.args[0]
        children = []
        ledger = type('Ledger', (), {'launcher': lambda _, child, role: children.append(child),
            'harvest': lambda _: [child.poll() for child in children]})()
        return s.Nad1Owner(spec, ledger, None, lambda: False, contract=operation['managed']['capabilities'])

    def test_fresh_dependency_install_and_owned_retirement_use_actual_operation_without_legacy_receipts(self):
        for seed in ('a', 'b'):
            with self.subTest(seed=seed):
                operation = self.operation(seed)
                owner = self.owner(operation)
                self.assertTrue(owner.production)
                self.assertIsNone(owner.session_authority)
                daemon = owner.drive / owner.daemon_relative
                calls = []
                state = {'registered': False, 'running': False}
                def command(action):
                    calls.append(action)
                    if action == 'install':
                        daemon.parent.mkdir(parents=True, exist_ok=True)
                        daemon.write_bytes(pathlib.Path(operation['application']['executable']['path']).read_bytes())
                        state['registered'] = True
                    elif action == 'start': state['running'] = True
                    elif action == 'stop':
                        state['running'] = False
                        owner.stop_observation = {'control_count': 1}
                    return {'registration': 'exact' if state['registered'] else 'absent',
                        'state': 4 if state['running'] else 1, 'image_sha256': hashlib.sha256(daemon.read_bytes()).hexdigest() if daemon.exists() else 'none',
                        'owned_endpoint_mask': 3 if state['running'] else 0,
                        'retirement_generation_confirmed': action == 'stop'}
                owner.command = command
                def census(*_):
                    found = [{'prefix_relation': 'same', 'exact': True, 'linux_pid': 10, 'start_ticks': 20}] if state['running'] else []
                    return {'candidates': found, 'private': found, 'unavailable': 0}
                with patch.object(ownership, 'census', side_effect=census), \
                     patch.object(s, 'nad1_generation_owned', return_value=True), \
                     patch.object(s, 'nad1_retain_artifact', side_effect=AssertionError('historical namespace used')):
                    self.assertTrue(owner.ensure(True)['ready_tested'])
                    self.assertEqual(calls, ['query', 'install', 'query', 'start', 'query'])
                    self.assertTrue(owner.retire_service())
                    self.assertTrue(owner.retire_service())
                self.assertEqual(calls.count('install'), 1)
                self.assertEqual(calls.count('stop'), 1)
                self.assertEqual(owner.service_stop_request_count, 1)
                self.assertFalse((self.managed / 'vendor-applications/native-access-dependency').exists())
                owner.bound_daemon.check()
                self.assertEqual(owner.bound_daemon.sha, hashlib.sha256(daemon.read_bytes()).hexdigest())
                self.assertTrue(owner.close_runtime())

    def test_bound_daemon_replacement_refuses_before_service_start(self):
        operation=self.operation('a',existing_daemon=True)
        owner=self.owner(operation);daemon=owner.drive/owner.daemon_relative
        data=bytearray(daemon.read_bytes());data[-1]^=1;daemon.write_bytes(data)
        try:
            with patch.object(s,'Nad1Runtime',side_effect=AssertionError('changed daemon reached service runtime')):
                with self.assertRaisesRegex(ValueError,'renderer_image_replaced'):
                    owner.command('start')
        finally:owner.close_runtime()

    def test_managed_service_request_passes_exact_observed_bundled_path_and_profile_facts(self):
        operation = self.operation('a')
        owner = self.owner(operation)
        requests = []
        def argv(request, cwd, env):
            fields = request.read_bytes().decode('utf-16le').splitlines()
            requests.append(fields)
            self.assertEqual(fields[0], 'NAD1_SERVICE_V2')
            self.assertEqual(fields[1:5], [owner.op, owner.token, 'query', owner.installer_sha])
            self.assertEqual(fields[5:], [s.windows(pathlib.Path(operation['managed']['capabilities']['dependency']['installer']['artifact']['path']), owner.prefix),
                'NTKDaemonService', s.windows(owner.drive / owner.daemon_relative, owner.prefix), '5146', '5563'])
            return [sys.executable, '-c', 'print("NAD1_SCM_V1 ' + owner.op + ' ' + owner.token + ' absent 1060 0 0 0 none 0 0 0")']
        owner.runtime = type('Runtime', (), {'argv': staticmethod(argv), 'drain': lambda _: None})()
        (owner.root / 'home').mkdir()
        with patch.object(s, 'environment', return_value=dict(os.environ)):
            self.assertEqual(owner.command('query')['registration'], 'absent')
        self.assertEqual(len(requests), 1)

    def test_existing_unregistered_or_foreign_service_cannot_be_reinstalled_or_stopped(self):
        operation = self.operation('a', existing_daemon=True)
        for foreign in (False, True):
            with self.subTest(foreign=foreign):
                owner = self.owner(operation)
                daemon = owner.drive / owner.daemon_relative
                original = daemon.read_bytes()
                admitted = ownership.image_identity(daemon)
                calls = []
                def command(action):
                    calls.append(action)
                    return {'registration': 'absent', 'state': 0}
                owner.command = command
                scan = {'candidates': [{'prefix_relation': 'foreign', 'exact': True}] if foreign else [], 'private': [], 'unavailable': 0}
                with patch.object(ownership, 'census', return_value=scan), patch.object(s.time, 'sleep'):
                    with self.assertRaisesRegex(ValueError, 'foreign_conflict' if foreign else 'qualified_registration_absent'):
                        owner.ensure(False, admitted)
                    self.assertEqual(owner.retire_service(), not foreign)
                self.assertEqual(daemon.read_bytes(), original)
                self.assertEqual(calls, [] if foreign else ['query', 'query'])
                self.assertEqual(owner.service_stop_request_count, 0)

    def test_ordinary_operation_resolves_fresh_runtime_tools_from_its_verified_tree(self):
        operation = self.operation('a')
        base = self.managed / 'runners/new-runtime'
        paths = [base / name for name in ['runner/proton', 'runner/files/bin/wine', 'slr/_v2-entry-point',
            'slr/pressure-vessel/bin/steam-runtime-launch-client', 'slr/pressure-vessel/bin/steam-runtime-launcher-interface-0']]
        for index, path in enumerate(paths):
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(('first-party-runtime-' + str(index)).encode())
            path.chmod(0o700)
        rows = []
        for path in base.rglob('*'):
            directory = path.is_dir()
            rows.append({'path': str(path.relative_to(base)), 'directory': directory, 'target': None,
                'sha256': None if directory else self.artifact(path)['sha256'],
                'size': 0 if directory else path.stat().st_size, 'mode': path.stat().st_mode & 0o777})
        manifest = base / 'runtime-tree.json'
        self.write(manifest, rows)
        manifest.chmod(0o400)
        runner = {'id': 'source-owned-fresh-runtime', 'proton': str(paths[0]), 'entry_point': str(paths[2]),
            'files': [self.artifact(path) for path in [paths[0], paths[1], paths[2], manifest]]}
        operation['application']['environment']['runner'] = runner
        self.write(pathlib.Path(operation['application']['environment']['root']) / 'environment.json', operation['application']['environment'])
        self.write(pathlib.Path(operation['report']).parent / 'application.json', operation['application'])
        self.persist(operation)
        runtime = s.Nad1Runtime(self.owner(operation))
        try:
            runtime.verify_tools()
            self.assertEqual(runtime.tool_sha256['client'], self.artifact(paths[3])['sha256'])
            self.assertEqual(runtime.tool_sha256['interface'], self.artifact(paths[4])['sha256'])
            paths[3].write_bytes(b'substituted tool')
            with self.assertRaises((ValueError, RuntimeError)): runtime.verify_tools()
        finally: runtime.close()

    def test_empty_cgroup_does_not_translate_unconfirmed_dependency_into_retirement(self):
        value = {'operation': '1' * 32, 'state': 'failed', 'cleanup_confirmed': True, 'owned_live': 0,
            'dependency': {'process_cleanup_confirmed': True, 'dependency_cleanup_disposition': 'cleanup_unconfirmed'}}
        result = s.managed_vendor_result(value)
        self.assertFalse(result['cleanup_confirmed'])
        self.assertEqual(result['state'], 'cleanup_unconfirmed')
        dependency = {'process_cleanup_confirmed': True, 'dependency_cleanup_disposition': 'exact_owned_session_cleanup',
            'post_cleanup': {'cgroup_empty': True, 'exact_daemon_generation_absent': True, 'application_image_unchanged': True,
                'daemon_image_unchanged': True, 'listener_mask': 0, 'candidate_count': 0, 'candidate_observation_unavailable': 0}}
        value['dependency'] = dependency
        self.assertTrue(s.managed_vendor_result(value)['cleanup_confirmed'])
        for key, changed in [('exact_daemon_generation_absent', False), ('listener_mask', 1), ('candidate_observation_unavailable', 1)]:
            altered = copy.deepcopy(value)
            altered['dependency']['post_cleanup'][key] = changed
            self.assertFalse(s.managed_vendor_result(altered)['cleanup_confirmed'])

    @unittest.skipUnless(sys.platform == 'linux', 'Linux operation owner')
    def test_resource_replacement_during_dependency_preparation_refuses_actual_application_dispatch(self):
        from test_installer import Scope
        operation = self.operation('a')
        root = pathlib.Path(operation['application']['environment']['root'])
        (root / 'compatdata/pfx/system.reg').touch()
        resource = pathlib.Path(operation['managed']['capabilities']['resources']['resources/app.asar']['artifact']['path'])
        class Dependency:
            bound_daemon=None
            runtime = type('Runtime', (), {'argv': lambda *_: ['never-launch'], 'drain': lambda *_: None})()
            anchor = None
            retirement_error = None
            def __init__(self, *_, **__): self.process_cleanup_confirmed = False
            def ensure(self, *_):
                replacement = resource.with_suffix('.replacement')
                replacement.write_bytes(resource.read_bytes())
                replacement.replace(resource)
            def retire_service(self): return True
            def close_runtime(self): return True
            def verify_session_cleanup(self, *_): return True
            def value(self): return {'process_cleanup_confirmed': self.process_cleanup_confirmed,
                'dependency_cleanup_disposition': 'graceful_service_retirement',
                'post_cleanup': {'cgroup_empty': True, 'exact_daemon_generation_absent': True, 'application_image_unchanged': True,
                    'daemon_image_unchanged': True, 'listener_mask': 0, 'candidate_count': 0, 'candidate_observation_unavailable': 0}}
        prior = [signal.getsignal(sig) for sig in (signal.SIGTERM, signal.SIGINT)]
        try:
            with patch.object(s, 'Nad1Owner', Dependency), patch.object(s, 'CompanionCgroup', return_value=Scope()), \
                 patch.object(s.subprocess, 'Popen', side_effect=AssertionError('substituted application launched')) as launch:
                self.assertFalse(s.vendor_application(operation))
                launch.assert_not_called()
            result = json.loads(pathlib.Path(operation['report']).read_text())
            self.assertEqual(result['error_code'], 'renderer_image_replaced')
            self.assertNotIn('renderer_image', result['error'])
        finally:
            for sig, old in zip((signal.SIGTERM, signal.SIGINT), prior): signal.signal(sig, old)
            s.ctypes.CDLL(None).prctl(36, 0, 0, 0, 0)

    @unittest.skipUnless(sys.platform == 'linux', 'Linux operation owner')
    def test_registry_default_failure_is_reported_and_application_launch_continues(self):
        from test_installer import Scope
        operation = self.operation('a')
        root = pathlib.Path(operation['application']['environment']['root'])
        (root / 'compatdata/pfx/system.reg').touch()
        requested = []
        class Dependency:
            bound_daemon = None
            runtime = type('Runtime', (), {'argv': lambda *_: ['application-launch'], 'drain': lambda *_: None})()
            anchor = None
            retirement_error = None
            def __init__(self, *_, **__): self.process_cleanup_confirmed = False
            def ensure(self, *_): pass
            def registry_defaults(self, defaults):
                requested.append(defaults)
                return [{'known_folder': 'Downloads', 'result': 'failed', 'reason': 'timeout'}]
            def retire_service(self): return True
            def close_runtime(self): return True
            def verify_session_cleanup(self, *_): return True
            def value(self): return {'process_cleanup_confirmed': self.process_cleanup_confirmed,
                'dependency_cleanup_disposition': 'graceful_service_retirement',
                'post_cleanup': {'cgroup_empty': True, 'exact_daemon_generation_absent': True, 'application_image_unchanged': True,
                    'daemon_image_unchanged': True, 'listener_mask': 0, 'candidate_count': 0, 'candidate_observation_unavailable': 0}}
        prior = [signal.getsignal(sig) for sig in (signal.SIGTERM, signal.SIGINT)]
        try:
            with patch.object(s, 'Nad1Owner', Dependency), patch.object(s, 'CompanionCgroup', return_value=Scope()), \
                 patch.object(s, 'environment', return_value=dict(os.environ)), \
                 patch.object(s.subprocess, 'Popen', side_effect=OSError('fixture launch')) as launch:
                self.assertFalse(s.vendor_application(operation))
                launch.assert_called_once()
                self.assertEqual(launch.call_args.args[0], ['application-launch'])
            self.assertEqual(requested, [self.profile['registry_defaults']])
            result = json.loads(pathlib.Path(operation['report']).read_text())
            self.assertEqual(result['error_code'], 'application_launch_failed')
            self.assertEqual(result['registry_defaults'], [{'known_folder': 'Downloads', 'result': 'failed', 'reason': 'timeout'}])
        finally:
            for sig, old in zip((signal.SIGTERM, signal.SIGINT), prior): signal.signal(sig, old)
            s.ctypes.CDLL(None).prctl(36, 0, 0, 0, 0)

    @unittest.skipUnless(sys.platform=='linux','Linux operation owner')
    def test_definite_runtime_pre_service_failure_retires_without_unknown_custody_claim(self):
        from test_installer import Scope
        for unresolved in (False,True):
            operation=self.operation('b' if unresolved else 'a')
            root=pathlib.Path(operation['application']['environment']['root'])
            (root/'compatdata/pfx/system.reg').touch()
            prior=[signal.getsignal(sig) for sig in (signal.SIGTERM,signal.SIGINT)]
            scan={'candidates':[],'private':[],'unavailable':int(unresolved)}
            try:
                with patch.object(s,'CompanionCgroup',return_value=Scope()),patch.object(ownership,'census',return_value=scan), \
                     patch.object(s,'environment',return_value=dict(os.environ)), \
                     patch.object(s.subprocess,'Popen',side_effect=AssertionError('failed runtime reached launch')) as launch:
                    self.assertFalse(s.vendor_application(operation));launch.assert_not_called()
                value=json.loads(pathlib.Path(operation['report']).read_text())
                self.assertEqual(value['cleanup_confirmed'],not unresolved)
                self.assertEqual(value['dependency']['dependency_cleanup_disposition'],
                    'cleanup_unconfirmed' if unresolved else 'not_started')
                self.assertFalse(value['dependency']['service_actions_started'])
            finally:
                for sig,old in zip((signal.SIGTERM,signal.SIGINT),prior):signal.signal(sig,old)
                s.ctypes.CDLL(None).prctl(36,0,0,0,0)

    def test_changed_origin_resources_or_definition_refuse_before_launch(self):
        operation = self.operation('a')
        for change in ('environment', 'installation', 'resource', 'definition', 'bundled_alias'):
            with self.subTest(change=change):
                altered = copy.deepcopy(operation)
                capability = altered['managed']['capabilities']
                if change == 'environment': altered['application']['environment']['id'] = 'b' * 32
                elif change == 'installation': altered['application']['installer_result']['path'] = str(self.home / 'foreign.json')
                elif change == 'resource': capability['resources']['resources/app.asar']['artifact']['sha256'] = '0' * 64
                elif change == 'definition': capability['profile']['dependency']['service'] = 'UnrelatedService'
                elif change == 'bundled_alias': capability['dependency']['installer']['artifact']['path'] = str(self.home / 'unrelated.exe')
                self.persist(altered)
                with patch.object(s, 'renderer_run') as run:
                    with self.assertRaises((ValueError, RuntimeError, FileNotFoundError, KeyError)):
                        s.managed_vendor_inputs(altered)
                    if change not in ('environment', 'installation'):
                        self.assertFalse(s.vendor_application(altered))
                        report = json.loads(pathlib.Path(altered['report']).read_text())
                        self.assertFalse(report['cleanup_confirmed'])
                        self.assertIsNone(report['owned_live'])
                    run.assert_not_called()
                self.persist(operation)


if __name__ == '__main__':
    unittest.main()
