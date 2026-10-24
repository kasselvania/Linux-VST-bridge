import copy, hashlib, json, os, pathlib, shutil, signal, sys, tempfile, unittest
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
        # The saved binding: the bundled profile without launch-time settings.
        bundled = json.loads((pathlib.Path(__file__).resolve().parents[2] / 'crates/service/src/vendor_profiles/native_access.json').read_text())
        self.profile = {key: value for key, value in bundled.items() if key not in ('registry_defaults', 'installer_started_app')}
        self.home_patch = patch.object(pathlib.Path, 'home', return_value=self.home)
        self.home_patch.start()
        # Nothing else holds the service's ports here, whatever the host runs.
        self.ports_patch = patch.object(s, 'nad1_listener_census', return_value={'schema': 1, 'listener_mask': 0})
        self.ports_patch.start(); self.addCleanup(self.ports_patch.stop)

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
            request=json.loads((directory/'request.json').read_bytes())
            request['schema']=30
            if policy:
                request['action'].update(kind='environment_installer_start_with_policy',powershell=policy)
            self.write(directory/'request.json',request)
            original=self.managed/'onboarding'/operation['application']['environment']['id']/'record.json'
            history=original.read_bytes()
            self.assertEqual(s.selected_installer_origin(self.managed,operation['application']),directory.name)
            with patch.object(s,'renderer_run',return_value=True) as run:
                self.assertTrue(s.vendor_application(operation))
            self.assertEqual(run.call_args.kwargs['managed_contract'],operation['managed']['capabilities'])
            self.assertEqual(original.read_bytes(),history)

    def test_dependency_installer_continuation_accepts_retained_and_current_wire_generations(self):
        operation,directory=self.replacement_operation('a')
        request=json.loads((directory/'request.json').read_bytes())
        for schema in (26,27,28,29,30):
            request['schema']=schema
            self.write(directory/'request.json',request)
            self.assertEqual(s.selected_installer_origin(self.managed,operation['application']),directory.name)
        request['schema']=31
        self.write(directory/'request.json',request)
        with self.assertRaisesRegex(ValueError,'managed_application_installation_binding'):
            s.selected_installer_origin(self.managed,operation['application'])

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
            'harvest': lambda _: [child.poll() for child in children], 'phase': 'dependency_query', 'first_failure': None})()
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
    def test_registry_default_outcome_reaches_the_manager_inside_the_dependency_record(self):
        # Older managers reject unknown top-level result fields; the dependency
        # record is free-form for them, so a rollback keeps reading results.
        from test_installer import Scope
        operation = self.operation('a')
        root = pathlib.Path(operation['application']['environment']['root'])
        (root / 'compatdata/pfx/system.reg').touch()
        outcome = [{'known_folder': 'Downloads', 'result': 'failed', 'reason': 'timeout'}]
        class Dependency:
            bound_daemon = None
            runtime = type('Runtime', (), {'argv': lambda *_: ['application-launch'], 'drain': lambda *_: None})()
            anchor = None
            retirement_error = None
            def __init__(self, *_, **__): self.process_cleanup_confirmed = False
            def ensure(self, *_): pass
            def retire_service(self): return True
            def close_runtime(self): return True
            def verify_session_cleanup(self, *_): return True
            def value(self): return {'process_cleanup_confirmed': self.process_cleanup_confirmed,
                'dependency_cleanup_disposition': 'graceful_service_retirement', 'registry_defaults': outcome,
                'post_cleanup': {'cgroup_empty': True, 'exact_daemon_generation_absent': True, 'application_image_unchanged': True,
                    'daemon_image_unchanged': True, 'listener_mask': 0, 'candidate_count': 0, 'candidate_observation_unavailable': 0}}
        prior = [signal.getsignal(sig) for sig in (signal.SIGTERM, signal.SIGINT)]
        try:
            with patch.object(s, 'Nad1Owner', Dependency), patch.object(s, 'CompanionCgroup', return_value=Scope()), \
                 patch.object(s, 'environment', return_value=dict(os.environ)), \
                 patch.object(s.subprocess, 'Popen', side_effect=OSError('fixture launch')) as launch:
                self.assertFalse(s.vendor_application(operation))
                launch.assert_called_once()
            result = json.loads(pathlib.Path(operation['report']).read_text())
            self.assertEqual(result['error_code'], 'application_launch_failed')
            self.assertEqual(result['dependency']['registry_defaults'], outcome)
            self.assertNotIn('registry_defaults', result)
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


