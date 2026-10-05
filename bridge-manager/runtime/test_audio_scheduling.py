"""Control-plane requests, restart and unavailable-capability behavior."""
import hashlib
import json
import os
import pathlib
import subprocess
import struct
import tempfile
import unittest
from unittest.mock import Mock, patch
import session


class AudioSchedulingTests(unittest.TestCase):
    def test_native_preparation_negotiates_once_and_uses_authenticated_peer(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp);helper=root/'manager';journal=root/'requests'
            helper.write_text('#!/usr/bin/env python3\nimport sys,json,pathlib\n'
                f'with pathlib.Path({str(journal)!r}).open("a") as f:f.write(sys.stdin.read()+"\\n")\n'
                'print(json.dumps({"outcome":"effective"}))\n')
            helper.chmod(0o700)
            artifact={'path':str(helper),'sha256':hashlib.sha256(helper.read_bytes()).hexdigest()}
            spec={'session':'42'*16,'directory':str(root),
                'graphical_session':{'peer_pid':123,'peer_start_ticks':456}}
            with patch.dict(os.environ,{'LVB_AUDIO_SCHEDULER':json.dumps(artifact)}):
                control=session.AudioScheduling(spec)
            header=b'LVNS'+struct.pack('<I',1)+bytes.fromhex(spec['session'])
            self.assertEqual((root/'native-scheduling.supported').read_bytes(),header)
            control.poll({(999,1)});self.assertFalse(journal.exists())
            # The Rust helper validates content. Python cannot choose a host
            # process from these untrusted namespace IDs.
            (root/'native-scheduling.request').write_bytes(header+struct.pack('<IIQ',20,21,22))
            control.poll({(999,1)});control.poll({(888,2)})
            requests=[json.loads(line) for line in journal.read_text().splitlines()]
            self.assertEqual(requests,[{'schema':2,'session':'42'*16,'status':str(root/'ap12.status'),
                'owned':[],'native_peer':[123,456]}])
            self.assertEqual((root/'native-scheduling.reply').read_bytes(),header+struct.pack('<I',1))
            self.assertEqual(control.value()['requests'],1)
            self.assertEqual(control.value()['records'][0]['role'],'native_worker')

    def test_every_render_restart_gets_exact_owned_request_but_idle_polls_do_not(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp);helper=root/'manager';journal=root/'requests'
            helper.write_text('#!/usr/bin/env python3\nimport sys,json,pathlib\n'
                'assert sys.argv[1:]==["owned-audio-scheduling"]\n'
                f'with pathlib.Path({str(journal)!r}).open("a") as f:f.write(sys.stdin.read()+"\\n")\n'
                'print(json.dumps({"outcome":"unavailable","reason":"fixture_denied"}))\n')
            helper.chmod(0o700)
            artifact={'path':str(helper),'sha256':hashlib.sha256(helper.read_bytes()).hexdigest()}
            with patch.dict(os.environ,{'LVB_AUDIO_SCHEDULER':json.dumps(artifact)}):
                control=session.AudioScheduling({'session':'42'*16,'directory':str(root)})
            control.poll({(123,456)})
            self.assertFalse(journal.exists())
            control.started();control.poll({(123,456)})
            control.poll({(999,1)})
            control.started();control.poll({(789,1011)})
            rows=[json.loads(line) for line in journal.read_text().splitlines()]
            self.assertEqual([r['owned'] for r in rows],[[[123,456]],[[789,1011]]])
            self.assertTrue(all(r['session']=='42'*16 and r['status']==str(root/'ap12.status') for r in rows))
            self.assertEqual(control.value()['requests'],2)
            self.assertEqual([r['reason'] for r in control.value()['records']],['fixture_denied']*2)

    def test_missing_or_changed_helper_is_unavailable_without_failing_processing(self):
        with patch.dict(os.environ,{},clear=True):
            control=session.AudioScheduling({'session':'42'*16,'directory':'/unused'})
        control.started();control.poll({(1,2)})
        self.assertEqual(control.value()['records'],[{'outcome':'unavailable','reason':'scheduler_artifact_unavailable','attempt':1}])
        # A missing helper is final: no retry chain is armed.
        self.assertIsNone(control.retry_at)

    def test_reply_failure_remains_unavailable_and_observations_are_bounded(self):
        with patch.dict(os.environ,{},clear=True):
            control=session.AudioScheduling({'session':'42'*16,'directory':'/unused'})
        for _ in range(70):control.started();control.poll({(1,2)})
        self.assertEqual(control.value()['requests'],70)
        self.assertEqual(len(control.value()['records']),64)
        self.assertEqual(control.value()['discarded'],6)

    def test_helper_exit_racing_timeout_does_not_fail_the_audio_owner(self):
        child=Mock();child.poll.return_value=None;child.pid=123
        child.communicate.side_effect=[subprocess.TimeoutExpired('fixture',2),(b'',None)]
        with patch.dict(os.environ,{'LVB_AUDIO_SCHEDULER':json.dumps({'path':'/fixture','sha256':'00'*32})}),patch.object(session,'verify'):
            control=session.AudioScheduling({'session':'42'*16,'directory':'/unused'})
        with patch.object(session.subprocess,'Popen',return_value=child),patch.object(session.os,'killpg',side_effect=ProcessLookupError):
            control.started();control.poll({(1,2)})
        self.assertEqual(child.communicate.call_count,2)
        self.assertEqual(control.value()['records'],[{'outcome':'unavailable','reason':'scheduling_request_failed','attempt':1}])

    def test_transient_render_refusals_retry_on_bounded_backoff_until_effective(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp);helper=root/'manager';journal=root/'requests'
            # Scripted outcomes in arrival order: a refused bus call, a render
            # thread that is not yet unique, then an effective policy.
            helper.write_text('#!/usr/bin/env python3\nimport sys,json,pathlib\n'
                f'j=pathlib.Path({str(journal)!r});n=len(j.read_text().splitlines()) if j.exists() else 0\n'
                f'with j.open("a") as f:f.write(sys.stdin.read()+"\\n")\n'
                'replies=[{"outcome":"unavailable","reason":"scheduling_capability_unavailable"},'
                '{"outcome":"unavailable","reason":"scheduling_unique_owned_render_thread"},'
                '{"outcome":"effective"}]\n'
                'print(json.dumps(replies[min(n,2)]))\n')
            helper.chmod(0o700)
            artifact={'path':str(helper),'sha256':hashlib.sha256(helper.read_bytes()).hexdigest()}
            now=[1000.0]
            with patch.dict(os.environ,{'LVB_AUDIO_SCHEDULER':json.dumps(artifact)}):
                control=session.AudioScheduling({'session':'42'*16,'directory':str(root)},clock=lambda:now[0])
            control.started();control.poll({(123,456)})
            # Not due yet: idle polls do not request.
            now[0]+=0.4;control.poll({(123,456)});control.poll({(123,456)})
            self.assertEqual(control.value()['requests'],1)
            now[0]+=0.2;control.poll({(123,456)})
            self.assertEqual(control.value()['requests'],2)
            # The second delay is longer than the first.
            now[0]+=0.6;control.poll({(123,456)});self.assertEqual(control.value()['requests'],2)
            now[0]+=0.5;control.poll({(123,456)})
            self.assertEqual(control.value()['requests'],3)
            # Effective ends the chain.
            now[0]+=100;control.poll({(123,456)});control.poll({(123,456)})
            self.assertEqual(control.value()['requests'],3)
            self.assertEqual(control.value()['retries'],2)
            rows=control.value()['records']
            self.assertEqual([(r['outcome'],r['attempt']) for r in rows],
                [('unavailable',1),('unavailable',2),('effective',3)])
            self.assertEqual(len(journal.read_text().splitlines()),3)

    def test_retry_chain_is_bounded_and_a_render_restart_begins_a_new_chain(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp);helper=root/'manager'
            helper.write_text('#!/usr/bin/env python3\nimport sys,json\nsys.stdin.read()\n'
                'print(json.dumps({"outcome":"unavailable","reason":"scheduling_unique_owned_render_thread"}))\n')
            helper.chmod(0o700)
            artifact={'path':str(helper),'sha256':hashlib.sha256(helper.read_bytes()).hexdigest()}
            now=[0.0]
            with patch.dict(os.environ,{'LVB_AUDIO_SCHEDULER':json.dumps(artifact)}):
                control=session.AudioScheduling({'session':'42'*16,'directory':str(root)},clock=lambda:now[0])
            control.started()
            for _ in range(20):
                control.poll({(1,2)});now[0]+=10
            delays=session.AudioScheduling.RETRY_DELAYS
            self.assertEqual(control.value()['requests'],1+len(delays))
            self.assertEqual(control.value()['retries'],len(delays))
            self.assertEqual([r['attempt'] for r in control.value()['records']],list(range(1,len(delays)+2)))
            # A new render thread supersedes the spent chain and any armed retry.
            control.started();control.poll({(1,2)})
            self.assertEqual(control.value()['records'][-1]['attempt'],1)
            self.assertIsNotNone(control.retry_at)
            control.started();self.assertIsNone(control.retry_at)
            control.poll({(1,2)});self.assertEqual(control.value()['records'][-1]['attempt'],1)

    def test_final_reasons_do_not_retry(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp);helper=root/'manager'
            helper.write_text('#!/usr/bin/env python3\nimport sys,json\nsys.stdin.read()\n'
                'print(json.dumps({"outcome":"unavailable","reason":"scheduling_existing_policy_preserved"}))\n')
            helper.chmod(0o700)
            artifact={'path':str(helper),'sha256':hashlib.sha256(helper.read_bytes()).hexdigest()}
            with patch.dict(os.environ,{'LVB_AUDIO_SCHEDULER':json.dumps(artifact)}):
                control=session.AudioScheduling({'session':'42'*16,'directory':str(root)},clock=lambda:0.0)
            control.started();control.poll({(1,2)})
            self.assertIsNone(control.retry_at)
            self.assertEqual(control.value()['requests'],1)


if __name__=='__main__':unittest.main()
