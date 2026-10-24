"""Real kernel fixture; no product/runtime/user installation is used.

Run in the admitted Linux test slot:
  LVB_RUN_MOUNT_BOUNDARY_TESTS=1 python3 -m unittest \
      test_library_location_mount_boundary -v

Requires util-linux unshare/mount, Python 3 and unprivileged user+mount
namespaces. Explicit Linux opt-in fails if these capabilities are unavailable;
default/non-Linux skips are not evidence for this boundary.
The post-bind rename test deliberately requires lasting pathname custody:
passing only a pre-exec fstat check is insufficient.
"""
import json
import ctypes
import os
import pathlib
import select
import shutil
import subprocess
import sys
import tempfile
import unittest


def _child(root, original, mode):
    root = pathlib.Path(root)
    host = root / 'source-parent/host-home'
    # The selected canonical host pathname is also the container pathname,
    # exactly as for PV and Wine's Z: mapping; there is no alternate alias that
    # accidentally leaves the original path writable in the test namespace.
    visible = host
    selected = visible / 'library'
    private_root = mode.startswith('private-root')
    def frame(value):
        print(json.dumps(value), flush=True)
    def pause():
        if sys.stdin.readline() != 'continue\n':
            raise RuntimeError('fixture coordinator disappeared')
    def mounted():
        if private_root:
            # Mechanism control only. Every destination ancestor is private,
            # including the process root: no host dentry above it is looked up
            # again after chroot. This is not a replacement for actual HOME.
            libc = ctypes.CDLL(None, use_errno=True)
            def mount(source, target, filesystem, flags, data):
                values = [None if value is None else os.fsencode(value)
                          for value in (source, target, filesystem)]
                if libc.mount(*values, ctypes.c_ulong(flags), data) != 0:
                    raise OSError(ctypes.get_errno(), 'fixture mount')
            stage = root / 'private-root'
            stage.mkdir()
            mount('tmpfs', stage, 'tmpfs', 0, b'mode=0700')
            destination = stage / selected.relative_to('/')
            destination.mkdir(parents=True)
            (stage / 'proc').mkdir()
            mount('/proc', stage / 'proc', None, 4096, None)
            # A direct syscall avoids util-linux's pathname canonicalization.
            # The open descriptor is retained through completed-mount checks.
            mount('/proc/self/fd/' + str(original), destination, None, 4096, None)  # MS_BIND
            mount(None, destination, None, 4096 | 32 | 1, None)  # BIND | REMOUNT | RDONLY
            for persistent in (root / 'product-home', root / 'product-setup'):
                target = stage / persistent.relative_to('/')
                target.mkdir(parents=True)
                mount(persistent, target, None, 4096, None)
            os.chroot(stage)
            os.chdir('/')
            return
        subprocess.run(['mount', '--bind', str(host), str(visible)], check=True)
        subprocess.run(['mount', '--bind', str(host / 'library'), str(selected)], check=True)
        if mode == 'readonly':
            subprocess.run(['mount', '-o', 'remount,bind,ro', str(selected)], check=True)
    def verify():
        expected = os.fstat(original)
        with_fd = os.open(selected, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
        try:
            actual = os.fstat(with_fd)
            if (expected.st_dev, expected.st_ino) != (actual.st_dev, actual.st_ino):
                raise RuntimeError('selected directory replaced')
            # Mount root identity is a separate requirement from matching a
            # directory exposed incidentally through the broad HOME mount.
            mount_id = next(line.split(':', 1)[1].strip() for line in
                pathlib.Path('/proc/self/fdinfo/' + str(with_fd)).read_text().splitlines()
                if line.startswith('mnt_id:'))
            rows = [line.split() for line in pathlib.Path('/proc/self/mountinfo').read_text().splitlines()
                    if line.split()[0] == mount_id]
            if len(rows) != 1 or rows[0][4] != str(selected):
                raise RuntimeError('selected directory is not the exact mount root')
            if (mode == 'readonly' or private_root) and not os.fstatvfs(with_fd).f_flag & os.ST_RDONLY:
                raise RuntimeError('selected mount is writable')
        finally:
            os.close(with_fd)
    try:
        frame({'stage': 'accepted'})
        pause()
        mounted()
        if mode == 'before-bind':
            try:
                verify()
            except RuntimeError as error:
                frame({'refused': str(error), 'vendor_started': False})
                return
            (root / 'vendor-started').write_text('unexpected')
            frame({'refused': None, 'vendor_started': True})
            return
        verify()
        frame({'stage': 'bound-and-verified'})
        pause()
        if mode == 'after-bind':
            expected = os.fstat(original)
            actual = selected.stat()
            frame({'same_directory': (actual.st_dev, actual.st_ino) == (expected.st_dev, expected.st_ino),
                   'content': (selected / 'sample').read_text()})
            return
        verify()
        # Model the two-phase guard release: no vendor child inherits a host
        # directory FD that could permit writes via openat or /proc/*/fd.
        os.close(original)
        original = None
        # Both the original canonical path in the container and its Wine-drive
        # alias must hit the same enforced read-only mount.
        alias = root / 'l-drive'
        alias.symlink_to(selected, target_is_directory=True)
        results = []
        for path in (selected, alias):
            try:
                (path / 'write-attempt').write_text('must not be written')
            except OSError as error:
                results.append(error.errno)
            else:
                results.append(None)
        if private_root:
            # Existing managed HOME and setup mounts must persist new writes.
            # Also expose the incompatible ambient host-HOME contract: this
            # write goes into private scaffolding, not the user's real HOME.
            (root / 'product-home/new-home-file').write_text('persistent home write')
            (root / 'product-setup/new-setup-file').write_text('persistent setup write')
            (host / 'new-ordinary-home-file').write_text('must not silently disappear')
        frame({'write_errors': results, 'content': (selected / 'sample').read_text()})
    finally:
        if original is not None:
            os.close(original)


@unittest.skipUnless(sys.platform == 'linux' and os.environ.get('LVB_RUN_MOUNT_BOUNDARY_TESTS') == '1',
                     'real mount-boundary fixture not executed; requires explicit admitted Linux slot')
class LibraryLocationMountBoundaryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if not shutil.which('unshare') or not shutil.which('mount'):
            raise RuntimeError('library_mount_test_capability_missing: unshare/mount unavailable; no mount-boundary proof')
        probe = subprocess.run(['unshare', '--user', '--map-root-user', '--mount',
                                '--propagation', 'private', 'true'], capture_output=True, text=True, timeout=5)
        if probe.returncode:
            raise RuntimeError('library_mount_test_capability_missing: user/mount namespace unavailable; no mount-boundary proof: '
                               + probe.stderr.strip())

    def run_boundary(self, mode):
        directory = tempfile.TemporaryDirectory(prefix='lvb-mount-fixture-')
        self.addCleanup(directory.cleanup)
        root = pathlib.Path(directory.name)
        original = root / 'source-parent/host-home/library'
        if mode == 'private-root-unmount':
            (root / 'source-parent').mkdir()
            (root / 'source-volume').mkdir()
            (root / 'alternate-volume').mkdir()
            subprocess.run(['mount', '--bind', str(root / 'source-volume'),
                            str(root / 'source-parent')], check=True)
            self.addCleanup(lambda: subprocess.run(['umount', '-l', str(root / 'source-parent')],
                                                    stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL))
        original.mkdir(parents=True)
        (original / 'sample').write_text('original fixture')
        (root / 'product-home').mkdir()
        (root / 'product-setup').mkdir()
        descriptor = os.open(original, os.O_RDONLY | os.O_DIRECTORY)
        try:
            namespaces = ['--mount'] if mode == 'private-root-unmount' else ['--user', '--map-root-user', '--mount']
            child = subprocess.Popen(['unshare', *namespaces,
                '--propagation', 'private', sys.executable, '-I', __file__, '--mount-child',
                str(root), str(descriptor), mode], pass_fds=(descriptor,), stdin=subprocess.PIPE,
                stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        finally:
            os.close(descriptor)
        def cleanup():
            if child.poll() is None:
                child.kill()
            child.communicate(timeout=5)
        self.addCleanup(cleanup)
        def read():
            self.assertTrue(select.select([child.stdout], [], [], 5)[0], 'fixture child did not answer')
            line = child.stdout.readline()
            self.assertTrue(line, 'fixture child ended without result')
            return json.loads(line)
        def proceed():
            child.stdin.write('continue\n')
            child.stdin.flush()
        substitutions = 0
        def substitute():
            nonlocal substitutions
            substitutions += 1
            original.rename(original.parent / ('retained-original-' + str(substitutions)))
            original.mkdir()
            (original / 'sample').write_text('substituted fixture')
        self.assertEqual(read(), {'stage': 'accepted'})
        if mode == 'before-bind' or mode.startswith('private-root'):
            substitute()
        proceed()
        if mode != 'before-bind':
            self.assertEqual(read(), {'stage': 'bound-and-verified'})
            if mode == 'after-bind':
                substitute()
            if mode == 'private-root':
                # Move the selected directory's parent AND its ancestor from
                # the outside namespace, then substitute the entire old path.
                original.parent.rename(root / 'source-parent/renamed-home')
                (root / 'source-parent').rename(root / 'renamed-source-parent')
                original.mkdir(parents=True)
                (original / 'sample').write_text('substituted parent and ancestor')
            if mode == 'private-root-unmount':
                # Detach the source volume in its owning namespace, revealing
                # an empty mountpoint; substitute a different directory there.
                subprocess.run(['umount', '-l', str(root / 'source-parent')], check=True)
                subprocess.run(['mount', '--bind', str(root / 'alternate-volume'),
                                str(root / 'source-parent')], check=True)
                original.mkdir(parents=True)
                (original / 'sample').write_text('substituted after source unmount')
            proceed()
        result = read()
        self.assertEqual(child.wait(timeout=5), 0)
        self.assertFalse((root / 'vendor-started').exists())
        if mode.startswith('private-root'):
            result['managed_home_persisted'] = (root / 'product-home/new-home-file').read_text() == 'persistent home write'
            result['setup_persisted'] = (root / 'product-setup/new-setup-file').read_text() == 'persistent setup write'
            result['ordinary_host_home_persisted'] = (original.parent / 'new-ordinary-home-file').exists()
        return result

    def test_substitution_after_fd_acceptance_refuses_before_vendor(self):
        self.assertEqual(self.run_boundary('before-bind'),
                         {'refused': 'selected directory replaced', 'vendor_started': False})

    def test_completed_bind_keeps_original_path_after_host_rename(self):
        self.assertEqual(self.run_boundary('after-bind'),
                         {'same_directory': True, 'content': 'original fixture'},
                         'PV-style bind under shared HOME does not provide lasting original-path custody')

    def test_readonly_mount_covers_canonical_path_and_drive_alias(self):
        import errno
        self.assertEqual(self.run_boundary('readonly'),
                         {'write_errors': [errno.EROFS, errno.EROFS], 'content': 'original fixture'})

    def test_private_root_fd_bind_survives_source_parent_and_ancestor_replacement(self):
        import errno
        self.assertEqual(self.run_boundary('private-root'),
                         {'write_errors': [errno.EROFS, errno.EROFS], 'content': 'original fixture',
                          'managed_home_persisted': True, 'setup_persisted': True,
                          'ordinary_host_home_persisted': False})

    def test_negative_private_root_changes_ambient_host_home_writes(self):
        # This negative control records the behavior requiring authorization;
        # it does not declare broad host-HOME access a product requirement.
        self.assertFalse(self.run_boundary('private-root')['ordinary_host_home_persisted'])

    def test_proposed_preservation_of_ambient_host_home_is_not_satisfied(self):
        # Opt-in design gate: must fail for this mechanism while the proposal
        # claims to preserve ambient host-HOME writes. Never a positive claim.
        self.assertTrue(self.run_boundary('private-root')['ordinary_host_home_persisted'],
                        'private HOME topology changes ambient host-HOME writes; requires an explicit product boundary decision')

    def test_private_root_retains_selected_volume_after_outer_source_unmount(self):
        import errno
        # Only this outer disposable namespace mounts the source volume. Its
        # child gets another mount namespace, permitting a real cross-namespace
        # source unmount without touching any host mount or product directory.
        completed = subprocess.run(['unshare', '--user', '--map-root-user', '--mount',
            '--propagation', 'private', sys.executable, '-I', __file__, '--outer-unmount'],
            capture_output=True, text=True, timeout=20)
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertEqual(json.loads(completed.stdout),
                         {'write_errors': [errno.EROFS, errno.EROFS], 'content': 'original fixture',
                          'managed_home_persisted': True, 'setup_persisted': True,
                          'ordinary_host_home_persisted': False})


if __name__ == '__main__':
    if len(sys.argv) == 5 and sys.argv[1] == '--mount-child':
        _child(sys.argv[2], int(sys.argv[3]), sys.argv[4])
    elif sys.argv[1:] == ['--outer-unmount']:
        case = LibraryLocationMountBoundaryTests()
        try:
            print(json.dumps(case.run_boundary('private-root-unmount')), flush=True)
        finally:
            case.doCleanups()
    else:
        unittest.main()
