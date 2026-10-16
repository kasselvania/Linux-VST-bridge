"""An app an installer starts by itself must not stay running in place of its
managed first launch; only that exact app, started near the installer's
successful end, is ended, and only after the installer itself exited 0."""
import json,os,pathlib,signal,subprocess,sys,tempfile,time,unittest
from unittest.mock import patch
import session
from test_installer import Scope

PROFILE={'profile':'fixture_app','executable':'App.exe','resources':['resources/app.asar','app.pak']}
def create(ts='1.000',path='C:\\\\Program Files\\\\Vendor\\\\App.exe'):
    return f'{ts}:0020:0024:trace:process:CreateProcessInternalW app L"{path}" cmdline L"x"\n'
STARTED='1.001:0020:0024:trace:process:CreateProcessInternalW started process pid 0030 tid 0034\n'
CREATE=create()

def root_lines(artifact,*,code=0,started='0.900',ended='1.500'):
    windows='Z:'+str(artifact).replace('/','\\\\')
    return ([f'{started}:0010:0014:trace:process:CreateProcessInternalW app L"{windows}" cmdline L"x"\n',
        f'{started}1:0010:0014:trace:process:CreateProcessInternalW started process pid 0020 tid 0024\n'],
        f'{ended}:0020:0024:trace:process:NtTerminateProcess handle 0xffffffffffffffff, exit_code {code}, process_exiting 1.\n')

def app_tree(root,*,resources=True):
    directory=root/'compatdata/pfx/drive_c/Program Files/Vendor';(directory/'resources').mkdir(parents=True)
    (directory/'App.exe').write_bytes(b'MZ'+bytes(4094))
    if resources:
        for name in PROFILE['resources']:(directory/name).write_bytes(b'r')
    return directory/'App.exe'

class ProfileDataTests(unittest.TestCase):
    def test_manager_profile_data_is_closed(self):
        self.assertEqual(session.installer_first_launch_profiles({}),[])
        self.assertEqual(session.installer_first_launch_profiles({'managed_first_launch':[PROFILE]}),[PROFILE])
        for bad in [{'profile':'x','executable':'App.exe'},dict(PROFILE,extra=1),dict(PROFILE,executable='../App.exe'),
                dict(PROFILE,executable='dir\\App.exe'),dict(PROFILE,executable='App.dll'),dict(PROFILE,profile='Not Valid'),
                dict(PROFILE,resources=[]),dict(PROFILE,resources=['../x']),dict(PROFILE,resources=['/abs']),dict(PROFILE,resources=['a//b'])]:
            with self.subTest(bad=bad),self.assertRaises(ValueError):session.installer_first_launch_profiles({'managed_first_launch':[bad]})
        with self.assertRaises(ValueError):session.installer_first_launch_profiles({'managed_first_launch':{}})