class RecoverCensusTests(unittest.TestCase):
    """An unconfirmed session is retired only when no process or listener remains."""
    HEADER = '  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n'
    KEYS = {'schema', 'verdict', 'prefix_unchanged', 'application_image_unchanged', 'daemon_image_unchanged',
        'daemon_identity', 'daemon_name_observable', 'images_changed', 'candidate_count',
        'candidate_observation_unavailable', 'listener_mask'}

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        base = pathlib.Path(self.temp.name).resolve()
        self.home = base / 'home'
        home_patch = patch.object(pathlib.Path, 'home', return_value=self.home)
        home_patch.start(); self.addCleanup(home_patch.stop)
        self.proc = base / 'proc'
        (self.proc / 'net').mkdir(parents=True)
        for name in ('tcp', 'tcp6'):
            (self.proc / 'net' / name).write_text(self.HEADER)
        self.root = self.home / '.local/share/linux-vst-bridge/managed/environments' / ('e' * 32)
        self.prefix = self.root / 'compatdata/pfx'
        drive = self.prefix / 'drive_c'
        self.environment = {'id': 'e' * 32, 'root': str(self.root), 'revision': 1}
        drive.mkdir(parents=True)
        (self.root / 'environment.json').write_bytes(canonical(self.environment))
        self.application = drive / 'Program Files/Vendor App/App.exe'
        self.application.parent.mkdir(parents=True)
        self.application.write_bytes(b'MZ application')
        self.relative = 'Program Files/Common Files/Vendor/' + ownership.NAME
        self.daemon = drive / self.relative
        self.daemon.parent.mkdir(parents=True)
        pe = bytearray(256); pe[:2] = b'MZ'; pe[60:64] = (64).to_bytes(4, 'little')
        pe[64:68] = b'PE\0\0'; pe[68:70] = b'\x64\x86'
        self.daemon.write_bytes(pe)
        identity = ownership.image_identity(self.daemon)
        self.request = {'schema': 1, 'environment': self.environment,
            'application': {'path': str(self.application), 'sha256': hashlib.sha256(b'MZ application').hexdigest()},
            'daemon': {'relative': self.relative, 'identity': {'sha256': identity['sha256'], 'size': identity['size']}},
            'listeners': [5146, 5563]}

    def census(self, request=None):
        value = s.vendor_recover_census(copy.deepcopy(request or self.request), proc=self.proc)
        self.assertEqual(set(value), self.KEYS)
        return value

    def daemon_process(self, prefix=None):
        task = self.proc / '123'; task.mkdir()
        (task / 'stat').write_text('123 (' + ownership.NAME + ') S ' + '0 ' * 18 + '10\n')
        (task / 'comm').write_bytes(ownership.NAME.encode() + b'\n')
        (task / 'cmdline').write_bytes(str(self.daemon).encode() + b'\0')
        (task / 'maps').write_bytes(b'1000-2000 r-xp 0 00:00 1 ' + str(self.daemon).encode() + b'\n')
        (task / 'environ').write_bytes(b'WINEPREFIX=' + str(prefix or self.prefix).encode() + b'\0')

    def test_complete_absence_retires_and_records_image_facts_truthfully(self):
        value = self.census()
        self.assertEqual(value['verdict'], 'absent')
        self.assertEqual((value['daemon_identity'], value['daemon_image_unchanged'], value['images_changed'],
            value['listener_mask'], value['candidate_count'], value['candidate_observation_unavailable']),
            ('unchanged', True, [], 0, 0, 0))
        self.daemon.unlink()
        value = self.census()
        self.assertEqual((value['verdict'], value['daemon_identity'], value['daemon_image_unchanged'], value['images_changed']),
            ('absent', 'absent', False, []))
        unadmitted = copy.deepcopy(self.request); unadmitted['daemon']['identity'] = None
        value = self.census(unadmitted)
        self.assertEqual((value['daemon_identity'], value['daemon_image_unchanged']), ('not_admitted_in_session', False))

    def test_changed_or_unreadable_images_do_not_keep_a_session_that_nothing_runs(self):
        # e.g. the app updated itself mid-session or the daemon was half written.
        changed = copy.deepcopy(self.request); changed['daemon']['identity']['sha256'] = '0' * 64
        value = self.census(changed)
        self.assertEqual((value['verdict'], value['daemon_identity'], value['images_changed']), ('absent', 'changed', ['daemon']))
        self.daemon.write_bytes(b'partial')
        self.assertEqual(self.census()['daemon_identity'], 'unavailable')
        self.application.write_bytes(b'MZ updated application')
        value = self.census()
        self.assertEqual((value['verdict'], value['application_image_unchanged'], value['images_changed']),
            ('absent', False, ['application', 'daemon']))
        self.environment['revision'] = 2
        value = self.census()
        self.assertEqual((value['verdict'], value['prefix_unchanged']), ('absent', False))
        self.daemon_process()
        self.assertEqual(self.census()['verdict'], 'running')

    def test_daemon_generation_in_any_prefix_keeps_the_session(self):
        for prefix in (None, self.root / 'other-prefix'):
            with self.subTest(prefix=prefix):
                if prefix is not None: prefix.mkdir()
                self.daemon_process(prefix)
                value = self.census()
                self.assertEqual((value['verdict'], value['candidate_count']), ('running', 1))
                (self.proc / '123' / 'maps').unlink()
                self.assertEqual(self.census()['verdict'], 'running')
                shutil.rmtree(self.proc / '123')

    def test_process_scan_stops_inside_the_managers_limit(self):
        with patch.object(ownership, 'census', return_value={'candidates': [], 'unavailable': 0}) as scan:
            self.assertEqual(self.census()['verdict'], 'absent')
        self.assertEqual(scan.call_args.kwargs['seconds'], 20)
        self.assertLess(s.VENDOR_RECOVER_SCAN_SECONDS, 30)

    def test_profile_listener_keeps_the_session(self):
        (self.proc / 'net/tcp6').write_text(self.HEADER +
            '   0: 00000000000000000000000001000000:15BB 00000000000000000000000000000000:0000 0A 0:0 0:0 0 0 0\n')
        value = self.census()
        self.assertEqual((value['verdict'], value['listener_mask']), ('running', 2))
        other = copy.deepcopy(self.request); other['listeners'] = [6000]
        self.assertEqual(self.census(other)['verdict'], 'absent')

    def test_unobserved_processes_or_listeners_never_retire(self):
        (self.proc / 'net/tcp6').unlink()
        value = self.census()
        self.assertEqual((value['verdict'], value['listener_mask']), ('unavailable', None))
        (self.proc / 'net/tcp6').write_text(self.HEADER)
        other = copy.deepcopy(self.request); other['daemon']['relative'] = 'Program Files/Vendor/OtherDaemon.exe'
        value = self.census(other)
        self.assertEqual((value['verdict'], value['daemon_name_observable']), ('unavailable', False))
        self.daemon_process(); (self.proc / '123' / 'maps').unlink(); (self.proc / '123' / 'comm').unlink()
        (self.proc / '123' / 'stat').write_text('123 (wine64-preload) S ' + '0 ' * 18 + '10\n')
        self.assertEqual(self.census()['verdict'], 'running')

    def test_reverified_record_retires_only_with_its_marker_absence_and_consistent_image_facts(self):
        dependency = {'process_cleanup_confirmed': True, 'dependency_cleanup_disposition': s.REVERIFIED_CLEANUP,
            'post_cleanup': {'cgroup_empty': True, 'exact_daemon_generation_absent': True, 'application_image_unchanged': True,
                'daemon_image_unchanged': True, 'daemon_identity': 'unchanged', 'images_changed': [], 'listener_mask': 0,
                'candidate_count': 0, 'candidate_observation_unavailable': 0, 'session': s.REVERIFIED_CLEANUP}}
        self.assertTrue(s.managed_dependency_retired(dependency))
        for changes in ({'application_image_unchanged': False, 'images_changed': ['application']},
                {'daemon_image_unchanged': False, 'daemon_identity': 'absent'},
                {'daemon_image_unchanged': False, 'daemon_identity': 'changed', 'images_changed': ['daemon']},
                {'daemon_image_unchanged': False, 'daemon_identity': 'unavailable', 'images_changed': ['daemon']}):
            altered = copy.deepcopy(dependency); altered['post_cleanup'].update(changes)
            self.assertTrue(s.managed_dependency_retired(altered), changes)
        for changes in ({'session': 'failed_before_readiness'}, {'session': None}, {'candidate_count': 1},
                {'listener_mask': None}, {'cgroup_empty': False}, {'exact_daemon_generation_absent': False},
                {'candidate_observation_unavailable': 1}, {'application_image_unchanged': False},
                {'daemon_identity': 'absent'}, {'images_changed': ['application']}, {'images_changed': None},
                {'daemon_image_unchanged': False, 'daemon_identity': 'changed'}):
            altered = copy.deepcopy(dependency); altered['post_cleanup'].update(changes)
            self.assertFalse(s.managed_dependency_retired(altered), changes)
        legacy = copy.deepcopy(dependency); legacy['dependency_cleanup_disposition'] = 'reverified_after_interruption'
        self.assertFalse(s.managed_dependency_retired(legacy))
        exact = copy.deepcopy(dependency); exact['dependency_cleanup_disposition'] = 'exact_owned_session_cleanup'
        exact['post_cleanup']['application_image_unchanged'] = False
        self.assertFalse(s.managed_dependency_retired(exact), 'Image changes are accepted only after a re-check')

    def test_requests_are_closed(self):
        for change in ({'extra': 1}, {'schema': 2}, {'listeners': []}, {'listeners': [0]},
                {'daemon': {'relative': '../escape/' + ownership.NAME, 'identity': None}},
                {'daemon': {'relative': '/abs/' + ownership.NAME, 'identity': None}},
                {'application': {'path': str(self.application)}},
                {'application': {'path': '/usr/bin/App.exe', 'sha256': '0' * 64}},
                {'application': {'path': str(self.prefix / 'drive_c/../../App.exe'), 'sha256': '0' * 64}},
                {'environment': dict(self.environment, root=str(self.home / 'elsewhere'))},
                {'environment': dict(self.environment, id='../' + 'e' * 29)}):
            with self.subTest(change=change):
                with self.assertRaises(ValueError):
                    s.vendor_recover_census(dict(copy.deepcopy(self.request), **change), proc=self.proc)

    @unittest.skipUnless(sys.platform == 'linux', 'Linux process census')
    def test_supervisor_entry_point_prints_one_closed_verdict(self):
        import subprocess
        result = subprocess.run([sys.executable, str(pathlib.Path(s.__file__)), '--vendor-recover-census'],
            input=json.dumps(self.request).encode(), capture_output=True, timeout=60,
            env=dict(os.environ, PYTHONDONTWRITEBYTECODE='1', HOME=str(self.home)))
        self.assertEqual(result.returncode, 0, result.stderr)
        value = json.loads(result.stdout)
        self.assertEqual(set(value), self.KEYS)
        self.assertIn(value['verdict'], ('absent', 'running', 'unavailable'))
        self.assertTrue(value['application_image_unchanged'] and value['prefix_unchanged'])
        for request in (b'{}', b'not json', json.dumps(dict(self.request, extra=1)).encode()):
            bad = subprocess.run([sys.executable, str(pathlib.Path(s.__file__)), '--vendor-recover-census'],
                input=request, capture_output=True, timeout=60, env=dict(os.environ, HOME=str(self.home)))
            self.assertEqual(bad.returncode, 1)
            self.assertEqual(json.loads(bad.stdout), {'schema': 1, 'error': 'vendor_recover_census_request'})
        with patch.object(s, 'vendor_recover_census', side_effect=OSError('boom')), \
                patch.object(sys, 'stdin', unittest.mock.Mock(buffer=unittest.mock.Mock(read=lambda n: b'{}'))), \
                patch('builtins.print') as printed:
            self.assertEqual(s.vendor_recover_census_main(), 1)
        self.assertEqual(json.loads(printed.call_args.args[0]), {'schema': 1, 'error': 'vendor_recover_census_internal'})


if __name__ == '__main__':
    unittest.main()
