"""The existing launch environment owner, with exact retained profile policies."""
import copy
import json
import os
from pathlib import Path
import unittest
from unittest.mock import patch
import session

class Scope(unittest.TestCase):
    def test_candidate17_is_process_local_and_does_not_rewrite_other_postures(self):
        root=Path(__file__).resolve().parents[2]
        paths=['compatibility/uio2/arturia-pigments.json','compatibility/if2/arturia-pigments.json',
               'compatibility/ap18/revision-11/arturia-pigments.json','compatibility/arturia-pure-lofi.json',
               'compatibility/arturia-efx-fragments.json']
        profiles=[json.loads((root/p).read_text()) for p in paths]
        ambient=dict(os.environ)
        reg={'environment':{'root':'/private/reference'},'compatibility':{'disable_windows_accessibility':False}}
        with patch.object(session.subprocess,'check_output',return_value='DISPLAY=:99\n'):
            before=[session.environment({**reg,'compatibility':{'disable_windows_accessibility':p['capabilities']['accessibility']=='disabled_for_vendor_process'}}) for p in profiles[1:]]
            candidate=session.environment({**reg,'compatibility':{'disable_windows_accessibility':True}})
            self.assertEqual(candidate['WINEDLLOVERRIDES'],'uiautomationcore=')
            self.assertNotIn('WINEDLLOVERRIDES',session.environment(reg)) # keeper/default child
            after=[session.environment({**reg,'compatibility':{'disable_windows_accessibility':p['capabilities']['accessibility']=='disabled_for_vendor_process'}}) for p in profiles[1:]]
        self.assertEqual(before,after);self.assertEqual(dict(os.environ),ambient)
        self.assertNotIn('WINEDLLOVERRIDES',before[0]);self.assertNotIn('WINEDLLOVERRIDES',before[1])
        # ASC owns its independent pre-existing posture, unchanged by this profile.
        self.assertEqual(session.vendor_compatibility('normal'),{'disable_windows_accessibility':True})
        prior=copy.deepcopy(profiles[0]);prior['revision']=profiles[1]['revision'];prior['evidence']=profiles[1]['evidence']
        prior['capabilities']['accessibility']='windows_default';prior['limitations'].remove('windows_accessibility_unavailable')
        self.assertEqual(prior,profiles[1])
if __name__=='__main__':unittest.main()