class ImageMatchTests(unittest.TestCase):
    def trace(self,root,lines,*,code=0,started='0.900',ended='1.500',rooted=True):
        artifact=root/'installer.exe'
        trace=session.InstallerWindowsTrace({'path':str(artifact),'sha256':'0'*64},root);trace.begin()
        head,tail=root_lines(artifact,code=code,started=started,ended=ended)
        for line in (head+lines+[tail] if rooted else lines):trace.feed(line.encode())
        return trace
    def found(self,root,lines,profiles=(PROFILE,),**kw):
        return session.installer_started_profiled_images(self.trace(root,lines,**kw),list(profiles))
    def test_only_a_running_profiled_image_with_its_resources_in_this_prefix(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp).resolve();exe=app_tree(root);st=exe.stat()
            found=self.found(root,[CREATE,STARTED])
            self.assertEqual(found[(os.major(st.st_dev),os.minor(st.st_dev),st.st_ino)],{'profile':'fixture_app',
                'image':'Program Files/Vendor/App.exe','executable':'App.exe','suffix':b'/drive_c/Program Files/Vendor/App.exe'})
            # Windows names are case-insensitive; the file found must still exist.
            self.assertTrue(self.found(root,[CREATE,STARTED],[dict(PROFILE,executable='APP.EXE')]))
            exited='1.002:0030:0034:trace:process:NtTerminateProcess handle 0xffffffffffffffff, exit_code 0, process_exiting 1.\n'
            self.assertEqual(self.found(root,[CREATE,STARTED,exited]),{})
            self.assertEqual(self.found(root,[CREATE,STARTED],[]),{})
            self.assertEqual(self.found(root,[CREATE,STARTED],[dict(PROFILE,executable='Other.exe')]),{})
    def test_nothing_is_closed_before_the_installer_itself_exits_0(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp).resolve();app_tree(root)
            self.assertEqual(self.found(root,[CREATE,STARTED],rooted=False),{})
            self.assertEqual(self.found(root,[CREATE,STARTED],code=3),{})
            self.assertTrue(self.found(root,[CREATE,STARTED],code=0))
    def test_only_a_copy_started_near_the_installers_end(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp).resolve();app_tree(root)
            # Started 59 s before the installer exited: closed.
            self.assertTrue(self.found(root,[CREATE,STARTED],started='0.500',ended='60.000'))
            # Started more than 60 s before the installer exited: left alone.
            self.assertEqual(self.found(root,[CREATE,STARTED],started='0.500',ended='61.500'),{})
            # Started before the installer itself: left alone.
            self.assertEqual(self.found(root,[create('0.100'),STARTED.replace('1.001','0.101')],started='0.500',ended='1.500'),{})
            # Started by the shell just after the installer exited: still its copy.
            self.assertTrue(self.found(root,[create('1.600'),STARTED.replace('1.001','1.601')],started='0.500',ended='1.500'))
    def test_home_reached_through_a_symlink_still_finds_the_app(self):
        with tempfile.TemporaryDirectory() as tmp:
            real=pathlib.Path(tmp).resolve()/'var-home';real.mkdir();app_tree(real)
            link=pathlib.Path(tmp).resolve()/'home';link.symlink_to(real)
            self.assertEqual(len({v['image'] for v in self.found(link,[CREATE,STARTED]).values()}),1)
    def test_installer_root_success_is_its_own_traced_exit_not_the_runner(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp).resolve();artifact=root/'installer.exe';artifact.write_bytes(b'x')
            head,ok=root_lines(artifact);_,failed=root_lines(artifact,code=3)
            self.assertFalse(self.trace(root,head,rooted=False).root_succeeded())
            self.assertEqual(self.trace(root,head+[ok],rooted=False).root_success(),{'started':0.9001,'ended':1.5})
            self.assertFalse(self.trace(root,head+[failed],rooted=False).root_succeeded())
            # A child exiting 0 is not the installer finishing.
            self.assertFalse(self.trace(root,head+[CREATE,STARTED,
                '1.2:0030:0034:trace:process:NtTerminateProcess handle 0xffffffffffffffff, exit_code 0, process_exiting 1.\n'],rooted=False).root_succeeded())
            unbound=self.trace(root,head+[ok],rooted=False);unbound.binding={'status':'unavailable'}
            self.assertFalse(unbound.root_succeeded())
    def test_missing_resource_alias_or_outside_prefix_is_not_the_profiled_app(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp).resolve();exe=app_tree(root,resources=False)
            self.assertEqual(self.found(root,[CREATE,STARTED]),{})
            for name in PROFILE['resources']:(exe.parent/name).write_bytes(b'r')
            (exe.parent/'app.pak').unlink();(exe.parent/'app.pak').symlink_to(exe.parent/'resources/app.asar')
            self.assertEqual(self.found(root,[CREATE,STARTED]),{})
            self.assertEqual(self.found(root,[create(path='Z:\\\\tmp\\\\Vendor\\\\App.exe'),STARTED]),{})

