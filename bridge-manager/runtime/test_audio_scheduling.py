"""Control-plane requests, restart and unavailable-capability behavior."""
import hashlib
import json
import os
import pathlib
import tempfile
import unittest
from unittest.mock import patch
import session


class AudioSchedulingTests(unittest.TestCase):
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
        self.assertEqual(control.value()['records'],[{'outcome':'unavailable','reason':'scheduler_artifact_unavailable'}])

    def test_reply_failure_remains_unavailable_and_observations_are_bounded(self):
        with patch.dict(os.environ,{},clear=True):
            control=session.AudioScheduling({'session':'42'*16,'directory':'/unused'})
        for _ in range(70):control.started();control.poll({(1,2)})
        self.assertEqual(control.value()['requests'],70)
        self.assertEqual(len(control.value()['records']),64)
        self.assertEqual(control.value()['discarded'],6)


if __name__=='__main__':unittest.main()