class MemberMatchTests(unittest.TestCase):
    """Which owned processes are the app: a fake /proc, no real signals."""
    KEY=(0,33,256)
    IMAGES={KEY:{'profile':'fixture_app','image':'Program Files/Vendor/App.exe','executable':'App.exe',
        'suffix':b'/drive_c/Program Files/Vendor/App.exe'}}
    APP_MAP=b'/home/u/.local/share/x/compatdata/pfx/drive_c/Program Files/Vendor/App.exe'
    def scope(self,proc,processes):
        for pid,(maps,program) in processes.items():
            (proc/str(pid)).mkdir();(proc/str(pid)/'maps').write_bytes(maps);(proc/str(pid)/'cmdline').write_bytes(program+b'\0--type=renderer\0')
        class Scope:
            proc_root=proc
            def members(self):return [{'pid':pid,'start_ticks':pid*10,'state':'S'} for pid in processes]
            def identity(self,pid):return {'pid':pid,'start_ticks':pid*10}
            def group_of(self,pid):return '/owned'
            def contains(self,group):return group=='/owned'
        return Scope()
    def maps(self,key,path,perms=b'r--p'):
        return b'140000000-140001000 '+perms+b' 00000000 %02x:%02x %d                 '%key+path+b'\n'
    def test_only_the_app_running_its_own_file_is_signalled(self):
        with tempfile.TemporaryDirectory() as tmp:
            proc=pathlib.Path(tmp)
            processes={
                101:(self.maps(self.KEY,self.APP_MAP),b'C:\\Program Files\\Vendor\\App.exe'),
                # Btrfs: same maps device and inode, but another subvolume's file.
                102:(self.maps(self.KEY,b'/usr/lib/libother.so'),b'C:\\Program Files\\Vendor\\App.exe'),
                # The app's file mapped as data by another program (icons, menus).
                103:(self.maps(self.KEY,self.APP_MAP),b'C:\\windows\\system32\\winemenubuilder.exe'),
                104:(self.maps(self.KEY,self.APP_MAP+b' (deleted)'),b'c:\\program files\\vendor\\app.exe'),
                105:(self.maps((0,33,257),self.APP_MAP),b'C:\\Program Files\\Vendor\\App.exe'),
                106:(b'',b'C:\\Program Files\\Vendor\\App.exe')}
            scope=self.scope(proc,processes);sent=[]
            with patch.object(session.os,'pidfd_open',side_effect=lambda pid:pid,create=True),patch.object(session.os,'close'),\
                    patch.object(session.signal,'pidfd_send_signal',side_effect=lambda fd,sig:sent.append((fd,sig)),create=True):
                signalled=session.installer_retire_profiled(scope,self.IMAGES)
            self.assertEqual(signalled,{(101,1010),(104,1040)})
            self.assertEqual(sorted(sent),[(101,signal.SIGTERM),(104,signal.SIGTERM)])

@unittest.skipUnless(sys.platform.startswith('linux'),'/proc maps')
class MappedIdentityTests(unittest.TestCase):
    def test_identity_is_how_maps_names_this_file_not_stat(self):
        # On Btrfs stat's device differs from maps' (stat 0:35, maps 00:21).
        import mmap
        with tempfile.TemporaryDirectory() as tmp:
            path=pathlib.Path(tmp).resolve()/'App With Space.exe';path.write_bytes(b'MZ'+bytes(5000))
            identity=session.mapped_file_identity(path)
            with open(path,'rb') as f:
                m=mmap.mmap(f.fileno(),4096,access=mmap.ACCESS_READ)
                try:lines=[l.split(None,5) for l in pathlib.Path('/proc/self/maps').read_bytes().splitlines()]
                finally:m.close()
            expected={(int(p[3].split(b':')[0],16),int(p[3].split(b':')[1],16),int(p[4])) for p in lines if len(p)==6 and p[5]==os.fsencode(str(path))}
            self.assertEqual({identity},expected)
            empty=pathlib.Path(tmp).resolve()/'empty';empty.touch()
            self.assertIsNone(session.mapped_file_identity(empty))

@unittest.skipUnless(sys.platform.startswith('linux'),'Linux supervisor subreaper and /proc maps')
class InstallerLoopTests(unittest.TestCase):
    def run_installer(self,profiles,*,resources=True,ignore_term=False,lifetime=6,installer_exit=0,reader=False):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp).resolve();(root/'operation.lock').touch();(root/'home').mkdir();artifact=root/'installer';artifact.write_bytes(b'fixture')
            exe=app_tree(root,resources=resources)
            # Programs named like the app and a menu helper (Python under another name).
            (root/'bin').mkdir();(root/'bin/App.exe').symlink_to(sys.executable);(root/'bin/winemenubuilder.exe').symlink_to(sys.executable)
            spec={'schema':2,'operation':'ab'*16,'environment':{'root':str(root),'runner':{'entry_point':'fixed-runner','proton':'fixed-proton','files':[]}},
                'installer':{'path':str(artifact),'sha256':session.hashlib.sha256(b'fixture').hexdigest()},'format':'pe_executable','report':str(root/'result.json')}
            if profiles is not None:spec['managed_first_launch']=profiles
            # As under the real runner: the trace shows the installer root start
            # the app and then end, while the runner itself stays until
            # everything it started has ended (pressure-vessel subreaper). Under
            # Wine the app's program name is its Windows path.
            head,tail=root_lines(artifact,code=installer_exit)
            mapper=('import mmap,signal,sys,time;f=open(sys.argv[1],"rb");m=mmap.mmap(f.fileno(),0,access=mmap.ACCESS_READ);'
                +('signal.signal(signal.SIGTERM,signal.SIG_IGN);' if ignore_term else '')
                +f'sys.stdout.write("running\\n");sys.stdout.flush();time.sleep({lifetime})')
            # Optionally another program maps the app's file as data the whole time.
            spawn=f'subprocess.Popen([{str(root/"bin/App.exe")!r},"-c",{mapper!r},{str(exe)!r}],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)'
            data=(f'r=subprocess.Popen([{str(root/"bin/winemenubuilder.exe")!r},"-c",{mapper!r},{str(exe)!r}],'
                'stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL);' if reader else '')
            installer=(f'import os,subprocess,sys,time;sys.stderr.write({"".join(head)+CREATE+STARTED+tail!r});sys.stderr.flush();'
                f'{data}p={spawn};p.wait();'+('r.wait();' if reader else '')+f'sys.exit({installer_exit})')
            real_popen=subprocess.Popen;children=[]
            def launch_owned(args,**kwargs):
                if not children:
                    (root/'compatdata/pfx').mkdir(parents=True,exist_ok=True);(root/'compatdata/pfx/system.reg').touch()
                program='print("prefix ready")' if not children else installer
                child=real_popen([sys.executable,'-c',program],stdout=subprocess.PIPE,stderr=subprocess.PIPE);children.append(child);return child
            scope=Scope();old=[signal.getsignal(s) for s in (signal.SIGTERM,signal.SIGINT)];started=time.monotonic()
            try:
                with patch.object(session,'CompanionCgroup',return_value=scope),patch.object(session.subprocess,'Popen',side_effect=launch_owned),patch.object(session,'environment',return_value={}):
                    ok=session.install(spec)
                return ok,json.loads((root/'result.json').read_text()),time.monotonic()-started
            finally:
                for sig,handler in zip((signal.SIGTERM,signal.SIGINT),old):signal.signal(sig,handler)
                for row in scope.members():
                    try:os.kill(row['pid'],signal.SIGKILL)
                    except ProcessLookupError:pass
                session.ctypes.CDLL(None).prctl(36,0,0,0,0)
    def test_installer_started_profiled_app_is_ended_after_the_installer_finishes(self):
        ok,v,elapsed=self.run_installer([PROFILE])
        self.assertTrue(ok);self.assertEqual(v['state'],'completed');self.assertEqual(v['raw_exit'],0);self.assertEqual(v['owned_live'],0)
        self.assertEqual(v['started_application']['result'],'closed_for_managed_first_launch')
        self.assertEqual(v['started_application']['profiles'],['fixture_app'])
        self.assertEqual(v['started_application']['images'],['Program Files/Vendor/App.exe'])
        self.assertEqual(v['started_application']['signalled_processes'],1)
        self.assertLess(elapsed,5)
    def test_a_program_mapping_the_app_as_data_is_left_alone(self):
        ok,v,elapsed=self.run_installer([PROFILE],reader=True)
        self.assertTrue(ok);self.assertEqual(v['state'],'completed')
        self.assertEqual(v['started_application']['signalled_processes'],1)
        self.assertGreater(elapsed,5) # the reader kept running to its own end
    def test_an_app_that_ignores_the_request_is_ended_outright_after_ten_seconds(self):
        ok,v,elapsed=self.run_installer([PROFILE],ignore_term=True,lifetime=40)
        self.assertTrue(ok);self.assertEqual(v['state'],'completed');self.assertEqual(v['owned_live'],0)
        self.assertEqual(v['started_application']['result'],'closed_for_managed_first_launch')
        self.assertGreater(elapsed,10);self.assertLess(elapsed,20)
    def test_a_failed_installer_closes_nothing_and_the_session_waits(self):
        ok,v,elapsed=self.run_installer([PROFILE],installer_exit=3)
        self.assertFalse(ok);self.assertEqual(v['state'],'failed');self.assertEqual(v['raw_exit'],3)
        self.assertIsNone(v['started_application']);self.assertGreater(elapsed,5)
    def test_without_a_matching_profile_the_session_still_waits_for_what_the_installer_started(self):
        for profiles,resources in [(None,True),([],True),([PROFILE],False)]:
            with self.subTest(profiles=profiles,resources=resources):
                ok,v,elapsed=self.run_installer(profiles,resources=resources)
                self.assertTrue(ok);self.assertEqual(v['state'],'completed');self.assertIsNone(v['started_application'])
                self.assertGreater(elapsed,5)

if __name__=='__main__':unittest.main()
